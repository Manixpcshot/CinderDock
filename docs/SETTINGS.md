# settings.json

CinderDock stores non-secret configuration in a documented JSON file. The default location on Windows is:

```text
%APPDATA%\CinderDock\settings.json
```

Development builds use the platform configuration directory from the `dirs` crate. If the file is corrupt, CinderDock backs it up as `settings.json.corrupt` and restores safe defaults.

## Example

```json
{
  "schemaVersion": 1,
  "theme": "ember-dark",
  "uiLanguage": "fa",
  "textMode": "native",
  "fontScale": 1.0,
  "defaultModelId": "model_abc",
  "projectPath": "D:\\Projects\\Example",
  "recentProjects": ["D:\\Projects\\Example"],
  "providers": [
    {
      "id": "provider_abc",
      "displayName": "OpenAI",
      "preset": "openai",
      "kind": "openAiCompatible",
      "baseUrl": "https://api.openai.com/v1/",
      "auth": { "method": "bearer", "headerName": null },
      "extraHeaders": {},
      "modelsPath": "models",
      "chatPath": "chat/completions"
    }
  ],
  "models": [
    {
      "id": "model_abc",
      "providerId": "provider_abc",
      "modelId": "gpt-4o",
      "displayName": "GPT-4o",
      "supportsTools": true,
      "supportsVision": true,
      "contextWindow": 128000
    }
  ],
  "desktopToolsEnabled": false,
  "webAccessEnabled": true,
  "maxToolIterations": 8,
  "maxRetries": 2,
  "terminalVisible": false,
  "updates": {
    "enabled": false,
    "reason": "In-app updates stay disabled until release signatures and private-key custody are configured."
  }
}
```

## What is never stored here

- API keys or bearer tokens.
- Authorization headers.
- Cookie/session values.
- Chat content.
- File contents or tool output.
- Screenshots.

API keys are encrypted with Windows DPAPI for the current user and kept in `%APPDATA%\CinderDock\secure\credentials.bin`. On non-Windows development platforms, secrets remain in memory only. See [SECURITY_MODEL.md](SECURITY_MODEL.md).

## Database

Chat history, messages, tool events, file changes and approval decisions are in:

```text
%APPDATA%\CinderDock\data\cinderdock.sqlite3
```

Migration v1 is idempotent and creates all required tables and indexes. Deleting a conversation cascades to its messages, events and file changes. Deleting the database removes local chats; it does not remove provider secrets.
