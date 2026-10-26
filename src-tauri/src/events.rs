use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::{
    error::{AppError, AppResult},
    secrets::SecretStore,
    security::redact_text,
};

pub const EVENT_NAME: &str = "cinderdock-event";

#[derive(Debug, Clone)]
pub struct EventBus {
    app: AppHandle,
    secrets: SecretStore,
}

impl EventBus {
    pub fn new(app: AppHandle, secrets: SecretStore) -> Self {
        Self { app, secrets }
    }

    pub fn emit<T: Serialize>(&self, payload: T) -> AppResult<()> {
        let mut value = serde_json::to_value(payload)?;
        let secrets = self.secrets.all();
        redact_value(&mut value, &secrets);
        self.app
            .emit(EVENT_NAME, value)
            .map_err(|e| AppError::Internal(format!("cannot emit event: {e}")))?;
        Ok(())
    }

    pub fn redact(&self, text: &str) -> String {
        redact_text(&self.secrets.all(), text)
    }
}

fn redact_value(value: &mut Value, secrets: &[String]) {
    match value {
        Value::String(text) => *text = redact_text(secrets, text),
        Value::Array(items) => {
            for item in items {
                redact_value(item, secrets);
            }
        }
        Value::Object(map) => {
            for (_, item) in map.iter_mut() {
                redact_value(item, secrets);
            }
        }
        _ => {}
    }
}
