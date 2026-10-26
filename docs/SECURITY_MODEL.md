# Security model

CinderDock is a local desktop agent with powerful tools. Its security boundary is deliberately explicit.

## Secrets

- API keys are entered only in a password field.
- They are stored with Windows DPAPI (`CryptProtectData` with `CRYPTPROTECT_UI_FORBIDDEN`) in a per-user encrypted credential file.
- Saved keys are never returned to the frontend, included in `settings.json`, written to logs, or included in release/source archives.
- Every emitted event and command error passes through redaction for known secrets and common key-shaped patterns.
- Extra provider headers that look like tokens or use reserved auth names are rejected.
- Screenshots in documentation contain no API keys. Desktop screenshots taken by the agent can capture anything visible on screen; explicit approval warns about this.

## Filesystem

- File tools are scoped to the selected project root.
- Absolute paths, `..`, drive prefixes and symlink escapes are rejected. Both the project root and target parent are canonicalized.
- Sensitive paths are blocked: `.env*`, `.git`, `.ssh`, `.aws`, private keys, credential files, `.npmrc`, `.netrc`, and similar names.
- Reads are capped; binary files are rejected; writes are capped to what can be shown and reverted from a full diff.
- Writes require approval, show a unified diff, and are atomic. File changes can be reverted only if the current file hash still matches the recorded post-write hash.

## Commands and processes

- Commands require approval with the exact command, risk level and bounded timeout.
- Child processes use null stdin, piped and capped stdout/stderr, wall-clock timeout and kill-on-drop.
- Critical commands cannot be granted for a whole session.
- Commands run inside the selected project directory. CinderDock does not claim a full OS sandbox; approval is the boundary.

## Network

- Provider Base URLs are user-configured and may be local.
- Web fetch rejects private, loopback, link-local, unspecified and credential-bearing URLs.
- Web search/fetch output is capped and redacted before events/logs.
- Provider responses are streamed with byte, character and time caps.

## Approvals

- Writes, commands, desktop control and destructive Git operations require a detailed approval.
- Choices are **Only this time**, **For this session**, or **Deny**.
- Session grants are in memory, scoped by project and tool name, and cleared on project/settings change or app restart.
- Desktop-control approvals are never session-grantable.
- Denial stops the tool path; the backend never reports success without a real tool result.

## Agent loop

- `max_tool_iterations` bounds the number of model/tool cycles.
- Repeated identical tool calls are detected and stop the loop.
- Retries are bounded and only happen before any streamed response delta.
- Stream parsers reject malformed payloads and provider error objects.

## Updates and telemetry

- In-app updates are disabled until signature verification and secure private-key custody are implemented.
- The app has no mandatory account, cloud sync or telemetry. It talks only to providers and web endpoints the user configures or invokes.

## Reporting

See [../SECURITY.md](../SECURITY.md). Do not include API keys in reports.
