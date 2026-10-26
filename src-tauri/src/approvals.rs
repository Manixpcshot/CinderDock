use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use tokio::sync::oneshot;

use crate::{
    error::{AppError, AppResult},
    models::{ApprovalDecision, ApprovalRequest},
    security::new_id,
};

struct PendingApproval {
    conversation_id: String,
    session_key: String,
    sender: oneshot::Sender<ApprovalDecision>,
}

#[derive(Debug)]
struct Inner {
    pending: HashMap<String, PendingApproval>,
    grants: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct ApprovalManager {
    inner: Arc<Mutex<Inner>>,
}

pub enum ApprovalStart {
    Granted {
        request: ApprovalRequest,
        from_session: bool,
    },
    Pending {
        request: ApprovalRequest,
        receiver: oneshot::Receiver<ApprovalDecision>,
    },
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                pending: HashMap::new(),
                grants: HashSet::new(),
            })),
        }
    }

    pub fn begin(&self, mut request: ApprovalRequest) -> AppResult<ApprovalStart> {
        request.id = new_id("approval");
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Internal("approval lock poisoned".into()))?;
        if request.allow_session && inner.grants.contains(&request.session_key) {
            return Ok(ApprovalStart::Granted {
                request,
                from_session: true,
            });
        }
        let (sender, receiver) = oneshot::channel();
        inner.pending.insert(
            request.id.clone(),
            PendingApproval {
                conversation_id: request.conversation_id.clone(),
                session_key: request.session_key.clone(),
                sender,
            },
        );
        Ok(ApprovalStart::Pending { request, receiver })
    }

    pub fn respond(&self, id: &str, decision: ApprovalDecision) -> AppResult<bool> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Internal("approval lock poisoned".into()))?;
        let Some(pending) = inner.pending.remove(id) else {
            return Ok(false);
        };
        if decision == ApprovalDecision::Session && pending.session_key.starts_with("session:") {
            inner.grants.insert(pending.session_key.clone());
        }
        let _ = pending.sender.send(decision);
        Ok(true)
    }

    pub fn cancel(&self, id: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(pending) = inner.pending.remove(id) {
                let _ = pending.sender.send(ApprovalDecision::Deny);
            }
        }
    }

    pub fn cancel_conversation(&self, conversation_id: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            let ids: Vec<String> = inner
                .pending
                .iter()
                .filter(|(_, pending)| pending.conversation_id == conversation_id)
                .map(|(id, _)| id.clone())
                .collect();
            for id in ids {
                if let Some(pending) = inner.pending.remove(&id) {
                    let _ = pending.sender.send(ApprovalDecision::Deny);
                }
            }
        }
    }

    pub fn clear_grants(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.grants.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ApprovalRequest;

    fn sample_request(allow_session: bool) -> ApprovalRequest {
        ApprovalRequest {
            id: String::new(),
            conversation_id: "conv".into(),
            tool_name: "run_command".into(),
            title: "Run command".into(),
            details: "cargo test".into(),
            risk: "normal".into(),
            target: "cargo test".into(),
            diff: None,
            allow_session,
            session_key: "session:project:run_command".into(),
            created_at: "now".into(),
        }
    }

    #[tokio::test]
    async fn session_grant_skips_second_request() {
        let manager = ApprovalManager::new();
        let ApprovalStart::Pending { request, receiver } =
            manager.begin(sample_request(true)).unwrap()
        else {
            panic!("first request must be pending");
        };
        manager
            .respond(&request.id, ApprovalDecision::Session)
            .unwrap();
        assert_eq!(receiver.await.unwrap(), ApprovalDecision::Session);
        assert!(matches!(
            manager.begin(sample_request(true)).unwrap(),
            ApprovalStart::Granted {
                from_session: true,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn deny_stops_pending_operation() {
        let manager = ApprovalManager::new();
        let ApprovalStart::Pending { request, receiver } =
            manager.begin(sample_request(false)).unwrap()
        else {
            panic!("request must be pending");
        };
        manager
            .respond(&request.id, ApprovalDecision::Deny)
            .unwrap();
        assert_eq!(receiver.await.unwrap(), ApprovalDecision::Deny);
    }
}
