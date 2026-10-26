use std::{collections::BTreeMap, time::Duration};

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    redirect::Policy,
    Client, RequestBuilder, Response, StatusCode,
};
use serde_json::{json, Map, Value};
use url::Url;

use crate::{
    error::{AppError, AppResult},
    models::{
        AuthConfig, AuthMethod, ModelInput, ProviderConfig, ProviderInput, ProviderKind,
        ProviderPreset, RemoteModel, SuggestedModel, TestConnectionResult,
    },
    security::{looks_like_secret, redact_text, MAX_WEB_BYTES},
};

const RESERVED_HEADERS: &[&str] = &[
    "authorization",
    "api-key",
    "x-api-key",
    "cookie",
    "set-cookie",
    "host",
    "content-length",
];

pub fn build_client() -> AppResult<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(10))
        .redirect(Policy::limited(3))
        .user_agent(format!("CinderDock/{}", env!("CARGO_PKG_VERSION")))
        .build()?)
}

pub fn preset_catalog() -> Vec<ProviderPreset> {
    vec![
        ProviderPreset {
            id: "openai".into(),
            label: "OpenAI".into(),
            description: "سازگار با OpenAI Chat Completions و فهرست /models".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "https://api.openai.com/v1".into(),
            auth_method: AuthMethod::Bearer,
            models_path: "models".into(),
            chat_path: "chat/completions".into(),
            suggested_models: vec![
                SuggestedModel {
                    id: "gpt-4o".into(),
                    display_name: "GPT-4o".into(),
                    supports_tools: true,
                    supports_vision: true,
                    context_window: Some(128_000),
                    note: "قابلیتها را با پاسخ حساب خود و مستندات روز پروایدر تأیید کنید.".into(),
                },
                SuggestedModel {
                    id: "gpt-4o-mini".into(),
                    display_name: "GPT-4o mini".into(),
                    supports_tools: true,
                    supports_vision: true,
                    context_window: Some(128_000),
                    note: "قابلیتها را با پاسخ حساب خود و مستندات روز پروایدر تأیید کنید.".into(),
                },
            ],
            supports_model_discovery: true,
            notes: "کلید API فقط در مخزن امن ویندوز ذخیره میشود.".into(),
        },
        ProviderPreset {
            id: "openrouter".into(),
            label: "OpenRouter".into(),
            description: "سازگار با OpenAI Chat Completions؛ شناسهها معمولاً publisher/model هستند".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "https://openrouter.ai/api/v1".into(),
            auth_method: AuthMethod::Bearer,
            models_path: "models".into(),
            chat_path: "chat/completions".into(),
            suggested_models: vec![SuggestedModel {
                id: "openai/gpt-4o".into(),
                display_name: "OpenAI GPT-4o (OpenRouter)".into(),
                supports_tools: true,
                supports_vision: true,
                context_window: Some(128_000),
                note: "قابلیت واقعی را از فهرست مدلهای OpenRouter بررسی کنید.".into(),
            }],
            supports_model_discovery: true,
            notes: "هدرهای اختیاری غیرمحرمانه مانند HTTP-Referer اینجا قابل تنظیماند.".into(),
        },
        ProviderPreset {
            id: "deepseek".into(),
            label: "DeepSeek".into(),
            description: "سازگار با OpenAI Chat Completions در نقطهٔ پایانی DeepSeek".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "https://api.deepseek.com/v1".into(),
            auth_method: AuthMethod::Bearer,
            models_path: "models".into(),
            chat_path: "chat/completions".into(),
            suggested_models: vec![SuggestedModel {
                id: "deepseek-chat".into(),
                display_name: "DeepSeek Chat".into(),
                supports_tools: false,
                supports_vision: false,
                context_window: Some(64_000),
                note: "برای فراخوانی ابزار یا تصویر، پشتیبانی مدل را از پروایدر تأیید کنید.".into(),
            }],
            supports_model_discovery: true,
            notes: "شناسهٔ دقیق مدل را همیشه از فهرست مدلهای حساب خود بردارید.".into(),
        },
        ProviderPreset {
            id: "ollama-native".into(),
            label: "Ollama (اتصالدهنده بومی)".into(),
            description: "از /api/chat و /api/tags بومی Ollama استفاده میکند؛ قالب OpenAI ادعا نمیشود".into(),
            kind: ProviderKind::OllamaNative,
            base_url: "http://127.0.0.1:11434".into(),
            auth_method: AuthMethod::None,
            models_path: "api/tags".into(),
            chat_path: "api/chat".into(),
            suggested_models: vec![SuggestedModel {
                id: "qwen2.5-coder".into(),
                display_name: "Qwen 2.5 Coder (در صورت نصب)".into(),
                supports_tools: true,
                supports_vision: false,
                context_window: Some(32_000),
                note: "فقط اگر قبلاً با ollama pull نصب شده باشد.".into(),
            }],
            supports_model_discovery: true,
            notes: "فراخوانی ابزار بومی Ollama فقط در مدلها و نسخههای سازگار کار میکند.".into(),
        },
        ProviderPreset {
            id: "ollama-openai".into(),
            label: "Ollama (سازگار با OpenAI)".into(),
            description: "از نقطهٔ پایانی OpenAI-compatible اختیاری Ollama در /v1 استفاده میکند".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "http://127.0.0.1:11434/v1".into(),
            auth_method: AuthMethod::None,
            models_path: "models".into(),
            chat_path: "chat/completions".into(),
            suggested_models: Vec::new(),
            supports_model_discovery: true,
            notes: "اگر نسخه Ollama شما /v1 را ارائه نمیکند، از اتصالدهنده بومی استفاده کنید.".into(),
        },
        ProviderPreset {
            id: "lmstudio".into(),
            label: "LM Studio".into(),
            description: "سرور محلی LM Studio با Chat Completions سازگار با OpenAI".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "http://127.0.0.1:1234/v1".into(),
            auth_method: AuthMethod::None,
            models_path: "models".into(),
            chat_path: "chat/completions".into(),
            suggested_models: Vec::new(),
            supports_model_discovery: true,
            notes: "ابتدا سرور محلی LM Studio را روشن کنید، سپس فهرست مدلها را بگیرید.".into(),
        },
        ProviderPreset {
            id: "custom".into(),
            label: "پروایدر سفارشی".into(),
            description: "هر سرور واقعاً سازگار با OpenAI Chat Completions".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "http://127.0.0.1:8000/v1".into(),
            auth_method: AuthMethod::Bearer,
            models_path: "models".into(),
            chat_path: "chat/completions".into(),
            suggested_models: Vec::new(),
            supports_model_discovery: true,
            notes: "اگر سرور شما قالب OpenAI ندارد، preset آن را انتخاب کنید یا شناسه و مسیرها را بهدقت وارد نمایید.".into(),
        },
    ]
}

