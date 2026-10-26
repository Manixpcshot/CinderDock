use std::{collections::BTreeMap, path::Path, time::Duration};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    models::{
        ApprovalDecision, AttachmentView, ConversationData, ConversationSummary, FileChangeView,
        MessageView, ToolCallView, ToolEventView,
    },
    security::{now_rfc3339, truncate_chars},
};

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  project_path TEXT,
  model_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL DEFAULT '',
  attachments_json TEXT NOT NULL DEFAULT '[]',
  tool_calls_json TEXT NOT NULL DEFAULT '[]',
  tool_call_id TEXT,
  status TEXT NOT NULL DEFAULT 'complete',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_events (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  summary TEXT NOT NULL,
  details TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS file_changes (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  before_text TEXT NOT NULL,
  after_text TEXT NOT NULL,
  diff_text TEXT NOT NULL,
  existed_before INTEGER NOT NULL,
  after_hash TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS approval_decisions (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  decision TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_seq ON messages(conversation_id, seq);
CREATE INDEX IF NOT EXISTS idx_tool_events_conversation ON tool_events(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_file_changes_conversation ON file_changes(conversation_id, created_at);

CREATE TRIGGER IF NOT EXISTS touch_conversation
AFTER INSERT ON messages
BEGIN
  UPDATE conversations SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = NEW.conversation_id;
END;
"#;

pub fn open_db(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_memory_db() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> AppResult<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}

fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
        [],
    )?;
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    if current < 1 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(MIGRATION_1)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?1)",
            params![now_rfc3339()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

pub fn migration_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?)
}

pub fn create_conversation(
    conn: &Connection,
    id: &str,
    title: &str,
    project_path: Option<&str>,
    model_id: Option<&str>,
) -> AppResult<ConversationSummary> {
    let now = now_rfc3339();
    conn.execute(
        "INSERT INTO conversations(id, title, project_path, model_id, created_at, updated_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, title, project_path, model_id, now],
    )?;
    get_conversation(conn, id)
}

pub fn get_conversation(conn: &Connection, id: &str) -> AppResult<ConversationSummary> {
    conn.query_row(
        "SELECT c.id, c.title, c.project_path, c.model_id, c.created_at, c.updated_at,
                (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id),
                COALESCE((SELECT substr(m.content, 1, 180) FROM messages m
                          WHERE m.conversation_id = c.id ORDER BY m.seq DESC LIMIT 1), '')
         FROM conversations c WHERE c.id = ?1",
        params![id],
        map_conversation,
    )
    .map_err(|_| AppError::Validation("گفتوگو پیدا نشد.".into()))
}

fn map_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationSummary> {
    Ok(ConversationSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        project_path: row.get(2)?,
        model_id: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        message_count: row.get(6)?,
        preview: row.get(7)?,
    })
}

pub fn list_conversations(conn: &Connection, limit: i64) -> AppResult<Vec<ConversationSummary>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.title, c.project_path, c.model_id, c.created_at, c.updated_at,
                (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id),
                COALESCE((SELECT substr(m.content, 1, 180) FROM messages m
                          WHERE m.conversation_id = c.id ORDER BY m.seq DESC LIMIT 1), '')
         FROM conversations c ORDER BY c.updated_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], map_conversation)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn search_conversations(conn: &Connection, query: &str) -> AppResult<Vec<ConversationSummary>> {
    let needle = format!("%{}%", query.trim().to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT c.id, c.title, c.project_path, c.model_id, c.created_at, c.updated_at,
                (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id),
                COALESCE((SELECT substr(m.content, 1, 180) FROM messages m
                          WHERE m.conversation_id = c.id ORDER BY m.seq DESC LIMIT 1), '')
         FROM conversations c
         WHERE LOWER(c.title) LIKE ?1
            OR EXISTS (SELECT 1 FROM messages m WHERE m.conversation_id = c.id AND LOWER(m.content) LIKE ?1)
         ORDER BY c.updated_at DESC LIMIT 50",
    )?;
    let rows = stmt.query_map(params![needle], map_conversation)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn delete_conversation(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::Validation("گفتوگو پیدا نشد.".into()));
    }
    Ok(())
}

pub fn set_conversation_model(conn: &Connection, id: &str, model_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE conversations SET model_id = ?2 WHERE id = ?1",
        params![id, model_id],
    )?;
    Ok(())
}

pub fn set_conversation_project(
    conn: &Connection,
    id: &str,
    project_path: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE conversations SET project_path = ?2 WHERE id = ?1",
        params![id, project_path],
    )?;
    Ok(())
}

