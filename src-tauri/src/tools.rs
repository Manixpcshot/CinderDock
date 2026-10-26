use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use base64::Engine;
use regex::Regex;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    sync::{oneshot::error::TryRecvError, Notify},
};
use walkdir::WalkDir;

use crate::{
    app_state::AppState,
    approvals::ApprovalStart,
    db,
    diff::{compact_summary, unified_diff, MAX_DIFF_CHARS},
    error::{AppError, AppResult},
    events::EventBus,
    models::{
        AgentEvent, ApprovalDecision, ApprovalRequest, FileChangeView, Settings, TerminalLine,
        ToolCall, ToolEventView,
    },
    providers,
    security::{
        atomic_write, command_risk, hash_text, is_sensitive_path, new_id, now_rfc3339,
        project_relative_path, safe_read_text, should_skip_dir, split_terminal_lines,
        truncate_chars, validate_public_http_url, MAX_FILE_BYTES, MAX_IMAGE_BYTES,
        MAX_OUTPUT_CHARS, MAX_WEB_BYTES,
    },
};

#[derive(Debug, Clone)]
pub struct ToolExecution {
    pub output: String,
    pub truncated: bool,
    pub denied: bool,
    pub is_error: bool,
    pub images: Vec<String>,
}

impl ToolExecution {
    fn ok(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            truncated: false,
            denied: false,
            is_error: false,
            images: Vec::new(),
        }
    }

    fn error(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            truncated: false,
            denied: false,
            is_error: true,
            images: Vec::new(),
        }
    }

    fn denied() -> Self {
        Self {
            output: "کاربر مجوز این عملیات را رد کرد. ابزار اجرا نشد و هیچ تغییری اعمال نشد."
                .into(),
            truncated: false,
            denied: true,
            is_error: true,
            images: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub state: Arc<AppState>,
    pub bus: EventBus,
    pub conversation_id: String,
    pub project_root: Option<PathBuf>,
    pub settings: Settings,
    pub stop: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct ShellOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct ToolRunner {
    ctx: ToolContext,
}

pub fn definitions(settings: &Settings, has_project: bool) -> Value {
    let mut tools: Vec<Value> = Vec::new();
    if has_project {
        tools.extend([
            tool_def(
                "list_files",
                "List text files inside the selected project, excluding sensitive paths.",
                json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Optional relative subfolder." },
                        "maxDepth": { "type": "integer", "minimum": 1, "maximum": 6 }
                    },
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "read_file",
                "Read a UTF-8 text file from the project. Sensitive files and binary files are blocked.",
                json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "search_project",
                "Search text inside project files and return matching lines with line numbers.",
                json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "path": { "type": "string" },
                        "maxResults": { "type": "integer", "minimum": 1, "maximum": 100 }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "write_file",
                "Create or replace a text file in the project. The user must approve a unified diff first.",
                json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" },
                        "mode": { "type": "string", "enum": ["create", "replace", "append"] }
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "run_command",
                "Run a shell command in the project directory. The user must approve the exact command first.",
                json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timeoutMs": { "type": "integer", "minimum": 1000, "maximum": 120000 }
                    },
                    "required": ["command"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "git_status",
                "Read git status --short from the selected project.",
                json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            ),
            tool_def(
                "git_diff",
                "Read a unified git diff from the selected project.",
                json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "staged": { "type": "boolean" }
                    },
                    "additionalProperties": false
                }),
            ),
        ]);
    }
    if settings.web_access_enabled {
        tools.extend([
            tool_def(
                "web_search",
                "Search the public web with DuckDuckGo HTML and return result titles and URLs.",
                json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "maxResults": { "type": "integer", "minimum": 1, "maximum": 8 }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "fetch_webpage",
                "Fetch a public HTTP(S) page and return its readable text. Private network hosts are blocked.",
                json!({
                    "type": "object",
                    "properties": { "url": { "type": "string" } },
                    "required": ["url"],
                    "additionalProperties": false
                }),
            ),
        ]);
    }
    if settings.desktop_tools_enabled && cfg!(target_os = "windows") {
        tools.extend([
            tool_def(
                "desktop_screenshot",
                "Capture the primary Windows screen. Each use needs explicit approval; the image may be sent to the vision model.",
                json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            ),
            tool_def(
                "desktop_click",
                "Move the Windows cursor and click. Each use needs explicit approval.",
                json!({
                    "type": "object",
                    "properties": {
                        "x": { "type": "integer" },
                        "y": { "type": "integer" },
                        "button": { "type": "string", "enum": ["left", "right"] }
                    },
                    "required": ["x", "y"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "desktop_type",
                "Type text into the active Windows window. Each use needs explicit approval.",
                json!({
                    "type": "object",
                    "properties": { "text": { "type": "string", "maxLength": 2000 } },
                    "required": ["text"],
                    "additionalProperties": false
                }),
            ),
            tool_def(
                "desktop_scroll",
                "Scroll the mouse wheel on Windows. Each use needs explicit approval.",
                json!({
                    "type": "object",
                    "properties": {
                        "x": { "type": "integer" },
                        "y": { "type": "integer" },
                        "amount": { "type": "integer", "minimum": -1200, "maximum": 1200 }
                    },
                    "required": ["x", "y", "amount"],
                    "additionalProperties": false
                }),
            ),
        ]);
    }
    Value::Array(tools)
}

fn tool_def(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

impl ToolRunner {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }

    fn root(&self) -> AppResult<&Path> {
        self.ctx
            .project_root
            .as_deref()
            .ok_or_else(|| AppError::Unavailable("ابتدا پوشهٔ پروژه را انتخاب کنید.".into()))
    }

    pub async fn execute(&self, call: ToolCall) -> AppResult<ToolExecution> {
        let args: Value = if call.arguments.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&call.arguments)
                .map_err(|e| AppError::Validation(format!("آرگومان ابزار نامعتبر است: {e}")))?
        };
        self.emit_tool_call(&call).await?;
        let mut result = match self.dispatch(&call.name, &args).await {
            Ok(result) => result,
            Err(error) => ToolExecution::error(format!("خطای ابزار {}: {}", call.name, error)),
        };
        result.output = self.ctx.bus.redact(&result.output);
        let (output, truncated) = truncate_chars(&result.output, MAX_OUTPUT_CHARS);
        result.output = output;
        result.truncated |= truncated;
        self.emit_tool_result(&call, &result).await?;
        Ok(result)
    }

    async fn emit_tool_call(&self, call: &ToolCall) -> AppResult<()> {
        let (details, truncated) = truncate_chars(&call.arguments, 20_000);
        let event = ToolEventView {
            id: new_id("event"),
            conversation_id: self.ctx.conversation_id.clone(),
            kind: "tool_call".into(),
            name: call.name.clone(),
            status: "running".into(),
            summary: format!("درخواست اجرای {}", call.name),
            details: if truncated {
                format!("{details}\n…")
            } else {
                details
            },
            created_at: now_rfc3339(),
        };
        self.record_event(event.clone()).await?;
        self.ctx.bus.emit(AgentEvent::ToolCall {
            event,
            tool_call: crate::models::ToolCallView {
                id: call.id.clone(),
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            },
        })
    }

    async fn emit_tool_result(&self, call: &ToolCall, result: &ToolExecution) -> AppResult<()> {
        let status = if result.denied {
            "denied"
        } else if result.is_error {
            "error"
        } else {
            "success"
        };
        let (details, _) = truncate_chars(&result.output, 30_000);
        let event = ToolEventView {
            id: new_id("event"),
            conversation_id: self.ctx.conversation_id.clone(),
            kind: "tool_result".into(),
            name: call.name.clone(),
            status: status.into(),
            summary: if result.denied {
                "مجوز رد شد؛ ابزار اجرا نشد".into()
            } else if result.is_error {
                "ابزار با خطا مواجه شد".into()
            } else {
                "نتیجهٔ واقعی ابزار دریافت شد".into()
            },
            details,
            created_at: now_rfc3339(),
        };
        self.record_event(event.clone()).await?;
        self.ctx.bus.emit(AgentEvent::ToolResult { event })
    }

    async fn record_event(&self, mut event: ToolEventView) -> AppResult<()> {
        event.details = self.ctx.bus.redact(&event.details);
        {
            let conn = self.ctx.state.db.lock().await;
            db::insert_tool_event(&conn, &event)?;
        }
        self.ctx.bus.emit(AgentEvent::ToolEvent { event })
    }

    async fn emit_terminal(&self, lines: Vec<TerminalLine>) -> AppResult<()> {
        if lines.is_empty() {
            return Ok(());
        }
        self.ctx.bus.emit(AgentEvent::Terminal {
            conversation_id: self.ctx.conversation_id.clone(),
            lines,
        })
    }

    async fn dispatch(&self, name: &str, args: &Value) -> AppResult<ToolExecution> {
        match name {
            "list_files" => self.list_files(args).await,
            "read_file" => self.read_file(args).await,
            "search_project" => self.search_project(args).await,
            "write_file" => self.write_file(args).await,
            "run_command" => self.run_command_tool(args).await,
            "git_status" => self.git_status().await,
            "git_diff" => self.git_diff(args).await,
            "web_search" => self.web_search(args).await,
            "fetch_webpage" => self.fetch_webpage(args).await,
            "desktop_screenshot" => self.desktop_screenshot().await,
            "desktop_click" => self.desktop_click(args).await,
            "desktop_type" => self.desktop_type(args).await,
            "desktop_scroll" => self.desktop_scroll(args).await,
            other => Err(AppError::Validation(format!("ابزار ناشناخته: {other}"))),
        }
    }

    async fn list_files(&self, args: &Value) -> AppResult<ToolExecution> {
        let root = self.root()?;
        let relative = string_arg(args, "path", "");
        let start = if relative.trim().is_empty() {
            root.to_path_buf()
        } else {
            project_relative_path(root, &relative)?
        };
        let depth = number_arg(args, "maxDepth", 3).clamp(1, 6) as usize;
        let mut files = Vec::new();
        for entry in WalkDir::new(&start)
            .max_depth(depth)
            .into_iter()
            .filter_entry(|entry| {
                entry.file_type().is_file()
                    || !should_skip_dir(&entry.file_name().to_string_lossy())
            })
        {
            let entry =
                entry.map_err(|e| AppError::Internal(format!("cannot walk project: {e}")))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
            if is_sensitive_path(relative) {
                continue;
            }
            files.push(format!(
                "[{}] {}",
                if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    "file"
                } else {
                    "file"
                },
                relative.display()
            ));
            if files.len() >= 500 {
                files.push("… [CinderDock: file list truncated at 500 items]".into());
                break;
            }
        }
        if files.is_empty() {
            Ok(ToolExecution::ok(
                "هیچ فایل متنی قابل نمایشی در این مسیر پیدا نشد.",
            ))
        } else {
            Ok(ToolExecution::ok(files.join("\n")))
        }
    }

    async fn read_file(&self, args: &Value) -> AppResult<ToolExecution> {
        let path = string_arg(args, "path", "");
        let resolved = project_relative_path(self.root()?, &path)?;
        let content = safe_read_text(&resolved, MAX_FILE_BYTES)?;
        let (content, truncated) = truncate_chars(&content, MAX_OUTPUT_CHARS);
        Ok(ToolExecution {
            truncated,
            ..ToolExecution::ok(format!("FILE: {path}\n{content}"))
        })
    }

    async fn search_project(&self, args: &Value) -> AppResult<ToolExecution> {
        let query = string_arg(args, "query", "");
        if query.trim().is_empty() {
            return Err(AppError::Validation("عبارت جستوجو خالی است.".into()));
        }
        let root = self.root()?;
        let subpath = string_arg(args, "path", "");
        let start = if subpath.trim().is_empty() {
            root.to_path_buf()
        } else {
            project_relative_path(root, &subpath)?
        };
        let max_results = number_arg(args, "maxResults", 30).clamp(1, 100) as usize;
        let needle = query.to_lowercase();
        let mut results = Vec::new();
        for entry in WalkDir::new(&start)
            .max_depth(8)
            .into_iter()
            .filter_entry(|entry| {
                entry.file_type().is_file()
                    || !should_skip_dir(&entry.file_name().to_string_lossy())
            })
        {
            let entry =
                entry.map_err(|e| AppError::Internal(format!("cannot walk project: {e}")))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
            if is_sensitive_path(relative)
                || entry.metadata().map(|m| m.len() > 512_000).unwrap_or(false)
            {
                continue;
            }
            let Ok(content) = safe_read_text(entry.path(), 512_000) else {
                continue;
            };
            for (index, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    results.push(format!(
                        "{}:{}: {}",
                        relative.display(),
                        index + 1,
                        line.trim()
                    ));
                    if results.len() >= max_results {
                        break;
                    }
                }
            }
            if results.len() >= max_results {
                break;
            }
        }
        if results.is_empty() {
            Ok(ToolExecution::ok("نتیجهای پیدا نشد."))
        } else {
            Ok(ToolExecution::ok(results.join("\n")))
        }
    }

    async fn write_file(&self, args: &Value) -> AppResult<ToolExecution> {
        let relative = string_arg(args, "path", "");
        let content = string_arg(args, "content", "");
        let mode = string_arg(args, "mode", "replace");
        if !matches!(mode.as_str(), "create" | "replace" | "append") {
            return Err(AppError::Validation("حالت نوشتن فایل نامعتبر است.".into()));
        }
        if content.len() > MAX_DIFF_CHARS {
            return Err(AppError::Safety(
                "محتوای پیشنهادی از سقف قابل نمایش diff بزرگتر است.".into(),
            ));
        }
        let resolved = project_relative_path(self.root()?, &relative)?;
        let existed_before = resolved.exists();
        if mode == "create" && existed_before {
            return Err(AppError::Safety(
                "فایل از قبل وجود دارد؛ از حالت replace استفاده کنید.".into(),
            ));
        }
        let before = if existed_before {
            safe_read_text(&resolved, MAX_DIFF_CHARS)?
        } else {
            String::new()
        };
        if before.len() > MAX_DIFF_CHARS {
            return Err(AppError::Safety(
                "فایل مقصد برای نمایش diff کامل بیش از حد بزرگ است.".into(),
            ));
        }
        let after = match mode.as_str() {
            "append" => format!("{before}{content}"),
            _ => content.clone(),
        };
        if after.len() > MAX_DIFF_CHARS {
            return Err(AppError::Safety(
                "نتیجهٔ نهایی فایل از سقف قابل بازگردانی بزرگتر است.".into(),
            ));
        }
        let (before, after, diff) = unified_diff(&relative, &before, &after, 3);
        let (added, removed) = compact_summary(&diff);
        let decision = self
            .ask_approval(
                "write_file",
                "اعمال تغییر فایل",
                format!(
                    "مدل میخواهد فایل «{relative}» را با حالت «{mode}» تغییر دهد. {added} خط اضافه و {removed} خط حذف میشود. diff زیر پیش از اعمال نشان داده شده است."
                ),
                "high",
                relative.clone(),
                Some(diff.clone()),
                true,
            )
            .await?;
        if decision == ApprovalDecision::Deny {
            return Ok(ToolExecution::denied());
        }
        atomic_write(&resolved, &after)?;
        let after_hash = hash_text(&after);
        let change = FileChangeView {
            id: new_id("change"),
            conversation_id: self.ctx.conversation_id.clone(),
            path: relative.clone(),
            before,
            after,
            diff,
            existed_before,
            after_hash,
            status: "applied".into(),
            created_at: now_rfc3339(),
        };
        {
            let conn = self.ctx.state.db.lock().await;
            db::insert_file_change(&conn, &change)?;
        }
        self.ctx.bus.emit(AgentEvent::FileChange { change })?;
        Ok(ToolExecution::ok(format!(
            "فایل «{relative}» پس از تأیید کاربر نوشته شد. {added} خط اضافه و {removed} خط حذف شد."
        )))
    }

    async fn run_command_tool(&self, args: &Value) -> AppResult<ToolExecution> {
        let command = string_arg(args, "command", "");
        if command.trim().is_empty() || command.len() > 8000 {
            return Err(AppError::Safety(
                "فرمان خالی یا بیش از حد طولانی است.".into(),
            ));
        }
        let timeout_ms = number_arg(args, "timeoutMs", 30_000).clamp(1000, 120_000) as u64;
        let root = self.root()?.to_path_buf();
        let risk = command_risk(&command);
        let decision = self
            .ask_approval(
                "run_command",
                "اجرای فرمان",
                format!(
                    "فرمان زیر در پوشهٔ پروژه اجرا میشود. سقف زمان {timeout_ms} میلیثانیه و سقف خروجی {MAX_OUTPUT_CHARS} نویسه است.\n\n{command}"
                ),
                risk,
                command.clone(),
                None,
                risk != "critical",
            )
            .await?;
        if decision == ApprovalDecision::Deny {
            return Ok(ToolExecution::denied());
        }
        let outcome = run_shell(
            &command,
            &root,
            Duration::from_millis(timeout_ms),
            MAX_OUTPUT_CHARS,
        )
        .await?;
        let (stdout_lines, stderr_lines) = split_terminal_lines(&outcome.stdout, &outcome.stderr);
        let mut terminal_lines = Vec::new();
        terminal_lines.extend(stdout_lines.into_iter().map(|line| TerminalLine {
            stream: "stdout".into(),
            line,
        }));
        terminal_lines.extend(stderr_lines.into_iter().map(|line| TerminalLine {
            stream: "stderr".into(),
            line,
        }));
        self.emit_terminal(terminal_lines).await?;
        let mut output = String::new();
        output.push_str(&format!(
            "exit_code: {}\n",
            outcome
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "killed".into())
        ));
        if outcome.timed_out {
            output.push_str("command timed out and was killed\n");
        }
        if !outcome.stdout.trim().is_empty() {
            output.push_str("--- stdout ---\n");
            output.push_str(&outcome.stdout);
            output.push('\n');
        }
        if !outcome.stderr.trim().is_empty() {
            output.push_str("--- stderr ---\n");
            output.push_str(&outcome.stderr);
            output.push('\n');
        }
        Ok(ToolExecution {
            output,
            truncated: outcome.truncated,
            denied: false,
            is_error: outcome.timed_out || outcome.exit_code.map(|code| code != 0).unwrap_or(true),
            images: Vec::new(),
        })
    }

    async fn git_status(&self) -> AppResult<ToolExecution> {
        let root = self.root()?.to_path_buf();
        let outcome = run_program(
            "git",
            &["status", "--short"],
            &root,
            &[],
            Duration::from_secs(20),
            MAX_OUTPUT_CHARS,
        )
        .await?;
        self.emit_terminal_from_outcome(&outcome).await?;
        Ok(git_result("git status --short", outcome))
    }

    async fn git_diff(&self, args: &Value) -> AppResult<ToolExecution> {
        let root = self.root()?.to_path_buf();
        let staged = bool_arg(args, "staged", false);
        let path = string_arg(args, "path", "");
        let mut owned_args: Vec<String> = vec!["diff".into()];
        if staged {
            owned_args.push("--cached".into());
        }
        if !path.trim().is_empty() {
            project_relative_path(&root, &path)?;
            owned_args.push("--".into());
            owned_args.push(path);
        }
        let arg_refs: Vec<&str> = owned_args.iter().map(String::as_str).collect();
        let outcome = run_program(
            "git",
            &arg_refs,
            &root,
            &[],
            Duration::from_secs(25),
            MAX_OUTPUT_CHARS,
        )
        .await?;
        self.emit_terminal_from_outcome(&outcome).await?;
        Ok(git_result("git diff", outcome))
    }

    async fn emit_terminal_from_outcome(&self, outcome: &ShellOutcome) -> AppResult<()> {
        let (stdout, stderr) = split_terminal_lines(&outcome.stdout, &outcome.stderr);
        let mut lines = Vec::new();
        lines.extend(stdout.into_iter().map(|line| TerminalLine {
            stream: "stdout".into(),
            line,
        }));
        lines.extend(stderr.into_iter().map(|line| TerminalLine {
            stream: "stderr".into(),
            line,
        }));
        self.emit_terminal(lines).await
    }

    async fn web_search(&self, args: &Value) -> AppResult<ToolExecution> {
        if !self.ctx.settings.web_access_enabled {
            return Err(AppError::Unavailable(
                "دسترسی وب در تنظیمات خاموش است.".into(),
            ));
        }
        let query = string_arg(args, "query", "");
        if query.trim().is_empty() {
            return Err(AppError::Validation("عبارت جستوجو خالی است.".into()));
        }
        let max_results = number_arg(args, "maxResults", 5).clamp(1, 8) as usize;
        let url = url::Url::parse_with_params(
            "https://html.duckduckgo.com/html/",
            &[("q", query.as_str())],
        )?;
        let mut response = self.ctx.state.client.get(url).send().await?;
        let status = response.status();
        let body = providers::read_limited_body(&mut response, MAX_WEB_BYTES).await?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "جستوجوی وب با HTTP {} مواجه شد.",
                status.as_u16()
            )));
        }
        let re = Regex::new(
            r#"(?is)<a[^>]+class="[^"]*result__a[^"]*"[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#,
        )?;
        let mut results = Vec::new();
        for caps in re.captures_iter(&body) {
            let raw_url = html_decode(&caps[1]);
            let resolved = resolve_ddg_url(&raw_url);
            let title = strip_html(&caps[2]);
            if title.trim().is_empty() || resolved.is_empty() {
                continue;
            }
            results.push(format!("{}. {}\n   {}", results.len() + 1, title, resolved));
            if results.len() >= max_results {
                break;
            }
        }
        if results.is_empty() {
            Ok(ToolExecution::ok(
                "نتیجهای از موتور جستوجو دریافت نشد. عبارت را تغییر دهید یا بعداً تلاش کنید.",
            ))
        } else {
            Ok(ToolExecution::ok(results.join("\n")))
        }
    }

    async fn fetch_webpage(&self, args: &Value) -> AppResult<ToolExecution> {
        if !self.ctx.settings.web_access_enabled {
            return Err(AppError::Unavailable(
                "دسترسی وب در تنظیمات خاموش است.".into(),
            ));
        }
        let raw_url = string_arg(args, "url", "");
        let url = validate_public_http_url(&raw_url)?;
        let mut response = self.ctx.state.client.get(url.clone()).send().await?;
        let status = response.status();
        let body = providers::read_limited_body(&mut response, MAX_WEB_BYTES).await?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "دریافت صفحه با HTTP {} مواجه شد.",
                status.as_u16()
            )));
        }
        let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>")?;
        let title = title_re
            .captures(&body)
            .map(|caps| strip_html(&caps[1]))
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| url.to_string());
        let text = strip_html(&body);
        let (text, truncated) = truncate_chars(&text, MAX_OUTPUT_CHARS);
        Ok(ToolExecution {
            truncated,
            ..ToolExecution::ok(format!("URL: {}\nTITLE: {}\n\n{}", url, title, text))
        })
    }

    async fn desktop_screenshot(&self) -> AppResult<ToolExecution> {
        self.desktop_guard().await?;
        let approved = self.ask_desktop_approval(
            "desktop_screenshot",
            "گرفتن تصویر صفحه",
            "تصویر کل صفحهٔ اصلی ویندوز گرفته میشود. ممکن است اطلاعات حساس روی صفحه دیده شود. اگر مدل انتخابشده توانایی دیدن تصویر داشته باشد، تصویر به همان پروایدر ارسال خواهد شد. فایل تصویر پس از خواندن حذف میشود.",
            "high",
            "primary screen",
        )
        .await?;
        if !approved {
            return Ok(ToolExecution::denied());
        }
        if !cfg!(target_os = "windows") {
            return Err(AppError::Unavailable(
                "ابزارهای کنترل دسکتاپ فقط روی ویندوز در دسترساند.".into(),
            ));
        }
        let path =
            std::env::temp_dir().join(format!("cinderdock-shot-{}.png", uuid::Uuid::new_v4()));
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bmp.Save($env:CINDERDOCK_SHOT_PATH, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bmp.Dispose()
Write-Output "$($bounds.Width)x$($bounds.Height)"
"#;
        let outcome = run_program(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                script,
            ],
            &std::env::temp_dir(),
            &[("CINDERDOCK_SHOT_PATH", path.to_string_lossy().as_ref())],
            Duration::from_secs(20),
            16_000,
        )
        .await?;
        if !outcome.stdout.contains('x') || outcome.exit_code.unwrap_or(1) != 0 {
            let _ = std::fs::remove_file(&path);
            return Err(AppError::Internal(format!(
                "screenshot failed: {} {}",
                outcome.stdout, outcome.stderr
            )));
        }
        let bytes = std::fs::read(&path)?;
        let _ = std::fs::remove_file(&path);
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(AppError::Safety(
                "تصویر صفحه از سقف مجاز ارسال به مدل بزرگتر است.".into(),
            ));
        }
        let data_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        );
        Ok(ToolExecution {
            images: vec![data_url],
            ..ToolExecution::ok(format!(
                "تصویر صفحه گرفته شد ({}) و فقط در حافظه برای ارسال به مدل نگه داشته میشود.",
                outcome.stdout.trim()
            ))
        })
    }

    async fn desktop_click(&self, args: &Value) -> AppResult<ToolExecution> {
        self.desktop_guard().await?;
        let x = number_arg(args, "x", 0).clamp(-10000, 10000);
        let y = number_arg(args, "y", 0).clamp(-10000, 10000);
        let button = string_arg(args, "button", "left");
        if !matches!(button.as_str(), "left" | "right") {
            return Err(AppError::Validation("دکمه ماوس نامعتبر است.".into()));
        }
        let approved = self.ask_desktop_approval(
            "desktop_click",
            "کلیک روی صفحه",
            format!("مکاننما به ({x}, {y}) منتقل و دکمهٔ {button} کلیک میشود. این عملیات روی پنجرهٔ فعال اثر میگذارد."),
            "high",
            &format!("{x},{y},{button}"),
        )
        .await?;
        if !approved {
            return Ok(ToolExecution::denied());
        }
        let down = if button == "left" { "0x0002" } else { "0x0008" };
        let up = if button == "left" { "0x0004" } else { "0x0010" };
        let script = format!(
            r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class CinderMouse {{
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}}
'@
[CinderMouse]::SetCursorPos({x}, {y}) | Out-Null
[CinderMouse]::mouse_event({down}, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 60
[CinderMouse]::mouse_event({up}, 0, 0, 0, [UIntPtr]::Zero)
Write-Output "clicked"
"#
        );
        let outcome = run_program(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &script,
            ],
            &std::env::temp_dir(),
            &[],
            Duration::from_secs(15),
            16_000,
        )
        .await?;
        Ok(shell_result("desktop_click", outcome))
    }

    async fn desktop_type(&self, args: &Value) -> AppResult<ToolExecution> {
        self.desktop_guard().await?;
        let text = string_arg(args, "text", "");
        if text.is_empty() || text.chars().count() > 2000 {
            return Err(AppError::Safety(
                "متن تایپ خالی یا بیش از حد طولانی است.".into(),
            ));
        }
        let approved = self.ask_desktop_approval(
            "desktop_type",
            "تایپ در پنجرهٔ فعال",
            format!("متن زیر با SendKeys در پنجرهٔ فعال تایپ میشود. لطفاً محل ورودی درست را پیش از تأیید انتخاب کنید.\n\n{}", truncate_chars(&text, 300).0),
            "high",
            &format!("{} characters", text.chars().count()),
        )
        .await?;
        if !approved {
            return Ok(ToolExecution::denied());
        }
        let escaped = escape_sendkeys(&text);
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait($env:CINDERDOCK_DESKTOP_TEXT)
Write-Output "typed"
"#;
        let outcome = run_program(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                script,
            ],
            &std::env::temp_dir(),
            &[("CINDERDOCK_DESKTOP_TEXT", escaped.as_str())],
            Duration::from_secs(15),
            16_000,
        )
        .await?;
        Ok(shell_result("desktop_type", outcome))
    }

    async fn desktop_scroll(&self, args: &Value) -> AppResult<ToolExecution> {
        self.desktop_guard().await?;
        let x = number_arg(args, "x", 0).clamp(-10000, 10000);
        let y = number_arg(args, "y", 0).clamp(-10000, 10000);
        let amount = number_arg(args, "amount", 0).clamp(-1200, 1200);
        let approved = self
            .ask_desktop_approval(
                "desktop_scroll",
                "اسکرول صفحه",
                format!("مکاننما به ({x}, {y}) منتقل و به اندازهٔ {amount} اسکرول میشود."),
                "high",
                &format!("{x},{y},{amount}"),
            )
            .await?;
        if !approved {
            return Ok(ToolExecution::denied());
        }
        let script = format!(
            r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class CinderScroll {{
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}}
'@
[CinderScroll]::SetCursorPos({x}, {y}) | Out-Null
$delta = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int32]({amount})), 0)
[CinderScroll]::mouse_event(0x0800, 0, 0, $delta, [UIntPtr]::Zero)
Write-Output "scrolled"
"#
        );
        let outcome = run_program(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &script,
            ],
            &std::env::temp_dir(),
            &[],
            Duration::from_secs(15),
            16_000,
        )
        .await?;
        Ok(shell_result("desktop_scroll", outcome))
    }

    async fn desktop_guard(&self) -> AppResult<()> {
        if !cfg!(target_os = "windows") || !self.ctx.settings.desktop_tools_enabled {
            return Err(AppError::Unavailable(
                "ابزارهای کنترل دسکتاپ خاموشاند یا فقط روی ویندوز در دسترساند.".into(),
            ));
        }
        Ok(())
    }

    async fn ask_desktop_approval(
        &self,
        tool: &str,
        title: &str,
        details: impl Into<String>,
        risk: &str,
        target: &str,
    ) -> AppResult<bool> {
        let decision = self
            .ask_approval(tool, title, details, risk, target, None, false)
            .await?;
        Ok(decision != ApprovalDecision::Deny)
    }

    async fn ask_approval(
        &self,
        tool_name: &str,
        title: &str,
        details: impl Into<String>,
        risk: &str,
        target: impl Into<String>,
        diff: Option<String>,
        allow_session: bool,
    ) -> AppResult<ApprovalDecision> {
        let scope = self
            .ctx
            .project_root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "no-project".into());
        let session_key = if allow_session {
            format!("session:{scope}:{tool_name}")
        } else {
            format!("once:{scope}:{tool_name}")
        };
        let request = ApprovalRequest {
            id: String::new(),
            conversation_id: self.ctx.conversation_id.clone(),
            tool_name: tool_name.into(),
            title: title.into(),
            details: details.into(),
            risk: risk.into(),
            target: target.into(),
            diff,
            allow_session,
            session_key,
            created_at: now_rfc3339(),
        };
        let start = self.ctx.state.approvals.begin(request)?;
        let (request, receiver, from_session) = match start {
            ApprovalStart::Granted {
                request,
                from_session,
            } => {
                self.record_approval_event(
                    &request,
                    "auto_session",
                    "تأیید خودکار برای this session",
                )
                .await?;
                self.ctx.bus.emit(AgentEvent::ApprovalResolved {
                    request_id: request.id.clone(),
                    decision: ApprovalDecision::Once,
                    from_session,
                })?;
                return Ok(ApprovalDecision::Once);
            }
            ApprovalStart::Pending { request, receiver } => (request, Some(receiver)),
        };
        self.record_approval_event(&request, "pending", "در انتظار تأیید کاربر")
            .await?;
        self.ctx.bus.emit(AgentEvent::ApprovalRequest {
            request: request.clone(),
        })?;
        let mut receiver = receiver.expect("pending approval must have receiver");
        let started = Instant::now();
        let decision = loop {
            if self.ctx.stop.load(Ordering::SeqCst) {
                self.ctx.state.approvals.cancel(&request.id);
                break ApprovalDecision::Deny;
            }
            match receiver.try_recv() {
                Ok(decision) => break decision,
                Err(TryRecvError::Empty) => {
                    if started.elapsed() > Duration::from_secs(300) {
                        self.ctx.state.approvals.cancel(&request.id);
                        break ApprovalDecision::Deny;
                    }
                    tokio::time::sleep(Duration::from_millis(180)).await;
                }
                Err(TryRecvError::Closed) => break ApprovalDecision::Deny,
            }
        };
        let label = match decision {
            ApprovalDecision::Once => "once",
            ApprovalDecision::Session => "session",
            ApprovalDecision::Deny => "denied",
        };
        self.record_approval_event(&request, label, "تصمیم کاربر ثبت شد")
            .await?;
        self.ctx.bus.emit(AgentEvent::ApprovalResolved {
            request_id: request.id.clone(),
            decision,
            from_session: false,
        })?;
        {
            let conn = self.ctx.state.db.lock().await;
            db::insert_approval_decision(
                &conn,
                &request.id,
                &request.conversation_id,
                tool_name,
                decision,
            )?;
        }
        Ok(decision)
    }

    async fn record_approval_event(
        &self,
        request: &ApprovalRequest,
        status: &str,
        summary: &str,
    ) -> AppResult<()> {
        let event = ToolEventView {
            id: new_id("event"),
            conversation_id: request.conversation_id.clone(),
            kind: "approval".into(),
            name: request.tool_name.clone(),
            status: status.into(),
            summary: summary.into(),
            details: request.details.clone(),
            created_at: now_rfc3339(),
        };
        self.record_event(event).await
    }
}