pub fn preset_by_id(id: &str) -> Option<ProviderPreset> {
    preset_catalog().into_iter().find(|p| p.id == id)
}

pub fn normalize_base_url(raw: &str) -> AppResult<String> {
    let mut url = Url::parse(raw.trim())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::Validation(
            "آدرس پروایدر باید http یا https باشد.".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() {
        return Err(AppError::Safety(
            "Base URL نباید شامل نام کاربری، رمز یا query باشد.".into(),
        ));
    }
    let mut path = url.path().trim_end_matches('/').to_string();
    path.push('/');
    url.set_path(&path);
    url.set_fragment(None);
    Ok(url.to_string())
}

pub fn endpoint(base_url: &str, path: &str) -> AppResult<Url> {
    let base = normalize_base_url(base_url)?;
    let base = Url::parse(&base)?;
    if path.trim().is_empty() {
        return Ok(base);
    }
    Ok(base.join(path.trim())?)
}

pub fn normalize_provider(
    input: ProviderInput,
    existing_id: Option<&str>,
) -> AppResult<(ProviderConfig, Option<String>)> {
    let display_name = input.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::Validation("نام نمایشی پروایدر لازم است.".into()));
    }
    let id = input
        .id
        .clone()
        .or_else(|| existing_id.map(str::to_string))
        .unwrap_or_else(|| crate::security::new_id("provider"));
    let fallback_models = match input.kind {
        ProviderKind::OpenAiCompatible => "models",
        ProviderKind::OllamaNative => "api/tags",
    };
    let fallback_chat = match input.kind {
        ProviderKind::OpenAiCompatible => "chat/completions",
        ProviderKind::OllamaNative => "api/chat",
    };
    let models_path = input.models_path.unwrap_or_else(|| fallback_models.into());
    let chat_path = input.chat_path.unwrap_or_else(|| fallback_chat.into());
    if chat_path.trim().is_empty() {
        return Err(AppError::Validation(
            "مسیر chat/completions یا معادل آن لازم است.".into(),
        ));
    }
    if models_path.contains("://") || chat_path.contains("://") {
        return Err(AppError::Safety(
            "مسیرهای نقطهٔ پایانی باید نسبت به Base URL باشند.".into(),
        ));
    }
    let mut extra_headers = BTreeMap::new();
    for (name, value) in input.extra_headers {
        let name = name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        if RESERVED_HEADERS.contains(&name.to_ascii_lowercase().as_str())
            || looks_like_secret(&name)
            || looks_like_secret(&value)
        {
            return Err(AppError::Safety(format!(
                "هدر «{name}» مجاز نیست یا ممکن است حاوی کلید باشد."
            )));
        }
        HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| AppError::Validation(format!("نام هدر نامعتبر است: {name}")))?;
        HeaderValue::from_str(value.trim())
            .map_err(|_| AppError::Validation(format!("مقدار هدر نامعتبر است: {name}")))?;
        extra_headers.insert(name, value.trim().to_string());
    }
    let auth_header = input
        .auth_header
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());
    if input.auth_method == AuthMethod::CustomHeader && auth_header.is_none() {
        return Err(AppError::Validation(
            "برای احراز هویت با هدر سفارشی، نام هدر لازم است.".into(),
        ));
    }
    let config = ProviderConfig {
        id,
        display_name: display_name.into(),
        preset: input.preset,
        kind: input.kind,
        base_url: normalize_base_url(&input.base_url)?,
        auth: AuthConfig {
            method: input.auth_method,
            header_name: auth_header,
        },
        extra_headers,
        models_path,
        chat_path,
    };
    Ok((
        config,
        input
            .api_key
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty()),
    ))
}

