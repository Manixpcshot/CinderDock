# Contributing

Thank you for helping improve CinderDock.

## Ground rules

- Keep the Rust core responsible for provider calls, tools, filesystem/command access, persistence, secrets and permissions.
- Do not add Electron, mandatory cloud services, telemetry, Python runtime dependencies, fake tool success, or no-op buttons.
- Do not add in-app updates without valid signature verification and secure private-key handling.
- Never commit API keys, tokens, personal chats, screenshots containing secrets, build outputs or local databases.
- Do not copy another product's branding or proprietary assets. CinderDock has its own identity.

## Development setup

```bash
npm ci
npm run tauri dev
```

See [docs/LOCAL_DEVELOPMENT.md](docs/LOCAL_DEVELOPMENT.md).

## Before opening a PR

```bash
npm run lint
npm run typecheck
npm test
npm run build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

If you change UI appearance, regenerate real screenshots:

```bash
npm run screenshots
```

If you change provider behavior, update [docs/PROVIDERS.md](docs/PROVIDERS.md) and add stream/parser tests.

## Versioning

Every update to `main` must bump the semantic version:

```bash
npm run version:bump X.Y.Z
```

Update `CHANGELOG.md` with **Changes** and **Known limitations**. The release workflow refuses unchanged versions and creates a tag only after checks and Windows build succeed.

## Pull request description

Include:

1. What changed and why.
2. Which checks you ran.
3. Screenshots for UI changes.
4. Security implications for provider, tool, file, command or secret changes.
5. Any new known limitations.

Use the PR template. Do not paste secrets into logs or test fixtures.
