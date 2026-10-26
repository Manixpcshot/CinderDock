use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use base64::Engine;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::{
    app_state::{AppState, RunHandle},
    db,
    error::{AppError, AppResult},
    events::EventBus,
    models::{
        AgentEvent, AttachmentInput, AttachmentView, MessageView, ModelConfig, ProviderConfig,
        RetryInput, RunStart, SendMessageInput, Settings, StreamOutcome, ToolCallView,
    },
    providers,
    security::{
        canonical_project_root, hash_text, new_id, now_rfc3339, project_relative_path,
        safe_read_text, truncate_chars, MAX_IMAGE_BYTES,
    },
    stream::StreamParser,
    tools::{definitions, ToolContext, ToolRunner},
};

const MAX_STREAM_BYTES: usize = 16 * 1024 * 1024;
const MAX_STREAM_CHARS: usize = 200_000;
const MAX_HISTORY_CHARS: usize = 120_000;

#[derive(Debug, Clone)]
pub struct AgentRun {
    pub state: Arc<AppState>,
    pub bus: EventBus,
    pub run_id: String,
    pub conversation_id: String,
    pub initial_assistant_message_id: String,
    pub provider: ProviderConfig,
    pub model: ModelConfig,
    pub settings: Settings,
    pub project_root: Option<PathBuf>,
    pub stop: Arc<AtomicBool>,
}

enum StreamAttempt {
    Success(StreamOutcome),
    Failure {
        error: AppError,
        had_delta: bool,
        retryable: bool,
    },
}

pub async fn start_send(
    state: Arc<AppState>,
    app: AppHandle,
    input: SendMessageInput,
) -> AppResult<RunStart> {
    if input.content.trim().is_empty() && input.attachments.is_empty() {
        return Err(AppError::Validation(
            "پیام یا پیوستی برای ارسال وجود ندارد.".into(),
        ));
    }
    let settings = state.settings_snapshot().await;
    let (provider, model) = resolve_model(&settings, input.model_id.as_deref())?;
    let project_root = project_root_from_settings(&settings)?;
    let attachments = prepare_attachments(project_root.as_deref(), &input.attachments, &model)?;
    let conversation_id = input
        .conversation_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| new_id("conv"));
    let user_message = MessageView {
        id: new_id("msg"),
        conversation_id: conversation_id.clone(),
        role: "user".into(),
        content: input.content.clone(),
        attachments,
        tool_calls: Vec::new(),
        tool_call_id: None,
        status: "complete".into(),
        created_at: now_rfc3339(),
        seq: 0,
    };
    {
        let conn = state.db.lock().await;
        match db::get_conversation(&conn, &conversation_id) {
            Ok(_) => {
                db::set_conversation_model(&conn, &conversation_id, &model.id)?;
                db::set_conversation_project(
                    &conn,
                    &conversation_id,
                    settings.project_path.as_deref(),
                )?;
            }
            Err(_) => {
                let title = conversation_title(&input.content, &user_message.attachments);
                db::create_conversation(
                    &conn,
                    &conversation_id,
                    &title,
                    settings.project_path.as_deref(),
                    Some(&model.id),
                )?;
            }
        }
        db::insert_message(&conn, &user_message)?;
    }
    spawn_run(
        state,
        app,
        settings,
        provider,
        model,
        conversation_id,
        project_root,
        Some(user_message),
    )
    .await
}