pub fn resolve_secret(
    provider: &ProviderConfig,
    supplied: Option<&str>,
    stored: Option<String>,
) -> AppResult<Option<String>> {
    match provider.auth.method {
        AuthMethod::None => Ok(None),
        AuthMethod::Bearer | AuthMethod::CustomHeader => {
            let secret = supplied
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .or(stored)
                .filter(|x| !x.trim().is_empty());
            match secret {
                Some(secret) => Ok(Some(secret)),
                None => Err(AppError::Validation(
                    "کلید API برای این پروایدر لازم است.".into(),
                )),
            }
        }
    }
}

pub fn build_headers(provider: &ProviderConfig, secret: Option<&str>) -> AppResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    match provider.auth.method {
        AuthMethod::Bearer => {
            let key = secret.ok_or_else(|| AppError::Validation("کلید API موجود نیست.".into()))?;
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|_| AppError::Validation("کلید API شامل نویسه نامعتبر است.".into()))?,
            );
        }
        AuthMethod::CustomHeader => {
            let key = secret.ok_or_else(|| AppError::Validation("کلید API موجود نیست.".into()))?;
            let name = provider
                .auth
                .header_name
                .as_deref()
                .ok_or_else(|| AppError::Validation("نام هدر سفارشی موجود نیست.".into()))?;
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| AppError::Validation("نام هدر سفارشی نامعتبر است.".into()))?;
            headers.insert(
                header_name,
                HeaderValue::from_str(key).map_err(|_| {
                    AppError::Validation("مقدار هدر احراز هویت نامعتبر است.".into())
                })?,
            );
        }
        AuthMethod::None => {}
    }
    for (name, value) in &provider.extra_headers {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| AppError::Validation(format!("نام هدر نامعتبر است: {name}")))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|_| AppError::Validation(format!("مقدار هدر نامعتبر است: {name}")))?;
        headers.insert(header_name, header_value);
    }
    Ok(headers)
}

