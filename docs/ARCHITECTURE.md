# Architecture

CinderDock is intentionally split into a thin presentation layer and a Rust core that owns all capabilities.

## Process boundary

- **Frontend:** React 18, TypeScript, Vite, design tokens and reusable components. It renders state, collects form input, and listens to typed `cinderdock-event` messages. It never calls a provider, opens a file, runs a command, or reads a secret directly.
- **Bridge:** Tauri 2 IPC commands in `src-tauri/src/commands.rs`. Command errors are redacted before crossing to the UI.
- **Rust core:** provider adapters, streaming parsers, bounded agent loop, tools, approvals, SQLite, settings, path guards and DPAPI secret storage.

## Module map

| Rust module | Responsibility |
| --- | --- |
| `commands.rs` | Tauri command surface and input validation |
| `app_state.rs` | Shared settings, database, secret store, approvals, run registry, HTTP client |
| `agent.rs` | Conversation/message preparation, retrying stream, bounded tool loop, persistence |
| `stream.rs` | OpenAI-compatible SSE and native Ollama NDJSON parsing, including tool-call deltas |
| `providers.rs` | Presets, URL/header validation, model discovery, test connection, chat payloads, Ollama conversion |
| `tools.rs` | Tool schemas and real tool execution, approvals, terminal output, web/desktop tools |
| `security.rs` | Project path canonicalization, sensitive-path blocking, SSRF guard, output caps, redaction |
| `secrets.rs` | Windows DPAPI-backed secret store; in-memory fallback on development platforms |
| `config.rs` | `settings.json` load/save/validation and app paths |
| `db.rs` | SQLite connection, idempotent migration v1, conversation/message/event/change CRUD |
| `approvals.rs` | Pending approvals, session grants, cancellation |
| `events.rs` | Recursive redaction and typed event emission |
| `diff.rs` | Real unified diff generation |

## Streaming and tool loop

1. `send_message` persists the user message and creates a streaming assistant placeholder.
2. `build_llm_messages` builds a bounded system+history payload. Text attachments are inlined; images are included only for vision-capable models.
3. `providers::chat_request` sends a streaming POST to the configured endpoint. `StreamParser` normalizes OpenAI SSE and Ollama NDJSON into typed deltas and tool calls.
4. `agent` streams deltas to the UI, persists the final assistant message, then executes tool calls through `ToolRunner`.
5. Tool calls, results, errors, terminals, approvals and file changes are persisted and emitted. A denied approval or repeated identical call stops the loop.
6. The loop is bounded by `max_tool_iterations`; retries are bounded by `max_retries` and only happen before any response delta.

## Persistence

- SQLite file: `%APPDATA%\CinderDock\data\cinderdock.sqlite3`.
- Migration v1 creates `conversations`, `messages`, `tool_events`, `file_changes`, `approval_decisions`, indexes and an update trigger.
- `settings.json` holds only non-secret configuration. Keys live in DPAPI-encrypted blobs.
- File changes store before/after text and a unified diff so a confirmed revert can be hash-checked before restoring.

## Security invariants

- No API key is returned by any command or event.
- Event payloads are recursively redacted before emission.
- File tools canonicalize both project root and target parent to prevent `..` and symlink escapes.
- Sensitive files such as `.env`, `.git`, private keys and credential files are blocked by default.
- Web fetch rejects private, loopback, link-local and metadata hosts.
- Commands use stdin null, bounded wall time, bounded output pipes and kill-on-drop.
- Web search/fetch is read-only; writes, commands and desktop control require approval.

## Rendering states

The UI models loading, connecting, retrying, streaming, completed, stopped, error, disconnected and empty states explicitly. A failed backend shows a reconnect screen rather than fake data. Model pickers, approvals, settings, search and dialogs have keyboard paths and visible focus.
