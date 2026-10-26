use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    models::{ProviderKind, StreamOutcome, ToolCall},
    security::new_id,
};

#[derive(Debug, Default, Clone)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Default, Clone)]
pub struct ParseUpdate {
    pub delta: String,
    pub done: bool,
}

#[derive(Debug)]
pub struct StreamParser {
    kind: ProviderKind,
    content: String,
    data_buffer: String,
    partials: BTreeMap<usize, PartialToolCall>,
    finished: bool,
}

impl StreamParser {
    pub fn new(kind: ProviderKind) -> Self {
        Self {
            kind,
            content: String::new(),
            data_buffer: String::new(),
            partials: BTreeMap::new(),
            finished: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn ingest(&mut self, line: &str) -> AppResult<ParseUpdate> {
        if self.finished {
            return Ok(ParseUpdate::default());
        }
        match self.kind {
            ProviderKind::OpenAiCompatible => self.ingest_sse(line),
            ProviderKind::OllamaNative => self.ingest_ndjson(line),
        }
    }

    fn ingest_sse(&mut self, line: &str) -> AppResult<ParseUpdate> {
        let trimmed = line.trim_end_matches('\r');
        if let Some(payload) = trimmed.strip_prefix("data:") {
            if !self.data_buffer.is_empty() {
                self.data_buffer.push('\n');
            }
            self.data_buffer.push_str(payload.trim());
            return Ok(ParseUpdate::default());
        }
        if trimmed.is_empty() {
            let update = self.flush_data()?;
            self.finished |= update.done;
            return Ok(update);
        }
        if trimmed.starts_with(':') || trimmed.starts_with("event:") || trimmed.starts_with("id:") {
            return Ok(ParseUpdate::default());
        }
        Ok(ParseUpdate::default())
    }

    fn flush_data(&mut self) -> AppResult<ParseUpdate> {
        let data = std::mem::take(&mut self.data_buffer);
        if data.is_empty() {
            return Ok(ParseUpdate::default());
        }
        if data == "[DONE]" {
            return Ok(ParseUpdate {
                delta: String::new(),
                done: true,
            });
        }
        let value: Value = serde_json::from_str(&data)
            .map_err(|e| AppError::Provider(format!("پاسخ جریانی نامعتبر بود: {e}")))?;
        self.parse_openai_object(&value)
    }

    fn parse_openai_object(&mut self, value: &Value) -> AppResult<ParseUpdate> {
        if let Some(error) = value.get("error") {
            return Err(AppError::Provider(format!(
                "خطای پروایدر: {}",
                human_error(error)
            )));
        }
        let mut delta = String::new();
        let mut done = false;
        if let Some(choice) = value.pointer("/choices/0") {
            if let Some(content) = choice.pointer("/delta/content") {
                extract_text(content, &mut delta);
            }
            if let Some(tool_calls) = choice
                .pointer("/delta/tool_calls")
                .and_then(Value::as_array)
            {
                for (position, call) in tool_calls.iter().enumerate() {
                    let index = call
                        .get("index")
                        .and_then(Value::as_u64)
                        .unwrap_or(position as u64) as usize;
                    let entry = self.partials.entry(index).or_default();
                    if let Some(id) = call.get("id").and_then(Value::as_str) {
                        entry.id = id.to_string();
                    }
                    if let Some(name) = call.pointer("/function/name").and_then(Value::as_str) {
                        entry.name.push_str(name);
                    }
                    if let Some(arguments) = call.pointer("/function/arguments") {
                        match arguments {
                            Value::String(text) => entry.arguments.push_str(text),
                            other => entry.arguments.push_str(&other.to_string()),
                        }
                    }
                }
            }
            if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                done = matches!(reason, "stop" | "tool_calls" | "length" | "content_filter");
            }
        }
        self.content.push_str(&delta);
        self.finished |= done;
        Ok(ParseUpdate { delta, done })
    }

    fn ingest_ndjson(&mut self, line: &str) -> AppResult<ParseUpdate> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(ParseUpdate::default());
        }
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|e| AppError::Provider(format!("خط NDJSON نامعتبر بود: {e}")))?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(AppError::Provider(format!("خطای Ollama: {error}")));
        }
        let mut delta = String::new();
        if let Some(content) = value.pointer("/message/content") {
            extract_text(content, &mut delta);
        }
        if let Some(tool_calls) = value
            .pointer("/message/tool_calls")
            .and_then(Value::as_array)
        {
            for (position, call) in tool_calls.iter().enumerate() {
                let index = call
                    .get("index")
                    .and_then(Value::as_u64)
                    .unwrap_or(position as u64) as usize;
                let entry = self.partials.entry(index).or_default();
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    entry.id = id.to_string();
                }
                if let Some(name) = call
                    .pointer("/function/name")
                    .or_else(|| call.get("name"))
                    .and_then(Value::as_str)
                {
                    entry.name.push_str(name);
                }
                if let Some(arguments) = call
                    .pointer("/function/arguments")
                    .or_else(|| call.get("arguments"))
                {
                    match arguments {
                        Value::String(text) => entry.arguments.push_str(text),
                        other => entry.arguments.push_str(&other.to_string()),
                    }
                }
            }
        }
        let done = value.get("done").and_then(Value::as_bool).unwrap_or(false);
        self.content.push_str(&delta);
        self.finished |= done;
        Ok(ParseUpdate { delta, done })
    }

    pub fn finish(&mut self) -> AppResult<StreamOutcome> {
        if !self.data_buffer.is_empty() {
            let update = self.flush_data()?;
            self.finished |= update.done;
        }
        let mut tool_calls = Vec::new();
        for partial in self.partials.values() {
            if partial.name.trim().is_empty() {
                continue;
            }
            let arguments = if partial.arguments.trim().is_empty() {
                "{}".into()
            } else {
                partial.arguments.clone()
            };
            tool_calls.push(ToolCall {
                id: if partial.id.is_empty() {
                    new_id("call")
                } else {
                    partial.id.clone()
                },
                name: partial.name.trim().into(),
                arguments,
            });
        }
        Ok(StreamOutcome {
            content: self.content.clone(),
            tool_calls,
            stopped: false,
        })
    }
}