pub async fn read_limited_body(response: &mut Response, max: usize) -> AppResult<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > max {
            let remaining = max.saturating_sub(bytes.len());
            bytes.extend_from_slice(&chunk[..remaining]);
            bytes.extend_from_slice(b"\n[CinderDock: body truncated]");
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn error_from_response(status: StatusCode, body: &str, secrets: &[String]) -> AppError {
    let (clean, _) = crate::security::truncate_chars(&redact_text(secrets, body), 2000);
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        AppError::Provider("احراز هویت رد شد. کلید API یا هدرها را بررسی کنید.".into())
    } else {
        AppError::Provider(format!("HTTP {}: {}", status.as_u16(), clean))
    }
}

pub async fn test_connection(
    client: &Client,
    provider: &ProviderConfig,
    secret: Option<&str>,
    all_secrets: &[String],
) -> AppResult<TestConnectionResult> {
    let started = std::time::Instant::now();
    let headers = build_headers(provider, secret)?;
    let models_path = provider.models_path.trim();
    let url = endpoint(&provider.base_url, models_path)?;
    let response = client.get(url).headers(headers.clone()).send().await;
    let Ok(mut response) = response else {
        return Ok(TestConnectionResult {
            ok: false,
            status: "offline".into(),
            message: "اتصال برقرار نشد. آدرس، اینترنت یا سرور محلی را بررسی کنید.".into(),
            model_count: None,
            latency_ms: started.elapsed().as_millis(),
        });
    };
    let status = response.status();
    let body = read_limited_body(&mut response, MAX_WEB_BYTES).await?;
    if status.is_success() {
        let count = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| parse_models_payload(provider.kind, &value).ok())
            .map(|models| models.len());
        return Ok(TestConnectionResult {
            ok: true,
            status: "online".into(),
            message: if count.is_some() {
                "اتصال برقرار شد و فهرست مدلها دریافت شد.".into()
            } else {
                "اتصال برقرار شد، اما ساختار فهرست مدلها قابل تشخیص نبود.".into()
            },
            model_count: count,
            latency_ms: started.elapsed().as_millis(),
        });
    }
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Ok(TestConnectionResult {
            ok: false,
            status: "auth_error".into(),
            message: "اتصال برقرار شد ولی کلید API پذیرفته نشد.".into(),
            model_count: None,
            latency_ms: started.elapsed().as_millis(),
        });
    }
    if matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED
    ) {
        let root = endpoint(&provider.base_url, "")?;
        let root_response = client.get(root).headers(headers).send().await;
        if let Ok(mut root_response) = root_response {
            let root_status = root_response.status();
            let _ = read_limited_body(&mut root_response, 64_000).await?;
            if root_status.is_success()
                || root_status == StatusCode::NOT_FOUND
                || root_status == StatusCode::METHOD_NOT_ALLOWED
            {
                return Ok(TestConnectionResult {
                    ok: true,
                    status: "discovery_unavailable".into(),
                    message:
                        "سرور پاسخ داد؛ فهرست مدلها در دسترس نیست. شناسهٔ مدل را دستی وارد کنید."
                            .into(),
                    model_count: None,
                    latency_ms: started.elapsed().as_millis(),
                });
            }
        }
    }
    Err(error_from_response(status, &body, all_secrets))
}

pub async fn fetch_models(
    client: &Client,
    provider: &ProviderConfig,
    secret: Option<&str>,
    all_secrets: &[String],
) -> AppResult<Vec<RemoteModel>> {
    let headers = build_headers(provider, secret)?;
    let url = endpoint(&provider.base_url, &provider.models_path)?;
    let mut response = client.get(url).headers(headers).send().await?;
    let status = response.status();
    let body = read_limited_body(&mut response, MAX_WEB_BYTES).await?;
    if !status.is_success() {
        return Err(error_from_response(status, &body, all_secrets));
    }
    let value: Value = serde_json::from_str(&body)?;
    parse_models_payload(provider.kind, &value)
}

