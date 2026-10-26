# Changelog

All notable changes to CinderDock are documented here. Every update to `main` must bump the semantic version.

## [0.1.0] - 2026-10-26

### Changes

- First usable Windows x64 desktop application built with Tauri 2, React, TypeScript, Vite and Rust.
- Original CinderDock identity, icons, dark design system, reusable components, Persian RTL layout and keyboard-accessible controls.
- Native Persian, display-only Finglish, and Latin-only text modes; stored messages are never rewritten.
- In-app provider connection wizard with OpenAI-compatible custom endpoints and OpenAI, OpenRouter, DeepSeek, Ollama, LM Studio presets.
- Real native Ollama adapter for `/api/chat` and `/api/tags`; OpenAI-compatible Ollama entry remains separate.
- Model discovery through supported APIs, manual model IDs, editable tool-calling/vision capabilities, searchable picker, default model and per-chat switching.
- Streaming Chat Completions and native Ollama responses, stop, bounded retries, bounded tool loop and repeated-call detection.
- Real tools: list/read files, project search, create/edit files, command execution, Git status/diff, DuckDuckGo web search and public web-page fetch.
- Optional Windows desktop screenshot/click/type/scroll tools, disabled by default with explicit per-use approval.
- Detailed approvals with “only once”, “for this session”, and “deny”; rejection stops the tool path.
- Unified diffs before writes, applied-change records, hash-checked revert, and visible tool calls, results, errors, approvals and terminal output.
- SQLite persistence with idempotent migration v1, conversation search/delete, message history, tool events, file changes and approval decisions.
- Windows DPAPI secret storage, recursive event/error redaction, project path traversal and symlink guards, sensitive-file blocking, SSRF guard, output caps and command timeouts.
- Complete loading, streaming, error, disconnected and empty states; 1366×768 to 4K responsive layout.
- CI with Rust formatting/Clippy/tests, TypeScript checks, UI tests, real `windows-latest` build, Windows portable smoke test, semantic version checks and guarded Release workflow.

### Known limitations

- Windows 10/11 x64 and Microsoft Edge WebView2 Runtime are required. The portable executable is not fully standalone.
- In-app updates are disabled until valid signature verification and secure private-key custody are implemented.
- Native Ollama tool calling depends on the installed Ollama version and model capabilities; unknown capabilities default to off.
- DuckDuckGo HTML search is best-effort and may change or rate-limit. Web-page fetch blocks private network hosts.
- Desktop screenshots may capture sensitive information visible on screen; they require separate approval and are not persisted, but users must still avoid exposing secrets on screen.
- Command approval is a user-authorization boundary, not a full operating-system sandbox.
