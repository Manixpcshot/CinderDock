# Security policy

## Supported versions

Security fixes target the latest released `vX.Y.Z` version and the `main` branch. Older releases may not receive backports.

## Reporting a vulnerability

Please **do not** open a public issue for a security vulnerability. Use GitHub's private vulnerability reporting for this repository, or contact the maintainers privately through GitHub.

Include:

- Affected version and Windows build.
- Reproduction steps.
- Impact and suggested fix if known.
- Logs or screenshots with **all API keys, tokens and personal data removed**.

Never send an API key, DPAPI blob, `settings.json` containing secrets, or an unredacted screenshot.

## What to expect

- Acknowledgement after the report is reviewed.
- Coordination on a fix, version bump and release notes.
- Public disclosure after a fixed release is available, or by mutual agreement.

## Security design

See [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md). CinderDock stores API keys with Windows DPAPI, never returns saved keys to the UI, redacts event/error payloads, scopes file tools to the selected project, blocks sensitive paths and private web hosts, requires approval for risky actions, and caps tool output and runtime.