pub fn parse_models_payload(kind: ProviderKind, value: &Value) -> AppResult<Vec<RemoteModel>> {
    match kind {
        ProviderKind::OpenAiCompatible => {
            let data = value
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AppError::Provider("فهرست مدلها فیلد data ندارد.".into()))?;
            let mut out = Vec::new();
            for item in data {
                let Some(id) = item.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let display_name = item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .to_string();
                let modalities = item.pointer("/architecture/input_modalities");
                let supports_vision = modalities.map(|m| {
                    m.as_array()
                        .map(|values| values.iter().any(|v| v.as_str() == Some("image")))
                });
                let supported = item.get("supported_parameters").and_then(Value::as_array);
                let supports_tools = supported.map(|values| {
                    values
                        .iter()
                        .any(|v| matches!(v.as_str(), Some("tools") | Some("tool_calls")))
                });
                let context_window =
                    item.get("context_length")
                        .and_then(Value::as_u64)
                        .or_else(|| {
                            item.pointer("/top_provider/context_length")
                                .and_then(Value::as_u64)
                        });
                out.push(RemoteModel {
                    id: id.to_string(),
                    display_name,
                    supports_tools,
                    supports_vision,
                    context_window,
                });
            }
            if out.is_empty() {
                return Err(AppError::Provider(
                    "هیچ مدل قابل استفادهٔ واقعی در پاسخ پروایدر پیدا نشد.".into(),
                ));
            }
            Ok(out)
        }
        ProviderKind::OllamaNative => {
            let data = value
                .get("models")
                .and_then(Value::as_array)
                .ok_or_else(|| AppError::Provider("پاسخ Ollama فیلد models ندارد.".into()))?;
            let mut out = Vec::new();
            for item in data {
                let Some(id) = item.get("name").and_then(Value::as_str) else {
                    continue;
                };
                out.push(RemoteModel {
                    id: id.to_string(),
                    display_name: id.to_string(),
                    supports_tools: None,
                    supports_vision: None,
                    // Ollama's parameter_size is not a context window; leave unknown.
                    context_window: None,
                });
            }
            if out.is_empty() {
                return Err(AppError::Provider(
                    "هیچ مدل نصبشدهای در Ollama پیدا نشد.".into(),
                ));
            }
            Ok(out)
        }
    }
}

pub fn model_input_from_remote(provider_id: &str, remote: &RemoteModel) -> ModelInput {
    ModelInput {
        id: None,
        provider_id: provider_id.into(),
        model_id: remote.id.clone(),
        display_name: remote.display_name.clone(),
        supports_tools: remote.supports_tools.unwrap_or(false),
        supports_vision: remote.supports_vision.unwrap_or(false),
        context_window: remote.context_window,
        set_default: false,
    }
}

pub fn build_chat_payload(
    kind: ProviderKind,
    model_id: &str,
    messages: &[Value],
    tools: &Value,
) -> Value {
    let has_tools = tools.as_array().map(|v| !v.is_empty()).unwrap_or(false);
    match kind {
        ProviderKind::OpenAiCompatible => {
            let mut payload = json!({
                "model": model_id,
                "messages": messages,
                "stream": true,
                "temperature": 0.2
            });
            if has_tools {
                payload["tools"] = tools.clone();
                payload["tool_choice"] = Value::String("auto".into());
            }
            payload
        }
        ProviderKind::OllamaNative => {
            let mut payload = json!({
                "model": model_id,
                "messages": to_ollama_messages(messages),
                "stream": true,
                "options": { "temperature": 0.2 }
            });
            if has_tools {
                payload["tools"] = tools.clone();
            }
            payload
        }
    }
}

pub fn chat_request(
    client: &Client,
    provider: &ProviderConfig,
    secret: Option<&str>,
    payload: &Value,
) -> AppResult<RequestBuilder> {
    let headers = build_headers(provider, secret)?;
    let url = endpoint(&provider.base_url, &provider.chat_path)?;
    Ok(client
        .post(url)
        .headers(headers)
        .timeout(Duration::from_secs(180))
        .json(payload))
}

