use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use rusqlite::Connection;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::{
    approvals::ApprovalManager,
    config::AppPaths,
    db,
    error::{AppError, AppResult},
    events::EventBus,
    models::{AuthMethod, Bootstrap, ModelView, Preferences, ProviderView, Settings, SettingsView},
    providers,
    secrets::SecretStore,
    security::redact_text,
};

#[derive(Debug, Clone)]
pub struct RunHandle {
    pub run_id: String,
    pub conversation_id: String,
    pub stop: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub settings: Mutex<Settings>,
    pub db: Mutex<Connection>,
    pub secrets: SecretStore,
    pub approvals: ApprovalManager,
    pub runs: Mutex<HashMap<String, RunHandle>>,
    pub client: reqwest::Client,
}

impl AppState {
    pub fn initialize() -> AppResult<Arc<Self>> {
        let paths = AppPaths::discover()?;
        let settings = Settings::load_or_create(&paths)?;
        let conn = db::open_db(&paths.database_path)?;
        let secrets = SecretStore::new(paths.secrets_path.clone())?;
        Ok(Arc::new(Self {
            paths,
            settings: Mutex::new(settings),
            db: Mutex::new(conn),
            secrets,
            approvals: ApprovalManager::new(),
            runs: Mutex::new(HashMap::new()),
            client: providers::build_client()?,
        }))
    }

    #[cfg(test)]
    pub fn for_tests(root: &std::path::Path) -> AppResult<Arc<Self>> {
        let paths = AppPaths::for_temp(root)?;
        let settings = Settings::load_or_create(&paths)?;
        let conn = db::open_db(&paths.database_path)?;
        let secrets = SecretStore::new(paths.secrets_path.clone())?;
        Ok(Arc::new(Self {
            paths,
            settings: Mutex::new(settings),
            db: Mutex::new(conn),
            secrets,
            approvals: ApprovalManager::new(),
            runs: Mutex::new(HashMap::new()),
            client: providers::build_client()?,
        }))
    }

    pub fn event_bus(&self, app: AppHandle) -> EventBus {
        EventBus::new(app, self.secrets.clone())
    }

    pub fn redact(&self, text: &str) -> String {
        redact_text(&self.secrets.all(), text)
    }

    pub async fn settings_snapshot(&self) -> Settings {
        self.settings.lock().await.clone()
    }

    pub async fn save_settings(&self, settings: Settings) -> AppResult<()> {
        settings.save(&self.paths)?;
        *self.settings.lock().await = settings;
        Ok(())
    }

    pub async fn settings_view(&self) -> SettingsView {
        let settings = self.settings.lock().await;
        SettingsView {
            preferences: Preferences::from(&*settings),
            providers: provider_views(&settings, &self.secrets),
            models: model_views(&settings),
            project_path: settings.project_path.clone(),
            recent_projects: settings.recent_projects.clone(),
            updates: settings.updates.clone(),
            app: self.paths.app_info(),
        }
    }

    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        let conversations = {
            let conn = self.db.lock().await;
            db::list_conversations(&conn, 100)?
        };
        Ok(Bootstrap {
            settings: self.settings_view().await,
            conversations,
            presets: providers::preset_catalog(),
        })
    }

    pub async fn register_run(&self, handle: RunHandle) -> AppResult<()> {
        let mut runs = self.runs.lock().await;
        if runs.values().any(|run| {
            run.conversation_id == handle.conversation_id
                && !run.stop.load(std::sync::atomic::Ordering::SeqCst)
        }) {
            return Err(AppError::Validation(
                "یک پاسخ برای این گفتوگو در حال اجراست.".into(),
            ));
        }
        runs.insert(handle.run_id.clone(), handle);
        Ok(())
    }

    pub async fn stop_run(&self, run_id: &str) -> Option<String> {
        let runs = self.runs.lock().await;
        let run = runs.get(run_id)?;
        run.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        Some(run.conversation_id.clone())
    }

    pub async fn remove_run(&self, run_id: &str) {
        self.runs.lock().await.remove(run_id);
    }
}

pub fn provider_views(settings: &Settings, secrets: &SecretStore) -> Vec<ProviderView> {
    settings
        .providers
        .iter()
        .map(|provider| ProviderView {
            id: provider.id.clone(),
            display_name: provider.display_name.clone(),
            preset: provider.preset.clone(),
            kind: provider.kind,
            base_url: provider.base_url.clone(),
            auth_method: provider.auth.method,
            auth_header: provider.auth.header_name.clone(),
            has_secret: provider.auth.method != AuthMethod::None && secrets.has(&provider.id),
            extra_headers: provider.extra_headers.clone(),
            models_path: provider.models_path.clone(),
            chat_path: provider.chat_path.clone(),
            model_count: settings
                .models
                .iter()
                .filter(|model| model.provider_id == provider.id)
                .count(),
        })
        .collect()
}

pub fn model_views(settings: &Settings) -> Vec<ModelView> {
    settings
        .models
        .iter()
        .map(|model| ModelView {
            config: model.clone(),
            provider_name: settings
                .providers
                .iter()
                .find(|provider| provider.id == model.provider_id)
                .map(|provider| provider.display_name.clone())
                .unwrap_or_else(|| "پروایدر حذفشده".into()),
            is_default: settings.default_model_id.as_deref() == Some(model.id.as_str()),
        })
        .collect()
}
