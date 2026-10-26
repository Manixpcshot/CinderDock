# Providers and models

CinderDock does not ship a model. You connect to providers you trust and choose real model IDs.

## Connection wizard

Open **Provider connections** from the sidebar. No manual JSON editing is required.

Fields:

- **Preset** — changes safe defaults only; every value remains editable.
- **Display name** — local label.
- **Base URL** — normalized with a trailing slash. Credentials in the URL are rejected.
- **Authentication** — `Bearer`, `None`, or `Custom header`.
- **API key** — password input, never displayed after saving. Editing a provider can leave this empty to keep the stored key.
- **Optional headers** — non-secret headers only. Reserved headers and values that look like tokens are rejected.
- **Models path / chat path** — relative endpoint paths. Use an empty models path if the provider has no discovery endpoint.
- **Test connection** — checks the discovery endpoint; if it is absent, it checks the base URL and clearly reports that discovery is unavailable.

## Presets

| Preset | Adapter | Default endpoint | Notes |
| --- | --- | --- | --- |
| OpenAI | OpenAI-compatible Chat Completions | `https://api.openai.com/v1` | Bearer key, `/models`, `/chat/completions` |
| OpenRouter | OpenAI-compatible Chat Completions | `https://openrouter.ai/api/v1` | Model IDs usually include a publisher prefix |
| DeepSeek | OpenAI-compatible Chat Completions | `https://api.deepseek.com/v1` | Verify current model IDs in your account |
| Ollama native | Native Ollama | `http://127.0.0.1:11434` | Uses `/api/chat` and `/api/tags`; no false OpenAI claim |
| Ollama OpenAI-compatible | OpenAI-compatible Chat Completions | `http://127.0.0.1:11434/v1` | Only for Ollama versions exposing `/v1` |
| LM Studio | OpenAI-compatible Chat Completions | `http://127.0.0.1:1234/v1` | Start the local server first |
| Custom | OpenAI-compatible Chat Completions | `http://127.0.0.1:8000/v1` | Use only if the server really implements Chat Completions |

## Model discovery

- OpenAI-compatible providers should return `{"data":[{"id":"..."}]}` from the models path.
- OpenRouter-style metadata can provide `architecture.input_modalities`, `supported_parameters`, and `context_length`; CinderDock uses these when present and leaves unknown capabilities off.
- Native Ollama returns `{"models":[{"name":"..."}]}`. Parameter size is not treated as a context window.
- If discovery fails or returns no models, manual model entry still works. The UI says discovery is unavailable instead of pretending success.

## Model capabilities

Each saved model stores:

- `id` — internal CinderDock ID.
- `modelId` — the real provider model ID sent over the wire.
- `displayName`.
- `providerId`.
- `supportsTools` — enables tool definitions in the chat payload.
- `supportsVision` — permits image attachments and screenshot images.
- `contextWindow` — informational metadata.

Unknown capabilities default to **off**. You can edit them in **Manage models**. If a selected model lacks vision, image attachments are blocked and the UI shows a clear warning.

## Switching without restart

The top-bar model selector is searchable and grouped by provider. Selecting a model updates the current conversation's model immediately; the Rust core reads settings on every run. Multiple providers and models can coexist. The default model is used for new chats.

## Local servers

For Ollama or LM Studio, start the server before testing. CinderDock allows `http://localhost` in **provider Base URL** values because that is a user-chosen provider, while the web-fetch tool blocks private hosts to reduce SSRF risk.