pub async fn start_retry(
    state: Arc<AppState>,
    app: AppHandle,
    input: RetryInput,
) -> AppResult<RunStart> {
    let settings = state.settings_snapshot().await;
    let conversation = {
        let conn = state.db.lock().await;
        db::get_conversation(&conn, &input.conversation_id)?
    };
    let selected = input
        .model_id
        .as_deref()
        .or(conversation.model_id.as_deref());
    let (provider, model) = resolve_model(&settings, selected)?;
    {
        let conn = state.db.lock().await;
        if !db::delete_last_assistant_turn(&conn, &input.conversation_id)? {
            return Err(AppError::Validation(
                "پیام کاربری برای تلاش دوباره پیدا نشد.".into(),
            ));
        }
        db::set_conversation_model(&conn, &input.conversation_id, &model.id)?;
    }
    let project_root = project_root_from_settings(&settings)?;
    spawn_run(
        state,
        app,
        settings,
        provider,
        model,
        input.conversation_id,
        project_root,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn spawn_run(
    state: Arc<AppState>,
    app: AppHandle,
    settings: Settings,
    provider: ProviderConfig,
    model: ModelConfig,
    conversation_id: String,
    project_root: Option<PathBuf>,
    user_message: Option<MessageView>,
) -> AppResult<RunStart> {
    let assistant_message_id = new_id("msg");
    let placeholder = MessageView {
        id: assistant_message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: "assistant".into(),
        content: String::new(),
        attachments: Vec::new(),
        tool_calls: Vec::new(),
        tool_call_id: None,
        status: "streaming".into(),
        created_at: now_rfc3339(),
        seq: 0,
    };
    {
        let conn = state.db.lock().await;
        db::insert_message(&conn, &placeholder)?;
    }
    let run_id = new_id("run");
    let stop = Arc::new(AtomicBool::new(false));
    let bus = state.event_bus(app);
    state
        .register_run(RunHandle {
            run_id: run_id.clone(),
            conversation_id: conversation_id.clone(),
            stop: stop.clone(),
        })
        .await?;
    let run = AgentRun {
        state: state.clone(),
        bus,
        run_id: run_id.clone(),
        conversation_id: conversation_id.clone(),
        initial_assistant_message_id: assistant_message_id.clone(),
        provider,
        model,
        settings,
        project_root,
        stop,
    };
    tokio::spawn(async move {
        let run_id = run.run_id.clone();
        let conversation_id = run.conversation_id.clone();
        let bus = run.bus.clone();
        let state = run.state.clone();
        if let Err(error) = run_agent(run).await {
            let message = bus.redact(&error.to_string());
            let _ = bus.emit(AgentEvent::Connection {
                provider_id: String::new(),
                state: "error".into(),
                message: message.clone(),
            });
            let _ = bus.emit(AgentEvent::Error {
                conversation_id: conversation_id.clone(),
                message: message.clone(),
                retryable: true,
            });
            let conn = state.db.lock().await;
            let _ = db::mark_streaming_messages_error(&conn, &conversation_id, &message);
        }
        state.remove_run(&run_id).await;
    });
    Ok(RunStart {
        run_id,
        conversation_id,
        user_message,
        assistant_message_id,
    })
}

fn resolve_model(
    settings: &Settings,
    requested: Option<&str>,
) -> AppResult<(ProviderConfig, ModelConfig)> {
    let model_id = requested
        .or(settings.default_model_id.as_deref())
        .ok_or_else(|| {
            AppError::Validation("هیچ مدلی انتخاب نشده است. ابتدا پروایدر و مدل اضافه کنید.".into())
        })?;
    let model = settings
        .models
        .iter()
        .find(|model| model.id == model_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("مدل انتخابشده دیگر وجود ندارد.".into()))?;
    let provider = settings
        .providers
        .iter()
        .find(|provider| provider.id == model.provider_id)
        .cloned()
        .ok_or_else(|| AppError::Validation("پروایدر مدل انتخابشده پیدا نشد.".into()))?;
    Ok((provider, model))
}

fn project_root_from_settings(settings: &Settings) -> AppResult<Option<PathBuf>> {
    match settings.project_path.as_deref() {
        Some(path) => Ok(Some(canonical_project_root(Path::new(path))?)),
        None => Ok(None),
    }
}

fn conversation_title(content: &str, attachments: &[AttachmentView]) -> String {
    let first = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let title = if first.trim().is_empty() {
        if attachments.is_empty() {
            "گفتوگوی جدید"
        } else {
            "گفتوگو با پیوست"
        }
    } else {
        first
    };
    truncate_chars(title, 70).0
}

fn prepare_attachments(
    root: Option<&Path>,
    inputs: &[AttachmentInput],
    model: &ModelConfig,
) -> AppResult<Vec<AttachmentView>> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let root = root.ok_or_else(|| {
        AppError::Validation("برای پیوست کردن فایل ابتدا پوشهٔ پروژه را انتخاب کنید.".into())
    })?;
    let mut out = Vec::new();
    for input in inputs.iter().take(8) {
        let resolved = project_relative_path(root, &input.path)?;
        let metadata = std::fs::metadata(&resolved)?;
        if metadata.len() > MAX_IMAGE_BYTES as u64 {
            return Err(AppError::Safety("حجم پیوست از سقف مجاز بیشتر است.".into()));
        }
        let name = resolved
            .file_name()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| input.path.clone());
        let mime = guess_mime(&resolved);
        if mime.starts_with("image/") {
            if !model.supports_vision {
                return Err(AppError::Validation(format!(
                    "مدل «{}» توانایی دریافت تصویر ندارد. یک مدل دارای قابلیت vision انتخاب کنید یا تصویر را حذف کنید.",
                    model.display_name
                )));
            }
            let bytes = std::fs::read(&resolved)?;
            let data_url = format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            );
            out.push(AttachmentView {
                path: input.path.clone(),
                name,
                kind: "image".into(),
                mime: mime.into(),
                size: metadata.len(),
                text: None,
                data_url: Some(data_url),
            });
        } else {
            let text = safe_read_text(&resolved, 256_000)?;
            let (text, _) = truncate_chars(&text, 40_000);
            out.push(AttachmentView {
                path: input.path.clone(),
                name,
                kind: "text".into(),
                mime: mime.into(),
                size: metadata.len(),
                text: Some(text),
                data_url: None,
            });
        }
    }
    Ok(out)
}

