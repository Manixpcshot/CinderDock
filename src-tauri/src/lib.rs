pub mod agent;
pub mod app_state;
pub mod approvals;
pub mod commands;
pub mod config;
pub mod db;
pub mod diff;
pub mod error;
pub mod events;
pub mod models;
pub mod providers;
pub mod secrets;
pub mod security;
pub mod stream;
pub mod tools;

use std::fs;

use tracing_subscriber::EnvFilter;

use crate::{
    config::AppPaths,
    error::{AppError, AppResult},
    models::{Settings, SmokeReport},
};

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("cinderdock=info,tauri=warn")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = app_state::AppState::initialize()?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::get_settings,
            commands::list_provider_presets,
            commands::test_provider,
            commands::test_saved_provider,
            commands::save_provider,
            commands::delete_provider,
            commands::fetch_provider_models,
            commands::save_model,
            commands::save_models,
            commands::delete_model,
            commands::set_default_model,
            commands::update_preferences,
            commands::choose_project_folder,
            commands::set_project,
            commands::choose_attachments,
            commands::list_project_files,
            commands::list_conversations,
            commands::search_conversations,
            commands::create_conversation,
            commands::load_conversation,
            commands::delete_conversation,
            commands::set_conversation_model,
            commands::send_message,
            commands::retry_last,
            commands::stop_generation,
            commands::respond_approval,
            commands::revert_file_change,
            commands::clear_session_grants,
        ])
        .run(tauri::generate_context!())
        .expect("CinderDock window failed to start");
}

pub fn smoke_test() -> AppResult<SmokeReport> {
    let paths = AppPaths::discover()?;
    let settings = Settings::load_or_create(&paths)?;
    let conn = db::open_db(&paths.database_path)?;
    let migration_version = db::migration_version(&conn)?;
    let _client = providers::build_client()?;
    let raw_settings = fs::read_to_string(&paths.settings_path)?;
    if raw_settings.contains("\"apiKey\"")
        || raw_settings.contains("\"api_key\"")
        || raw_settings.contains("sk-")
    {
        return Err(AppError::Safety(
            "settings.json appears to contain a credential-shaped value.".into(),
        ));
    }
    let app = paths.app_info();
    let checks = vec![
        format!("settings schema v{} loaded", settings.schema_version),
        format!("SQLite migration v{migration_version} verified"),
        "HTTP client initialized".into(),
        "settings contains no API-key fields".into(),
        "secret backend configured".into(),
    ];
    Ok(SmokeReport {
        ok: true,
        version: env!("CARGO_PKG_VERSION").into(),
        config_dir: app.config_dir,
        database: paths.database_path.display().to_string(),
        migration_version,
        secret_backend: app.secret_backend,
        checks,
    })
}