fn string_arg(args: &Value, key: &str, default: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn number_arg(args: &Value, key: &str, default: i64) -> i64 {
    args.get(key).and_then(Value::as_i64).unwrap_or(default)
}

fn bool_arg(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn git_result(label: &str, outcome: ShellOutcome) -> ToolExecution {
    let mut output = format!("{label}\nexit_code: {}\n", outcome.exit_code.unwrap_or(-1));
    if !outcome.stdout.trim().is_empty() {
        output.push_str("--- stdout ---\n");
        output.push_str(&outcome.stdout);
        output.push('\n');
    }
    if !outcome.stderr.trim().is_empty() {
        output.push_str("--- stderr ---\n");
        output.push_str(&outcome.stderr);
        output.push('\n');
    }
    ToolExecution {
        output,
        truncated: outcome.truncated,
        denied: false,
        is_error: outcome.timed_out || outcome.exit_code.map(|code| code != 0).unwrap_or(true),
        images: Vec::new(),
    }
}

fn shell_result(label: &str, outcome: ShellOutcome) -> ToolExecution {
    let mut output = format!("{label}\n{}", outcome.stdout.trim());
    if !outcome.stderr.trim().is_empty() {
        output.push_str("\nstderr:\n");
        output.push_str(&outcome.stderr);
    }
    ToolExecution {
        output,
        truncated: outcome.truncated,
        denied: false,
        is_error: outcome.timed_out || outcome.exit_code.map(|code| code != 0).unwrap_or(true),
        images: Vec::new(),
    }
}

async fn run_shell(
    command: &str,
    cwd: &Path,
    timeout: Duration,
    cap: usize,
) -> AppResult<ShellOutcome> {
    let (program, flag) = if cfg!(target_os = "windows") {
        ("cmd.exe", "/C")
    } else {
        ("sh", "-c")
    };
    let mut cmd = Command::new(program);
    cmd.arg(flag)
        .arg(command)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let child = cmd.spawn()?;
    capture_child(child, cap, timeout).await
}

async fn run_program(
    program: &str,
    args: &[&str],
    cwd: &Path,
    envs: &[(&str, &str)],
    timeout: Duration,
    cap: usize,
) -> AppResult<ShellOutcome> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let child = cmd.spawn()?;
    capture_child(child, cap, timeout).await
}

async fn capture_child(mut child: Child, cap: usize, timeout: Duration) -> AppResult<ShellOutcome> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Internal("stdout pipe missing".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Internal("stderr pipe missing".into()))?;
    let exceeded = Arc::new(Notify::new());
    let out_task = tokio::spawn(read_pipe(stdout, cap, exceeded.clone()));
    let err_task = tokio::spawn(read_pipe(stderr, cap, exceeded.clone()));
    enum End {
        Exit(std::process::ExitStatus),
        Timeout,
        Cap,
    }
    let end = tokio::select! {
        status = child.wait() => End::Exit(status?),
        _ = exceeded.notified() => End::Cap,
        _ = tokio::time::sleep(timeout) => End::Timeout,
    };
    let (exit_code, timed_out, mut killed_by_cap) = match end {
        End::Exit(status) => (status.code(), false, false),
        End::Timeout => (None, true, false),
        End::Cap => (None, true, true),
    };
    if timed_out {
        drop(child);
    }
    let (stdout, stdout_truncated) = out_task.await??;
    let (stderr, stderr_truncated) = err_task.await??;
    killed_by_cap |= stdout_truncated || stderr_truncated;
    Ok(ShellOutcome {
        stdout,
        stderr,
        exit_code,
        timed_out,
        truncated: killed_by_cap,
    })
}