pub fn clear_model_references(conn: &Connection, model_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE conversations SET model_id = NULL WHERE model_id = ?1",
        params![model_id],
    )?;
    Ok(())
}

pub fn insert_message(conn: &Connection, message: &MessageView) -> AppResult<MessageView> {
    let next_seq: i64 = conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE conversation_id = ?1",
        params![message.conversation_id],
        |row| row.get(0),
    )?;
    let seq = if message.seq > 0 {
        message.seq
    } else {
        next_seq
    };
    let (content, _) = truncate_chars(&message.content, 200_000);
    let attachments = serde_json::to_string(&message.attachments)?;
    let tool_calls = serde_json::to_string(&message.tool_calls)?;
    conn.execute(
        "INSERT INTO messages(id, conversation_id, seq, role, content, attachments_json, tool_calls_json,
                             tool_call_id, status, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
           content = excluded.content,
           attachments_json = excluded.attachments_json,
           tool_calls_json = excluded.tool_calls_json,
           status = excluded.status",
        params![
            message.id,
            message.conversation_id,
            seq,
            message.role,
            content,
            attachments,
            tool_calls,
            message.tool_call_id,
            message.status,
            message.created_at
        ],
    )?;
    let mut saved = message.clone();
    saved.seq = seq;
    saved.content = content;
    Ok(saved)
}

pub fn list_messages(conn: &Connection, conversation_id: &str) -> AppResult<Vec<MessageView>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, attachments_json, tool_calls_json,
                tool_call_id, status, created_at, seq
         FROM messages WHERE conversation_id = ?1 ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        let attachments_json: String = row.get(4)?;
        let tool_calls_json: String = row.get(5)?;
        Ok(MessageView {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            attachments: serde_json::from_str(&attachments_json).unwrap_or_default(),
            tool_calls: serde_json::from_str(&tool_calls_json).unwrap_or_default(),
            tool_call_id: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
            seq: row.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn delete_last_assistant_turn(conn: &Connection, conversation_id: &str) -> AppResult<bool> {
    let last_user_seq: Option<i64> = conn
        .query_row(
            "SELECT MAX(seq) FROM messages WHERE conversation_id = ?1 AND role = 'user'",
            params![conversation_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let Some(seq) = last_user_seq else {
        return Ok(false);
    };
    let changed = conn.execute(
        "DELETE FROM messages WHERE conversation_id = ?1 AND seq > ?2",
        params![conversation_id, seq],
    )?;
    Ok(changed > 0)
}

pub fn insert_tool_event(conn: &Connection, event: &ToolEventView) -> AppResult<()> {
    let (details, _) = truncate_chars(&event.details, 100_000);
    conn.execute(
        "INSERT OR REPLACE INTO tool_events(id, conversation_id, kind, name, status, summary, details, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event.id,
            event.conversation_id,
            event.kind,
            event.name,
            event.status,
            event.summary,
            details,
            event.created_at
        ],
    )?;
    Ok(())
}

pub fn list_tool_events(conn: &Connection, conversation_id: &str) -> AppResult<Vec<ToolEventView>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, kind, name, status, summary, details, created_at
         FROM tool_events WHERE conversation_id = ?1 ORDER BY created_at ASC, rowid ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok(ToolEventView {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            kind: row.get(2)?,
            name: row.get(3)?,
            status: row.get(4)?,
            summary: row.get(5)?,
            details: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn insert_file_change(conn: &Connection, change: &FileChangeView) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO file_changes(id, conversation_id, path, before_text, after_text,
            diff_text, existed_before, after_hash, status, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            change.id,
            change.conversation_id,
            change.path,
            change.before,
            change.after,
            change.diff,
            change.existed_before,
            change.after_hash,
            change.status,
            change.created_at
        ],
    )?;
    Ok(())
}

pub fn get_file_change(conn: &Connection, id: &str) -> AppResult<FileChangeView> {
    conn.query_row(
        "SELECT id, conversation_id, path, before_text, after_text, diff_text,
                existed_before, after_hash, status, created_at
         FROM file_changes WHERE id = ?1",
        params![id],
        map_file_change,
    )
    .map_err(|_| AppError::Validation("تغییر فایل پیدا نشد.".into()))
}

fn map_file_change(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileChangeView> {
    Ok(FileChangeView {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        path: row.get(2)?,
        before: row.get(3)?,
        after: row.get(4)?,
        diff: row.get(5)?,
        existed_before: row.get::<_, i64>(6)? != 0,
        after_hash: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn list_file_changes(
    conn: &Connection,
    conversation_id: &str,
) -> AppResult<Vec<FileChangeView>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, path, before_text, after_text, diff_text,
                existed_before, after_hash, status, created_at
         FROM file_changes WHERE conversation_id = ?1 ORDER BY created_at ASC, rowid ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], map_file_change)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn set_file_change_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE file_changes SET status = ?2 WHERE id = ?1",
        params![id, status],
    )?;
    Ok(())
}

pub fn insert_approval_decision(
    conn: &Connection,
    request_id: &str,
    conversation_id: &str,
    tool_name: &str,
    decision: ApprovalDecision,
) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO approval_decisions(id, conversation_id, tool_name, decision, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            request_id,
            conversation_id,
            tool_name,
            format!("{decision:?}").to_lowercase(),
            now_rfc3339()
        ],
    )?;
    Ok(())
}

pub fn load_conversation(conn: &Connection, id: &str) -> AppResult<ConversationData> {
    Ok(ConversationData {
        conversation: get_conversation(conn, id)?,
        messages: list_messages(conn, id)?,
        tool_events: list_tool_events(conn, id)?,
        file_changes: list_file_changes(conn, id)?,
    })
}

pub fn tool_call_view_from_value(value: &Value) -> Option<ToolCallView> {
    Some(ToolCallView {
        id: value.get("id")?.as_str()?.to_string(),
        name: value
            .get("name")
            .or_else(|| value.pointer("/function/name"))
            .and_then(Value::as_str)?
            .to_string(),
        arguments: value
            .get("arguments")
            .or_else(|| value.pointer("/function/arguments"))
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| "{}".into()),
    })
}