fn guess_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "json" => "application/json",
        "md" | "markdown" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" | "cjs" | "ts" | "tsx" => "text/javascript",
        "rs" => "text/rust",
        "py" => "text/x-python",
        "toml" => "text/toml",
        "yaml" | "yml" => "text/yaml",
        _ => "text/plain",
    }
}

pub async fn run_agent(run: AgentRun) -> AppResult<()> {
    let mut messages =
        build_llm_messages(&run.state, &run.conversation_id, &run.model, &run.settings).await?;
    let tools = if run.model.supports_tools {
        definitions(&run.settings, run.project_root.is_some())
    } else {
        json!([])
    };
    let runner = ToolRunner::new(ToolContext {
        state: run.state.clone(),
        bus: run.bus.clone(),
        conversation_id: run.conversation_id.clone(),
        project_root: run.project_root.clone(),
        settings: run.settings.clone(),
        stop: run.stop.clone(),
    });
    let mut repeated_calls: HashMap<String, usize> = HashMap::new();
    let mut current_assistant_id = run.initial_assistant_message_id.clone();
    for iteration in 0..run.settings.max_tool_iterations {
        if run.stop.load(Ordering::SeqCst) {
            return stop_run(&run, &current_assistant_id, String::new()).await;
        }
        let assistant_message_id = if iteration == 0 {
            current_assistant_id.clone()
        } else {
            let id = create_assistant_placeholder(&run.state, &run.conversation_id).await?;
            current_assistant_id = id.clone();
            id
        };
        run.bus.emit(AgentEvent::Status {
            conversation_id: run.conversation_id.clone(),
            status: if iteration == 0 {
                "connecting".into()
            } else {
                "thinking".into()
            },
            message: if iteration == 0 {
                "در حال اتصال به پروایدر…".into()
            } else {
                "در حال پردازش نتیجهٔ ابزار…".into()
            },
        })?;
        let outcome = stream_with_retries(&run, &messages, &tools, &assistant_message_id).await?;
        if outcome.stopped {
            return stop_run(&run, &assistant_message_id, outcome.content.clone()).await;
        }
        let assistant = MessageView {
            id: assistant_message_id.clone(),
            conversation_id: run.conversation_id.clone(),
            role: "assistant".into(),
            content: outcome.content.clone(),
            attachments: Vec::new(),
            tool_calls: outcome
                .tool_calls
                .iter()
                .map(|call| ToolCallView {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                })
                .collect(),
            tool_call_id: None,
            status: "complete".into(),
            created_at: now_rfc3339(),
            seq: 0,
        };
        {
            let conn = run.state.db.lock().await;
            db::insert_message(&conn, &assistant)?;
        }
        run.bus.emit(AgentEvent::MessageDone {
            message: assistant.clone(),
        })?;
        messages.push(to_provider_assistant_message(&assistant));
        if outcome.tool_calls.is_empty() {
            if outcome.content.trim().is_empty() {
                run.bus.emit(AgentEvent::Error {
                    conversation_id: run.conversation_id.clone(),
                    message: "پروایدر پاسخی تولید نکرد. مدل یا تنظیمات پروایدر را بررسی کنید."
                        .into(),
                    retryable: true,
                })?;
                return Ok(());
            }
            run.bus.emit(AgentEvent::Done {
                conversation_id: run.conversation_id.clone(),
                message_id: assistant_message_id,
            })?;
            return Ok(());
        }
        if tools.as_array().map(|v| v.is_empty()).unwrap_or(true) {
            let message = "مدل انتخابشده درخواست ابزار داد، در حالی که قابلیت فراخوانی ابزار برای آن فعال نیست.".into();
            run.bus.emit(AgentEvent::Error {
                conversation_id: run.conversation_id.clone(),
                message,
                retryable: false,
            })?;
            return Ok(());
        }
        for call in outcome.tool_calls {
            if run.stop.load(Ordering::SeqCst) {
                return stop_run(&run, &assistant_message_id, assistant.content.clone()).await;
            }
            let signature = hash_text(&format!("{}:{}", call.name, call.arguments));
            let count = repeated_calls.entry(signature).or_insert(0);
            *count += 1;
            if *count > 2 {
                let message = "حلقهٔ تکراری فراخوانی ابزار شناسایی و متوقف شد.".into();
                run.bus.emit(AgentEvent::Error {
                    conversation_id: run.conversation_id.clone(),
                    message,
                    retryable: false,
                })?;
                return Ok(());
            }
            let result = runner.execute(call.clone()).await?;
            let tool_message = MessageView {
                id: new_id("msg"),
                conversation_id: run.conversation_id.clone(),
                role: "tool".into(),
                content: result.output.clone(),
                attachments: Vec::new(),
                tool_calls: Vec::new(),
                tool_call_id: Some(call.id.clone()),
                status: if result.is_error {
                    "error".into()
                } else {
                    "complete".into()
                },
                created_at: now_rfc3339(),
                seq: 0,
            };
            {
                let conn = run.state.db.lock().await;
                db::insert_message(&conn, &tool_message)?;
            }
            messages.push(json!({
                "role": "tool",
                "name": call.name,
                "tool_call_id": call.id,
                "content": result.output
            }));
            if result.denied {
                let message = "مجوز کاربر رد شد؛ اجرای ابزار و ادامهٔ حلقه متوقف شد.".into();
                run.bus.emit(AgentEvent::Error {
                    conversation_id: run.conversation_id.clone(),
                    message,
                    retryable: false,
                })?;
                run.bus.emit(AgentEvent::Stopped {
                    conversation_id: run.conversation_id.clone(),
                    message_id: assistant_message_id,
                })?;
                return Ok(());
            }
            if !result.images.is_empty() && run.model.supports_vision {
                messages.push(synthetic_image_message(&result.images));
            }
        }
        if iteration + 1 >= run.settings.max_tool_iterations {
            let message = format!(
                "سقف {} مرحله از حلقهٔ ابزار رسید. برای ادامه، پیام تازهای بفرستید.",
                run.settings.max_tool_iterations
            );
            run.bus.emit(AgentEvent::Error {
                conversation_id: run.conversation_id.clone(),
                message,
                retryable: false,
            })?;
            return Ok(());
        }
    }
    Ok(())
}