async fn read_pipe<R>(reader: R, cap: usize, exceeded: Arc<Notify>) -> AppResult<(String, bool)>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut limited = reader.take((cap + 1) as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes).await?;
    let truncated = bytes.len() > cap;
    if truncated {
        bytes.truncate(cap);
        exceeded.notify_one();
    }
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

fn html_decode(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&ndash;", "-")
        .replace("&mdash;", "—")
        .replace("&hellip;", "…")
}

fn strip_html(input: &str) -> String {
    let scripts = Regex::new(r"(?is)<(script|style|noscript)[^>]*>.*?</\1>")
        .unwrap()
        .replace_all(input, " ");
    let tags = Regex::new(r"(?s)<[^>]+>")
        .unwrap()
        .replace_all(&scripts, " ");
    let decoded = html_decode(&tags);
    Regex::new(r"\s+")
        .unwrap()
        .replace_all(decoded.trim(), " ")
        .into_owned()
}

fn resolve_ddg_url(raw: &str) -> String {
    let raw = raw.trim();
    if let Ok(url) = url::Url::parse(raw) {
        if let Some((_, target)) = url.query_pairs().find(|(key, _)| key == "uddg") {
            return target.into_owned();
        }
        return url.to_string();
    }
    if let Ok(url) = url::Url::parse(&format!("https://duckduckgo.com{raw}")) {
        if let Some((_, target)) = url.query_pairs().find(|(key, _)| key == "uddg") {
            return target.into_owned();
        }
    }
    raw.to_string()
}