fn to_ollama_messages(messages: &[Value]) -> Vec<Value> {
    let mut tool_names = std::collections::HashMap::new();
    for message in messages {
        if message.get("role").and_then(Value::as_str) == Some("assistant") {
            if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    if let (Some(id), Some(name)) = (
                        call.get("id").and_then(Value::as_str),
                        call.pointer("/function/name").and_then(Value::as_str),
                    ) {
                        tool_names.insert(id.to_string(), name.to_string());
                    }
                }
            }
        }
    }
    messages
        .iter()
        .map(|message| {
            let role = message
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("user");
            let mut out = Map::new();
            out.insert("role".into(), Value::String(role.into()));
            let mut text = String::new();
            let mut images = Vec::new();
            if let Some(content) = message.get("content") {
                match content {
                    Value::String(value) => text.push_str(value),
                    Value::Array(parts) => {
                        for part in parts {
                            if let Some(value) = part.get("text").and_then(Value::as_str) {
                                text.push_str(value);
                            }
                            if let Some(url) =
                                part.pointer("/image_url/url").and_then(Value::as_str)
                            {
                                images.push(Value::String(
                                    url.split_once(',')
                                        .map(|(_, data)| data)
                                        .unwrap_or(url)
                                        .into(),
                                ));
                            }
                        }
                    }
                    other => text.push_str(&other.to_string()),
                }
            }
            out.insert("content".into(), Value::String(text));
            if !images.is_empty() {
                out.insert("images".into(), Value::Array(images));
            }
            if let Some(calls) = message.get("tool_calls") {
                out.insert("tool_calls".into(), normalize_ollama_tool_calls(calls));
            }
            if role == "tool" {
                let name = message
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        message
                            .get("tool_call_id")
                            .and_then(Value::as_str)
                            .and_then(|id| tool_names.get(id).map(String::as_str))
                    })
                    .unwrap_or("tool");
                out.insert("tool_name".into(), Value::String(name.into()));
            }
            Value::Object(out)
        })
        .collect()
}

fn normalize_ollama_tool_calls(value: &Value) -> Value {
    let Some(calls) = value.as_array() else {
        return value.clone();
    };
    Value::Array(
        calls
            .iter()
            .map(|call| {
                let mut function = Map::new();
                if let Some(name) = call.pointer("/function/name") {
                    function.insert("name".into(), name.clone());
                }
                if let Some(description) = call.pointer("/function/description") {
                    function.insert("description".into(), description.clone());
                }
                if let Some(arguments) = call.pointer("/function/arguments") {
                    let parsed = match arguments {
                        Value::String(text) => serde_json::from_str::<Value>(text)
                            .unwrap_or_else(|_| json!({ "raw": text })),
                        other => other.clone(),
                    };
                    function.insert("arguments".into(), parsed);
                }
                json!({ "function": Value::Object(function) })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_include_independent_native_ollama_adapter() {
        let presets = preset_catalog();
        assert!(presets
            .iter()
            .any(|p| p.id == "ollama-native" && p.kind == ProviderKind::OllamaNative));
        assert!(presets.iter().any(|p| p.id == "lmstudio"));
    }

    #[test]
    fn base_url_is_normalized_and_endpoints_are_relative() {
        assert_eq!(
            normalize_base_url("https://api.example.com/v1").unwrap(),
            "https://api.example.com/v1/"
        );
        assert_eq!(
            endpoint("https://api.example.com/v1", "chat/completions")
                .unwrap()
                .as_str(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn rejects_secret_like_extra_headers() {
        let input = ProviderInput {
            id: None,
            display_name: "Bad".into(),
            preset: "custom".into(),
            kind: ProviderKind::OpenAiCompatible,
            base_url: "https://example.com/v1".into(),
            auth_method: AuthMethod::Bearer,
            auth_header: None,
            api_key: Some("sk-test".into()),
            extra_headers: BTreeMap::from([("X-Token".into(), "abc".into())]),
            models_path: None,
            chat_path: None,
        };
        assert!(normalize_provider(input, None).is_err());
    }

    #[test]
    fn parses_openrouter_style_capabilities() {
        let value = json!({
            "data": [{
                "id": "openai/gpt-4o",
                "name": "GPT-4o",
                "architecture": { "input_modalities": ["text", "image"] },
                "supported_parameters": ["tools"],
                "context_length": 128000
            }]
        });
        let models = parse_models_payload(ProviderKind::OpenAiCompatible, &value).unwrap();
        assert_eq!(models[0].id, "openai/gpt-4o");
        assert_eq!(models[0].supports_vision, Some(true));
        assert_eq!(models[0].supports_tools, Some(true));
    }

    #[test]
    fn ollama_payload_uses_native_message_shape() {
        let messages = vec![json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "what is this?" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,QUJD" } }
            ]
        })];
        let payload =
            build_chat_payload(ProviderKind::OllamaNative, "llava", &messages, &json!([]));
        assert_eq!(payload.pointer("/messages/0/images/0").unwrap(), "QUJD");
    }
}