async fn create_assistant_placeholder(
    state: &Arc<AppState>,
    conversation_id: &str,
) -> AppResult<String> {
    let message = MessageView {
        id: new_id("msg"),
        conversation_id: conversation_id.into(),
        role: "assistant".into(),
        content: String::new(),
        attachments: Vec::new(),
        tool_calls: Vec::new(),
        tool_call_id: None,
        status: "streaming".into(),
        created_at: now_rfc3339(),
        seq: 0,
    };
    let id = message.id.clone();
    let conn = state.db.lock().await;
    db::insert_message(&conn, &message)?;
    Ok(id)
}

async fn stop_run(run: &AgentRun, message_id: &str, content: String) -> AppResult<()> {
    let message = MessageView {
        id: message_id.into(),
        conversation_id: run.conversation_id.clone(),
        role: "assistant".into(),
        content,
        attachments: Vec::new(),
        tool_calls: Vec::new(),
        tool_call_id: None,
        status: "stopped".into(),
        created_at: now_rfc3339(),
        seq: 0,
    };
    let conn = run.state.db.lock().await;
    db::insert_message(&conn, &message)?;
    drop(conn);
    run.bus.emit(AgentEvent::MessageDone {
        message: message.clone(),
    })?;
    run.bus.emit(AgentEvent::Stopped {
        conversation_id: run.conversation_id.clone(),
        message_id: message_id.into(),
    })
}