#[allow(dead_code)]
pub fn empty_headers() -> BTreeMap<String, String> {
    BTreeMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::new_id;

    #[test]
    fn migrations_are_idempotent_and_preserve_data() {
        let conn = open_memory_db().unwrap();
        migrate(&conn).unwrap();
        assert_eq!(migration_version(&conn).unwrap(), 1);
        let id = new_id("conv");
        create_conversation(&conn, &id, "Test", None, None).unwrap();
        assert_eq!(list_conversations(&conn, 10).unwrap().len(), 1);
    }

    #[test]
    fn messages_and_search_work() {
        let conn = open_memory_db().unwrap();
        let id = new_id("conv");
        create_conversation(&conn, &id, "پروژه فارسی", None, None).unwrap();
        let message = MessageView {
            id: new_id("msg"),
            conversation_id: id.clone(),
            role: "user".into(),
            content: "سلام CinderDock".into(),
            attachments: vec![AttachmentView {
                path: "a.txt".into(),
                name: "a.txt".into(),
                kind: "text".into(),
                mime: "text/plain".into(),
                size: 5,
                text: Some("hello".into()),
                data_url: None,
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            status: "complete".into(),
            created_at: now_rfc3339(),
            seq: 0,
        };
        insert_message(&conn, &message).unwrap();
        let messages = list_messages(&conn, &id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].attachments.len(), 1);
        assert_eq!(search_conversations(&conn, "CinderDock").unwrap().len(), 1);
    }

    #[test]
    fn retry_deletes_only_turn_after_last_user() {
        let conn = open_memory_db().unwrap();
        let id = new_id("conv");
        create_conversation(&conn, &id, "Retry", None, None).unwrap();
        for (role, content) in [
            ("user", "first"),
            ("assistant", "old"),
            ("user", "second"),
            ("assistant", "bad"),
            ("tool", "result"),
        ] {
            insert_message(
                &conn,
                &MessageView {
                    id: new_id("msg"),
                    conversation_id: id.clone(),
                    role: role.into(),
                    content: content.into(),
                    attachments: Vec::new(),
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                    status: "complete".into(),
                    created_at: now_rfc3339(),
                    seq: 0,
                },
            )
            .unwrap();
        }
        assert!(delete_last_assistant_turn(&conn, &id).unwrap());
        let messages = list_messages(&conn, &id).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages.last().unwrap().content, "second");
    }
}

pub fn mark_streaming_messages_error(
    conn: &Connection,
    conversation_id: &str,
    message: &str,
) -> AppResult<()> {
    let (clean, _) = truncate_chars(message, 4000);
    conn.execute(
        "UPDATE messages
         SET status = 'error',
             content = CASE
               WHEN TRIM(content) = '' THEN ?2
               ELSE content || char(10,10) || ?2
             END
         WHERE conversation_id = ?1 AND status = 'streaming'",
        params![conversation_id, format!("خطا: {clean}")],
    )?;
    Ok(())
}