fn extract_text(value: &Value, out: &mut String) {
    match value {
        Value::String(text) => out.push_str(text),
        Value::Array(parts) => {
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    out.push_str(text);
                } else if let Some(text) = part.as_str() {
                    out.push_str(text);
                }
            }
        }
        _ => {}
    }
}

fn human_error(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Object(map) => map
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_sse_content_and_tool_calls() {
        let mut parser = StreamParser::new(ProviderKind::OpenAiCompatible);
        for line in [
            "data: {\"choices\":[{\"delta\":{\"content\":\"سلام\"}}]}",
            "",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"function\":{\"name\":\"read_file\",\"arguments\":\"{\\\"path\\\"\"}}]}}]}",
            "",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\":\\\"a.txt\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}",
            "",
            "data: [DONE]",
            "",
        ] {
            parser.ingest(line).unwrap();
        }
        let outcome = parser.finish().unwrap();
        assert_eq!(outcome.content, "سلام");
        assert_eq!(outcome.tool_calls.len(), 1);
        assert_eq!(outcome.tool_calls[0].name, "read_file");
        assert_eq!(outcome.tool_calls[0].arguments, "{\"path\":\"a.txt\"}");
    }

    #[test]
    fn parses_ollama_ndjson() {
        let mut parser = StreamParser::new(ProviderKind::OllamaNative);
        parser
            .ingest("{\"message\":{\"content\":\"Hello\"},\"done\":false}")
            .unwrap();
        parser
            .ingest("{\"message\":{\"tool_calls\":[{\"function\":{\"name\":\"git_status\",\"arguments\":{}}}]},\"done\":true}")
            .unwrap();
        let outcome = parser.finish().unwrap();
        assert_eq!(outcome.content, "Hello");
        assert_eq!(outcome.tool_calls[0].name, "git_status");
        assert_eq!(outcome.tool_calls[0].arguments, "{}");
    }

    #[test]
    fn provider_error_is_returned() {
        let mut parser = StreamParser::new(ProviderKind::OpenAiCompatible);
        parser
            .ingest("data: {\"error\":{\"message\":\"invalid key\"}}")
            .unwrap();
        assert!(parser.ingest("").is_err());
    }
}