fn to_provider_assistant_message(message: &MessageView) -> Value {
    let mut value = json!({
        "role": "assistant",
        "content": message.content
    });
    if !message.tool_calls.is_empty() {
        value["tool_calls"] = Value::Array(
            message
                .tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": call.arguments
                        }
                    })
                })
                .collect(),
        );
    }
    value
}

fn synthetic_image_message(images: &[String]) -> Value {
    let mut parts = vec![json!({
        "type": "text",
        "text": "تصویر صفحه بهصورت امن و موقت ضمیمه شد. لطفاً آن را تحلیل کن."
    })];
    for image in images.iter().take(2) {
        parts.push(json!({
            "type": "image_url",
            "image_url": { "url": image }
        }));
    }
    json!({ "role": "user", "content": Value::Array(parts) })
}

async fn build_llm_messages(
    state: &Arc<AppState>,
    conversation_id: &str,
    model: &ModelConfig,
    settings: &Settings,
) -> AppResult<Vec<Value>> {
    let history = {
        let conn = state.db.lock().await;
        db::list_messages(&conn, conversation_id)?
    };
    let limited = limit_history(history);
    let project = settings
        .project_path
        .clone()
        .unwrap_or_else(|| "هیچ پوشه پروژهای باز نیست".into());
    let system = format!(
        "شما CinderDock هستید، یک عامل کدنویسی محلی برای ویندوز. تاریخ: {}. پوشه پروژه: {}. \
         قوانین: هرگز پیش از دریافت نتیجهٔ واقعی ابزار ادعای موفقیت نکنید. برای خواندن و تغییر فایلها فقط از ابزارهای ارائهشده استفاده کنید. \
         تغییر فایل و اجرای فرمان نیازمند تأیید کاربر است. اطلاعات محرمانه، کلیدها و فایلهای حساس را افشا نکن. \
         پاسخها را روشن، کوتاه و در صورت نیاز همراه با قطعهکد بده.",
        chrono::Utc::now().date_naive(),
        project
    );
    let mut out = vec![json!({ "role": "system", "content": system })];
    let mut tool_names = HashMap::new();
    for message in &limited {
        if message.role == "assistant" {
            for call in &message.tool_calls {
                tool_names.insert(call.id.clone(), call.name.clone());
            }
        }
    }
    for message in limited {
        if message.status == "streaming" && message.content.trim().is_empty() {
            continue;
        }
        match message.role.as_str() {
            "user" => out.push(user_message_value(&message, model.supports_vision)),
            "assistant" => out.push(to_provider_assistant_message(&message)),
            "tool" => {
                let name = message
                    .tool_call_id
                    .as_deref()
                    .and_then(|id| tool_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "tool".into());
                out.push(json!({
                    "role": "tool",
                    "name": name,
                    "tool_call_id": message.tool_call_id,
                    "content": message.content
                }));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn limit_history(history: Vec<MessageView>) -> Vec<MessageView> {
    let mut out = Vec::new();
    let mut chars = 0usize;
    for message in history.into_iter().rev() {
        chars += message.content.chars().count();
        if chars > MAX_HISTORY_CHARS && out.len() >= 2 {
            break;
        }
        out.push(message);
    }
    out.reverse();
    out
}

fn user_message_value(message: &MessageView, supports_vision: bool) -> Value {
    let mut text = message.content.clone();
    let mut images = Vec::new();
    for attachment in &message.attachments {
        match attachment.kind.as_str() {
            "text" => {
                if let Some(excerpt) = &attachment.text {
                    let (excerpt, _) = truncate_chars(excerpt, 20_000);
                    text.push_str(&format!(
                        "\n\n--- FILE: {} ---\n{}",
                        attachment.path, excerpt
                    ));
                }
            }
            "image" => {
                if supports_vision {
                    if let Some(data_url) = &attachment.data_url {
                        if data_url.len() <= MAX_IMAGE_BYTES * 2 {
                            images.push(json!({
                                "type": "image_url",
                                "image_url": { "url": data_url }
                            }));
                        } else {
                            text.push_str(&format!(
                                "\n\n[تصویر {} بهدلیل اندازهٔ زیاد ارسال نشد.]",
                                attachment.path
                            ));
                        }
                    }
                } else {
                    text.push_str(&format!(
                        "\n\n[تصویر {} حذف شد: مدل فعلی توانایی دیدن تصویر ندارد.]",
                        attachment.path
                    ));
                }
            }
            _ => {}
        }
    }
    if images.is_empty() {
        json!({ "role": "user", "content": text })
    } else {
        let mut parts = vec![json!({ "type": "text", "text": text })];
        parts.extend(images);
        json!({ "role": "user", "content": Value::Array(parts) })
    }
}

async fn stream_with_retries(
    run: &AgentRun,
    messages: &[Value],
    tools: &Value,
    message_id: &str,
) -> AppResult<StreamOutcome> {
    for attempt in 0..=run.settings.max_retries {
        if run.stop.load(Ordering::SeqCst) {
            return Ok(StreamOutcome {
                content: String::new(),
                tool_calls: Vec::new(),
                stopped: true,
            });
        }
        if attempt > 0 {
            run.bus.emit(AgentEvent::Status {
                conversation_id: run.conversation_id.clone(),
                status: "retrying".into(),
                message: format!("تلاش دوبارهٔ {} از {}", attempt, run.settings.max_retries),
            })?;
        }
        match stream_chat(run, messages, tools, message_id).await {
            StreamAttempt::Success(outcome) => {
                run.bus.emit(AgentEvent::Connection {
                    provider_id: run.provider.id.clone(),
                    state: "online".into(),
                    message: "اتصال و جریان پاسخ برقرار است.".into(),
                })?;
                return Ok(outcome);
            }
            StreamAttempt::Failure {
                error,
                had_delta,
                retryable,
            } => {
                let message = run.bus.redact(&error.to_string());
                let can_retry = retryable && !had_delta && attempt < run.settings.max_retries;
                run.bus.emit(AgentEvent::Connection {
                    provider_id: run.provider.id.clone(),
                    state: if can_retry { "retrying" } else { "offline" }.into(),
                    message: message.clone(),
                })?;
                if can_retry {
                    tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await;
                    continue;
                }
                return Err(error);
            }
        }
    }
    Err(AppError::Provider("تلاش دوباره به سقف مجاز رسید.".into()))
}

async fn stream_chat(
    run: &AgentRun,
    messages: &[Value],
    tools: &Value,
    message_id: &str,
) -> StreamAttempt {
    let secret = match providers::resolve_secret(
        &run.provider,
        None,
        run.state.secrets.get(&run.provider.id),
    ) {
        Ok(secret) => secret,
        Err(error) => {
            return StreamAttempt::Failure {
                error,
                had_delta: false,
                retryable: false,
            }
        }
    };
    let payload =
        providers::build_chat_payload(run.provider.kind, &run.model.model_id, messages, tools);
    let request = match providers::chat_request(
        &run.state.client,
        &run.provider,
        secret.as_deref(),
        &payload,
    ) {
        Ok(request) => request,
        Err(error) => {
            return StreamAttempt::Failure {
                error,
                had_delta: false,
                retryable: false,
            }
        }
    };
    let mut response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return StreamAttempt::Failure {
                error: AppError::Provider(format!(
                    "اتصال به پروایدر برقرار نشد: {}",
                    run.bus.redact(&error.to_string())
                )),
                had_delta: false,
                retryable: true,
            }
        }
    };
    if !response.status().is_success() {
        let status = response.status();
        let body = providers::read_limited_body(&mut response, 64_000)
            .await
            .unwrap_or_default();
        let clean = run.bus.redact(&body);
        let (clean, _) = truncate_chars(&clean, 2000);
        let error = AppError::Provider(format!("HTTP {} از پروایدر: {}", status.as_u16(), clean));
        let retryable = status.is_server_error() || status.as_u16() == 429;
        return StreamAttempt::Failure {
            error,
            had_delta: false,
            retryable,
        };
    }
    let mut parser = StreamParser::new(run.provider.kind);
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut streamed_content = String::new();
    let mut total_bytes = 0usize;
    let started = Instant::now();
    loop {
        if run.stop.load(Ordering::SeqCst) {
            return StreamAttempt::Success(StreamOutcome {
                content: streamed_content,
                tool_calls: Vec::new(),
                stopped: true,
            });
        }
        if started.elapsed() > Duration::from_secs(180) {
            return StreamAttempt::Failure {
                error: AppError::Provider("زمان جریان پاسخ به پایان رسید.".into()),
                had_delta: !streamed_content.is_empty(),
                retryable: false,
            };
        }
        let Some(chunk) = stream.next().await else {
            break;
        };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                return StreamAttempt::Failure {
                    error: AppError::Provider(format!(
                        "جریان پاسخ قطع شد: {}",
                        run.bus.redact(&error.to_string())
                    )),
                    had_delta: !streamed_content.is_empty(),
                    retryable: false,
                }
            }
        };
        total_bytes += chunk.len();
        if total_bytes > MAX_STREAM_BYTES {
            return StreamAttempt::Failure {
                error: AppError::Safety("خروجی جریان از سقف مجاز بیشتر شد.".into()),
                had_delta: !streamed_content.is_empty(),
                retryable: false,
            };
        }
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(position) = buffer.find('\n') {
            let line: String = buffer.drain(..=position).collect();
            let line = line.trim_end_matches(['\n', '\r']);
            match parser.ingest(line) {
                Ok(update) => {
                    if !update.delta.is_empty() {
                        streamed_content.push_str(&update.delta);
                        if streamed_content.chars().count() > MAX_STREAM_CHARS {
                            return StreamAttempt::Failure {
                                error: AppError::Safety("متن پاسخ از سقف مجاز بیشتر شد.".into()),
                                had_delta: true,
                                retryable: false,
                            };
                        }
                        if let Err(error) = run.bus.emit(AgentEvent::MessageDelta {
                            conversation_id: run.conversation_id.clone(),
                            message_id: message_id.into(),
                            delta: update.delta,
                        }) {
                            return StreamAttempt::Failure {
                                error,
                                had_delta: true,
                                retryable: false,
                            };
                        }
                    }
                }
                Err(error) => {
                    return StreamAttempt::Failure {
                        error,
                        had_delta: !streamed_content.is_empty(),
                        retryable: false,
                    }
                }
            }
            if parser.is_finished() {
                break;
            }
        }
        if parser.is_finished() {
            break;
        }
    }
    if !buffer.trim().is_empty() && !parser.is_finished() {
        if let Err(error) = parser.ingest(&buffer) {
            return StreamAttempt::Failure {
                error,
                had_delta: !streamed_content.is_empty(),
                retryable: false,
            };
        }
    }
    let mut outcome = match parser.finish() {
        Ok(outcome) => outcome,
        Err(error) => {
            return StreamAttempt::Failure {
                error,
                had_delta: !streamed_content.is_empty(),
                retryable: false,
            }
        }
    };
    if let Some(extra) = outcome.content.strip_prefix(streamed_content.as_str()) {
        if !extra.is_empty() {
            let _ = run.bus.emit(AgentEvent::MessageDelta {
                conversation_id: run.conversation_id.clone(),
                message_id: message_id.into(),
                delta: extra.into(),
            });
        }
    }
    outcome.stopped = run.stop.load(Ordering::SeqCst);
    StreamAttempt::Success(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_limited_by_character_budget() {
        let mut history = Vec::new();
        for index in 0..20 {
            history.push(MessageView {
                id: format!("m{index}"),
                conversation_id: "c".into(),
                role: "user".into(),
                content: "x".repeat(20_000),
                attachments: Vec::new(),
                tool_calls: Vec::new(),
                tool_call_id: None,
                status: "complete".into(),
                created_at: "now".into(),
                seq: index,
            });
        }
        let limited = limit_history(history);
        assert!(limited.len() < 20);
        assert!(limited.len() >= 2);
    }

    #[test]
    fn non_vision_models_do_not_receive_images() {
        let message = MessageView {
            id: "m".into(),
            conversation_id: "c".into(),
            role: "user".into(),
            content: "see image".into(),
            attachments: vec![AttachmentView {
                path: "a.png".into(),
                name: "a.png".into(),
                kind: "image".into(),
                mime: "image/png".into(),
                size: 10,
                text: None,
                data_url: Some("data:image/png;base64,QQ==".into()),
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            status: "complete".into(),
            created_at: "now".into(),
            seq: 1,
        };
        let value = user_message_value(&message, false);
        assert!(value.get("content").unwrap().is_string());
        assert!(value.to_string().contains("توانایی دیدن تصویر"));
    }

    #[test]
    fn synthetic_image_message_uses_vision_content_parts() {
        let value = synthetic_image_message(&["data:image/png;base64,QQ==".into()]);
        assert_eq!(value.pointer("/content/1/type").unwrap(), "image_url");
    }
}
