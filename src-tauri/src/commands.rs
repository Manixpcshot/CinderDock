use std::{collections::HashSet, path::PathBuf};

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use walkdir::WalkDir;

use crate::{
    agent,
    app_state::{provider_views, AppState},
    db,
    error::{AppError, AppResult},
    models::{
        AgentEvent, ApprovalDecision, AttachmentInput, Bootstrap, ConversationData,
        ConversationSummary, FileChangeView, ModelConfig, ModelInput, PreferencesPatch,
        ProjectInfo, ProviderConfig, ProviderInput, ProviderPreset, ProviderView, RemoteModel,
        RetryInput, RunStart, SendMessageInput, SettingsView, TestConnectionResult,
    },
    providers,
    security::{canonical_project_root, hash_text, is_sensitive_path, new_id, should_skip_dir},
};

type SharedState<'a> = State<'a, std::sync::Arc<AppState>>;

fn command_error(state: &AppState, error: AppError) -> String {
    state.redact(&error.to_string())
}

#[tauri::command]
pub async fn get_bootstrap(state: SharedState<'_>) -> Result<Bootstrap, String> {
    state
        .bootstrap()
        .await
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn get_settings(state: SharedState<'_>) -> Result<SettingsView, String> {
    Ok(state.settings_view().await)
}

#[tauri::command]
pub async fn list_provider_presets() -> Result<Vec<ProviderPreset>, String> {
    Ok(providers::preset_catalog())
}

#[tauri::command]
pub async fn test_provider(
    state: SharedState<'_>,
    input: ProviderInput,
) -> Result<TestConnectionResult, String> {
    let result = async {
        let (provider, supplied) =
            providers::normalize_provider(input.clone(), input.id.as_deref())?;
        let stored = input.id.as_deref().and_then(|id| state.secrets.get(id));
        let secret = providers::resolve_secret(&provider, supplied.as_deref(), stored)?;
        providers::test_connection(
            &state.client,
            &provider,
            secret.as_deref(),
            &state.secrets.all(),
        )
        .await
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn test_saved_provider(
    state: SharedState<'_>,
    provider_id: String,
) -> Result<TestConnectionResult, String> {
    let result = async {
        let settings = state.settings_snapshot().await;
        let provider = find_provider(&settings.providers, &provider_id)?;
        let secret = providers::resolve_secret(provider, None, state.secrets.get(&provider_id))?;
        providers::test_connection(
            &state.client,
            provider,
            secret.as_deref(),
            &state.secrets.all(),
        )
        .await
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn save_provider(
    state: SharedState<'_>,
    input: ProviderInput,
) -> Result<ProviderView, String> {
    let result = async {
        let (provider, supplied) =
            providers::normalize_provider(input.clone(), input.id.as_deref())?;
        let stored = state.secrets.get(&provider.id);
        let _ = providers::resolve_secret(&provider, supplied.as_deref(), stored.clone())?;
        if let Some(secret) = supplied {
            state.secrets.set(&provider.id, &secret)?;
        }
        if provider.auth.method == crate::models::AuthMethod::None {
            state.secrets.delete(&provider.id)?;
        }
        let mut settings = state.settings_snapshot().await;
        if let Some(slot) = settings.providers.iter_mut().find(|p| p.id == provider.id) {
            *slot = provider.clone();
        } else {
            settings.providers.push(provider.clone());
        }
        settings.sanitize();
        state.save_settings(settings.clone()).await?;
        state.approvals.clear_grants();
        provider_views(&settings, &state.secrets)
            .into_iter()
            .find(|view| view.id == provider.id)
            .ok_or_else(|| AppError::Internal("saved provider missing".into()))
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn delete_provider(
    state: SharedState<'_>,
    provider_id: String,
) -> Result<SettingsView, String> {
    let result = async {
        let mut settings = state.settings_snapshot().await;
        let removed_models: Vec<String> = settings
            .models
            .iter()
            .filter(|model| model.provider_id == provider_id)
            .map(|model| model.id.clone())
            .collect();
        settings
            .providers
            .retain(|provider| provider.id != provider_id);
        settings
            .models
            .retain(|model| model.provider_id != provider_id);
        if settings
            .default_model_id
            .as_deref()
            .map(|id| removed_models.iter().any(|model| model == id))
            .unwrap_or(false)
        {
            settings.default_model_id = None;
        }
        state.secrets.delete(&provider_id)?;
        state.save_settings(settings).await?;
        {
            let conn = state.db.lock().await;
            for model_id in removed_models {
                db::clear_model_references(&conn, &model_id)?;
            }
        }
        state.approvals.clear_grants();
        Ok(state.settings_view().await)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn fetch_provider_models(
    state: SharedState<'_>,
    provider_id: String,
) -> Result<Vec<RemoteModel>, String> {
    let result = async {
        let settings = state.settings_snapshot().await;
        let provider = find_provider(&settings.providers, &provider_id)?;
        let secret = providers::resolve_secret(provider, None, state.secrets.get(&provider_id))?;
        providers::fetch_models(
            &state.client,
            provider,
            secret.as_deref(),
            &state.secrets.all(),
        )
        .await
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn save_model(state: SharedState<'_>, input: ModelInput) -> Result<SettingsView, String> {
    save_models_impl(&state, vec![input])
        .await
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn save_models(
    state: SharedState<'_>,
    inputs: Vec<ModelInput>,
) -> Result<SettingsView, String> {
    save_models_impl(&state, inputs)
        .await
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn delete_model(
    state: SharedState<'_>,
    model_id: String,
) -> Result<SettingsView, String> {
    let result = async {
        let mut settings = state.settings_snapshot().await;
        settings.models.retain(|model| model.id != model_id);
        if settings.default_model_id.as_deref() == Some(model_id.as_str()) {
            settings.default_model_id = settings.models.first().map(|model| model.id.clone());
        }
        state.save_settings(settings).await?;
        let conn = state.db.lock().await;
        db::clear_model_references(&conn, &model_id)?;
        drop(conn);
        Ok(state.settings_view().await)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn set_default_model(
    state: SharedState<'_>,
    model_id: Option<String>,
) -> Result<SettingsView, String> {
    let result = async {
        let mut settings = state.settings_snapshot().await;
        if let Some(id) = &model_id {
            if !settings.models.iter().any(|model| model.id == *id) {
                return Err(AppError::Validation("مدل پیشفرض پیدا نشد.".into()));
            }
        }
        settings.default_model_id = model_id;
        state.save_settings(settings).await?;
        Ok(state.settings_view().await)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn update_preferences(
    state: SharedState<'_>,
    patch: PreferencesPatch,
) -> Result<SettingsView, String> {
    let result = async {
        if !(0.85..=1.35).contains(&patch.font_scale)
            || !(1..=16).contains(&patch.max_tool_iterations)
            || patch.max_retries > 5
        {
            return Err(AppError::Validation(
                "مقادیر تنظیمات خارج از محدودهٔ مجاز است.".into(),
            ));
        }
        let mut settings = state.settings_snapshot().await;
        if let Some(id) = &patch.default_model_id {
            if !settings.models.iter().any(|model| model.id == *id) {
                return Err(AppError::Validation("مدل پیشفرض پیدا نشد.".into()));
            }
        }
        settings.theme = patch.theme;
        settings.ui_language = patch.ui_language;
        settings.text_mode = patch.text_mode;
        settings.font_scale = patch.font_scale;
        settings.desktop_tools_enabled = patch.desktop_tools_enabled;
        settings.web_access_enabled = patch.web_access_enabled;
        settings.max_tool_iterations = patch.max_tool_iterations;
        settings.max_retries = patch.max_retries;
        settings.terminal_visible = patch.terminal_visible;
        settings.default_model_id = patch.default_model_id;
        state.save_settings(settings).await?;
        state.approvals.clear_grants();
        Ok(state.settings_view().await)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn choose_project_folder(
    state: SharedState<'_>,
    app: AppHandle,
) -> Result<Option<ProjectInfo>, String> {
    let start = state.settings_snapshot().await.project_path.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file();
        if let Some(path) = start {
            dialog = dialog.set_directory(PathBuf::from(path));
        }
        dialog.blocking_pick_folder()
    })
    .await
    .map_err(|e| command_error(&state, AppError::Internal(format!("dialog failed: {e}"))))?;
    let Some(file_path) = picked else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|e| command_error(&state, AppError::Internal(format!("invalid path: {e}"))))?;
    set_project_impl(&state, path)
        .await
        .map(Some)
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn set_project(state: SharedState<'_>, path: String) -> Result<ProjectInfo, String> {
    set_project_impl(&state, PathBuf::from(path))
        .await
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn choose_attachments(
    state: SharedState<'_>,
    app: AppHandle,
) -> Result<Vec<AttachmentInput>, String> {
    let result = async {
        let settings = state.settings_snapshot().await;
        let root = settings
            .project_path
            .as_deref()
            .map(|p| canonical_project_root(std::path::Path::new(p)))
            .transpose()?
            .ok_or_else(|| AppError::Validation("ابتدا پوشهٔ پروژه را انتخاب کنید.".into()))?;
        let start = root.clone();
        let picked = tauri::async_runtime::spawn_blocking(move || {
            app.dialog()
                .file()
                .set_directory(start)
                .blocking_pick_files()
        })
        .await
        .map_err(|e| AppError::Internal(format!("dialog failed: {e}")))?;
        let Some(paths) = picked else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for file_path in paths.into_iter().take(8) {
            let absolute = file_path
                .into_path()
                .map_err(|e| AppError::Internal(format!("invalid file path: {e}")))?;
            let canonical = std::fs::canonicalize(&absolute)?;
            if !canonical.starts_with(&root) {
                return Err(AppError::Safety(
                    "فایل انتخابشده بیرون از پوشهٔ پروژه است.".into(),
                ));
            }
            let relative = canonical.strip_prefix(&root).unwrap_or(&canonical);
            if is_sensitive_path(relative) {
                return Err(AppError::Safety(
                    "این فایل حساس قابل پیوست کردن نیست.".into(),
                ));
            }
            out.push(AttachmentInput {
                path: relative.to_string_lossy().into_owned(),
            });
        }
        Ok(out)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn list_project_files(state: SharedState<'_>) -> Result<Vec<String>, String> {
    let result = async {
        let settings = state.settings_snapshot().await;
        let Some(path) = settings.project_path else {
            return Ok(Vec::new());
        };
        let root = canonical_project_root(std::path::Path::new(&path))?;
        Ok(list_project_files_inner(&root))
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn list_conversations(
    state: SharedState<'_>,
) -> Result<Vec<ConversationSummary>, String> {
    let conn = state.db.lock().await;
    db::list_conversations(&conn, 100).map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn search_conversations(
    state: SharedState<'_>,
    query: String,
) -> Result<Vec<ConversationSummary>, String> {
    let conn = state.db.lock().await;
    db::search_conversations(&conn, &query).map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn create_conversation(
    state: SharedState<'_>,
    title: Option<String>,
    model_id: Option<String>,
) -> Result<ConversationSummary, String> {
    let result = async {
        let settings = state.settings_snapshot().await;
        let id = new_id("conv");
        let title = title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "گفتوگوی جدید".into());
        let model = model_id.or(settings.default_model_id);
        let conn = state.db.lock().await;
        db::create_conversation(
            &conn,
            &id,
            &title,
            settings.project_path.as_deref(),
            model.as_deref(),
        )
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn load_conversation(
    state: SharedState<'_>,
    conversation_id: String,
) -> Result<ConversationData, String> {
    let conn = state.db.lock().await;
    db::load_conversation(&conn, &conversation_id).map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn delete_conversation(
    state: SharedState<'_>,
    conversation_id: String,
) -> Result<Vec<ConversationSummary>, String> {
    let result = async {
        {
            let conn = state.db.lock().await;
            db::delete_conversation(&conn, &conversation_id)?;
        }
        let conn = state.db.lock().await;
        db::list_conversations(&conn, 100)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn set_conversation_model(
    state: SharedState<'_>,
    conversation_id: String,
    model_id: String,
) -> Result<ConversationSummary, String> {
    let result = async {
        let settings = state.settings_snapshot().await;
        if !settings.models.iter().any(|model| model.id == model_id) {
            return Err(AppError::Validation("مدل انتخابشده پیدا نشد.".into()));
        }
        let conn = state.db.lock().await;
        db::set_conversation_model(&conn, &conversation_id, &model_id)?;
        db::get_conversation(&conn, &conversation_id)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn send_message(
    state: SharedState<'_>,
    app: AppHandle,
    input: SendMessageInput,
) -> Result<RunStart, String> {
    agent::start_send(state.inner().clone(), app, input)
        .await
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn retry_last(
    state: SharedState<'_>,
    app: AppHandle,
    input: RetryInput,
) -> Result<RunStart, String> {
    agent::start_retry(state.inner().clone(), app, input)
        .await
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn stop_generation(state: SharedState<'_>, run_id: String) -> Result<bool, String> {
    if let Some(conversation_id) = state.stop_run(&run_id).await {
        state.approvals.cancel_conversation(&conversation_id);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn respond_approval(
    state: SharedState<'_>,
    request_id: String,
    decision: String,
) -> Result<bool, String> {
    let parsed = match decision.as_str() {
        "once" => ApprovalDecision::Once,
        "session" => ApprovalDecision::Session,
        "deny" => ApprovalDecision::Deny,
        _ => return Err("تصمیم مجوز نامعتبر است.".into()),
    };
    state
        .approvals
        .respond(&request_id, parsed)
        .map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn revert_file_change(
    state: SharedState<'_>,
    app: AppHandle,
    change_id: String,
    confirmed: bool,
) -> Result<FileChangeView, String> {
    let result = async {
        if !confirmed {
            return Err(AppError::Validation(
                "بازگردانی نیازمند تأیید صریح کاربر است.".into(),
            ));
        }
        let change = {
            let conn = state.db.lock().await;
            db::get_file_change(&conn, &change_id)?
        };
        let settings = state.settings_snapshot().await;
        let root = settings
            .project_path
            .as_deref()
            .map(|p| canonical_project_root(std::path::Path::new(p)))
            .transpose()?
            .ok_or_else(|| AppError::Validation("پوشه پروژه مشخص نیست.".into()))?;
        let resolved = crate::security::project_relative_path(&root, &change.path)?;
        let current = if resolved.exists() {
            crate::security::safe_read_text(&resolved, crate::security::MAX_FILE_BYTES)?
        } else {
            String::new()
        };
        if hash_text(&current) != change.after_hash {
            return Err(AppError::Safety(
                "فایل پس از اعمال تغییر، دستکاری شده است. برای جلوگیری از از دست رفتن دادهها بازگردانی انجام نشد.".into(),
            ));
        }
        if change.existed_before {
            crate::security::atomic_write(&resolved, &change.before)?;
        } else if resolved.exists() {
            std::fs::remove_file(&resolved)?;
        }
        db::set_file_change_status(
            &state.db.lock().await,
            &change_id,
            "reverted",
        )?;
        let mut updated = change;
        updated.status = "reverted".into();
        let bus = state.event_bus(app);
        bus.emit(AgentEvent::FileChange {
            change: updated.clone(),
        })?;
        Ok(updated)
    }
    .await;
    result.map_err(|e| command_error(&state, e))
}

#[tauri::command]
pub async fn clear_session_grants(state: SharedState<'_>) -> Result<bool, String> {
    state.approvals.clear_grants();
    Ok(true)
}

fn find_provider<'a>(
    providers: &'a [ProviderConfig],
    provider_id: &str,
) -> AppResult<&'a ProviderConfig> {
    providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| AppError::Validation("پروایدر پیدا نشد.".into()))
}

async fn save_models_impl(state: &AppState, inputs: Vec<ModelInput>) -> AppResult<SettingsView> {
    if inputs.is_empty() || inputs.len() > 100 {
        return Err(AppError::Validation(
            "تعداد مدلهای ارسالی باید بین ۱ و ۱۰۰ باشد.".into(),
        ));
    }
    let mut settings = state.settings_snapshot().await;
    let mut seen = HashSet::new();
    let mut selected_default: Option<String> = None;
    let mut added = Vec::new();
    for input in inputs {
        let set_default = input.set_default;
        let model = normalize_model(input, &settings)?;
        let pair = format!("{}::{}", model.provider_id, model.model_id);
        if !seen.insert(pair.clone()) {
            return Err(AppError::Validation(format!(
                "مدل «{}» دوباره وارد شده است.",
                model.model_id
            )));
        }
        if settings.models.iter().any(|existing| {
            existing.id != model.id
                && existing.provider_id == model.provider_id
                && existing.model_id == model.model_id
        }) {
            return Err(AppError::Validation(format!(
                "مدل «{}» قبلاً برای این پروایدر ثبت شده است.",
                model.model_id
            )));
        }
        if set_default {
            selected_default = Some(model.id.clone());
        }
        added.push(model);
    }
    for model in added {
        if let Some(slot) = settings.models.iter_mut().find(|item| item.id == model.id) {
            *slot = model;
        } else {
            settings.models.push(model);
        }
    }
    if let Some(id) = selected_default {
        settings.default_model_id = Some(id);
    } else if settings.default_model_id.is_none() {
        settings.default_model_id = settings.models.first().map(|model| model.id.clone());
    }
    settings.sanitize();
    state.save_settings(settings).await?;
    Ok(state.settings_view().await)
}

fn normalize_model(
    input: ModelInput,
    settings: &crate::models::Settings,
) -> AppResult<ModelConfig> {
    let model_id = input.model_id.trim();
    let display_name = input.display_name.trim();
    if model_id.is_empty() || display_name.is_empty() {
        return Err(AppError::Validation(
            "شناسه و نام نمایشی مدل لازم است.".into(),
        ));
    }
    if !settings
        .providers
        .iter()
        .any(|provider| provider.id == input.provider_id)
    {
        return Err(AppError::Validation("پروایدر مدل پیدا نشد.".into()));
    }
    if let Some(window) = input.context_window {
        if window > 10_000_000 {
            return Err(AppError::Validation(
                "پنجرهٔ زمینه بیش از حد بزرگ است.".into(),
            ));
        }
    }
    Ok(ModelConfig {
        id: input
            .id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| new_id("model")),
        provider_id: input.provider_id,
        model_id: model_id.into(),
        display_name: display_name.into(),
        supports_tools: input.supports_tools,
        supports_vision: input.supports_vision,
        context_window: input.context_window,
    })
}

async fn set_project_impl(state: &AppState, path: PathBuf) -> AppResult<ProjectInfo> {
    let root = canonical_project_root(&path)?;
    let display = root.display().to_string();
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| display.clone());
    let file_count = list_project_files_inner(&root).len();
    let mut settings = state.settings_snapshot().await;
    settings.project_path = Some(display.clone());
    settings
        .recent_projects
        .retain(|project| project != &display);
    settings.recent_projects.insert(0, display.clone());
    settings.recent_projects.truncate(8);
    state.save_settings(settings).await?;
    state.approvals.clear_grants();
    Ok(ProjectInfo {
        path: display,
        name,
        file_count,
    })
}

fn list_project_files_inner(root: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_entry(|entry| {
            entry.file_type().is_file() || !should_skip_dir(&entry.file_name().to_string_lossy())
        })
    {
        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if is_sensitive_path(relative) {
            continue;
        }
        files.push(relative.to_string_lossy().into_owned());
        if files.len() >= 300 {
            break;
        }
    }
    files
}