fn escape_sendkeys(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '+' | '^' | '%' | '~' | '(' | ')' | '{' | '}' | '[' | ']' => {
                out.push('{');
                out.push(ch);
                out.push('}');
            }
            '\n' => out.push_str("{ENTER}"),
            '\r' => {}
            '\t' => out.push_str("{TAB}"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_tools_are_absent_by_default() {
        let settings = Settings::default();
        let tools = definitions(&settings, true);
        let text = tools.to_string();
        assert!(!text.contains("desktop_screenshot"));
        assert!(text.contains("web_search"));
    }

    #[test]
    fn file_tools_require_a_project() {
        let settings = Settings::default();
        let tools = definitions(&settings, false).to_string();
        assert!(!tools.contains("read_file"));
        assert!(tools.contains("web_search"));
    }

    #[test]
    fn sendkeys_escapes_control_characters() {
        assert_eq!(escape_sendkeys("a+b\n"), "a{+}b{ENTER}");
    }

    #[test]
    fn html_is_stripped_and_decoded() {
        let text =
            strip_html("<html><script>bad()</script><h1>Salam&nbsp;&amp;&nbsp;Dock</h1></html>");
        assert_eq!(text, "Salam & Dock");
    }

    #[test]
    fn ddg_redirect_is_resolved() {
        assert_eq!(
            resolve_ddg_url("//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fa"),
            "https://example.com/a"
        );
    }
}
