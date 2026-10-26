use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    OpenAiCompatible,
    OllamaNative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    Bearer,
    None,
    CustomHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub method: AuthMethod,
    #[serde(default)]
    pub header_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    pub preset: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub auth: AuthConfig,
    #[serde(default)]
    pub extra_headers: BTreeMap<String, String>,
    pub models_path: String,
    pub chat_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    #[serde(default)]
    pub id: Option<String>,
    pub display_name: String,
    pub preset: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub extra_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub models_path: Option<String>,
    #[serde(default)]
    pub chat_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub id: String,
    pub display_name: String,
    pub preset: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub auth_method: AuthMethod,
    pub auth_header: Option<String>,
    pub has_secret: bool,
    pub extra_headers: BTreeMap<String, String>,
    pub models_path: String,
    pub chat_path: String,
    pub model_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedModel {
    pub id: String,
    pub display_name: String,
    pub supports_tools: bool,
    pub supports_vision: bool,
    #[serde(default)]
    pub context_window: Option<u64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: String,
    pub label: String,
    pub description: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub auth_method: AuthMethod,
    pub models_path: String,
    pub chat_path: String,
    pub suggested_models: Vec<SuggestedModel>,
    pub supports_model_discovery: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteModel {
    pub id: String,
    pub display_name: String,
    pub supports_tools: Option<bool>,
    pub supports_vision: Option<bool>,
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResult {
    pub ok: bool,
    pub status: String,
    pub message: String,
    pub model_count: Option<usize>,
    pub latency_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInput {
    #[serde(default)]
    pub id: Option<String>,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub supports_tools: bool,
    pub supports_vision: bool,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub set_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub supports_tools: bool,
    pub supports_vision: bool,
    #[serde(default)]
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    #[serde(flatten)]
    pub config: ModelConfig,
    pub provider_name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePolicy {
    pub enabled: bool,
    pub reason: String,
}

impl Default for UpdatePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            reason: "In-app updates stay disabled until release signatures and private-key custody are configured.".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub theme: String,
    pub ui_language: String,
    pub text_mode: String,
    pub font_scale: f32,
    pub default_model_id: Option<String>,
    pub project_path: Option<String>,
    pub recent_projects: Vec<String>,
    pub providers: Vec<ProviderConfig>,
    pub models: Vec<ModelConfig>,
    pub desktop_tools_enabled: bool,
    pub web_access_enabled: bool,
    pub max_tool_iterations: u8,
    pub max_retries: u8,
    pub terminal_visible: bool,
    pub updates: UpdatePolicy,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: "ember-dark".into(),
            ui_language: "fa".into(),
            text_mode: "native".into(),
            font_scale: 1.0,
            default_model_id: None,
            project_path: None,
            recent_projects: Vec::new(),
            providers: Vec::new(),
            models: Vec::new(),
            desktop_tools_enabled: false,
            web_access_enabled: true,
            max_tool_iterations: 8,
            max_retries: 2,
            terminal_visible: false,
            updates: UpdatePolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub theme: String,
    pub ui_language: String,
    pub text_mode: String,
    pub font_scale: f32,
    pub desktop_tools_enabled: bool,
    pub web_access_enabled: bool,
    pub max_tool_iterations: u8,
    pub max_retries: u8,
    pub terminal_visible: bool,
    pub default_model_id: Option<String>,
}

impl From<&Settings> for Preferences {
    fn from(value: &Settings) -> Self {
        Self {
            theme: value.theme.clone(),
            ui_language: value.ui_language.clone(),
            text_mode: value.text_mode.clone(),
            font_scale: value.font_scale,
            desktop_tools_enabled: value.desktop_tools_enabled,
            web_access_enabled: value.web_access_enabled,
            max_tool_iterations: value.max_tool_iterations,
            max_retries: value.max_retries,
            terminal_visible: value.terminal_visible,
            default_model_id: value.default_model_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesPatch {
    pub theme: String,
    pub ui_language: String,
    pub text_mode: String,
    pub font_scale: f32,
    pub desktop_tools_enabled: bool,
    pub web_access_enabled: bool,
    pub max_tool_iterations: u8,
    pub max_retries: u8,
    pub terminal_visible: bool,
    pub default_model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub config_dir: String,
    pub data_dir: String,
    pub secret_backend: String,
    pub update_check_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub preferences: Preferences,
    pub providers: Vec<ProviderView>,
    pub models: Vec<ModelView>,
    pub project_path: Option<String>,
    pub recent_projects: Vec<String>,
    pub updates: UpdatePolicy,
    pub app: AppInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: SettingsView,
    pub conversations: Vec<ConversationSummary>,
    pub presets: Vec<ProviderPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub project_path: Option<String>,
    pub model_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentView {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub mime: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallView {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageView {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentView>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallView>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolEventView {
    pub id: String,
    pub conversation_id: String,
    pub kind: String,
    pub name: String,
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub details: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeView {
    pub id: String,
    pub conversation_id: String,
    pub path: String,
    pub before: String,
    pub after: String,
    pub diff: String,
    pub existed_before: bool,
    pub after_hash: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: String,
    pub conversation_id: String,
    pub tool_name: String,
    pub title: String,
    pub details: String,
    pub risk: String,
    pub target: String,
    #[serde(default)]
    pub diff: Option<String>,
    pub allow_session: bool,
    pub session_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalDecision {
    Once,
    Session,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLine {
    pub stream: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Status {
        conversation_id: String,
        status: String,
        message: String,
    },
    MessageDelta {
        conversation_id: String,
        message_id: String,
        delta: String,
    },
    MessageDone {
        message: MessageView,
    },
    ToolCall {
        event: ToolEventView,
        tool_call: ToolCallView,
    },
    ToolResult {
        event: ToolEventView,
    },
    ToolEvent {
        event: ToolEventView,
    },
    ApprovalRequest {
        request: ApprovalRequest,
    },
    ApprovalResolved {
        request_id: String,
        decision: ApprovalDecision,
        from_session: bool,
    },
    FileChange {
        change: FileChangeView,
    },
    Terminal {
        conversation_id: String,
        lines: Vec<TerminalLine>,
    },
    Error {
        conversation_id: String,
        message: String,
        retryable: bool,
    },
    Done {
        conversation_id: String,
        message_id: String,
    },
    Stopped {
        conversation_id: String,
        message_id: String,
    },
    Connection {
        provider_id: String,
        state: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageInput {
    #[serde(default)]
    pub conversation_id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryInput {
    pub conversation_id: String,
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStart {
    pub run_id: String,
    pub conversation_id: String,
    pub user_message: Option<MessageView>,
    pub assistant_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationData {
    pub conversation: ConversationSummary,
    pub messages: Vec<MessageView>,
    pub tool_events: Vec<ToolEventView>,
    pub file_changes: Vec<FileChangeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmokeReport {
    pub ok: bool,
    pub version: String,
    pub config_dir: String,
    pub database: String,
    pub migration_version: i64,
    pub secret_backend: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamOutcome {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub stopped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmMessage {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
