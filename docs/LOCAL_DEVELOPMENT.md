# Local development

## Prerequisites

- Node.js 22 and npm 10+.
- Rust stable with `rustfmt` and `clippy`.
- For Tauri on Linux: WebKit2GTK 4.1, Ayatana AppIndicator, librsvg and patchelf.
- Windows development: Windows 10/11, Microsoft C++ Build Tools, WebView2 Runtime.

```bash
git clone https://github.com/Manixpcshot/cinderdock.git
cd cinderdock
npm ci
npm run tauri dev
```

`npm run tauri dev` starts Vite on `http://localhost:1420` and the Tauri shell. The frontend can also be started with `npm run dev` for UI-only work, but backend commands will not be available unless the Tauri shell is running.

## Checks

```bash
npm run lint
npm run typecheck
npm test
npm run build

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Run `npm run screenshots` to regenerate the real UI screenshots from the production build. The script starts its own Vite preview, launches a headless Chromium with non-sensitive sample data, writes images to `docs/screenshots/`, and shuts the server down.

## Code style

- Rust: `cargo fmt` (edition 2021).
- TypeScript: ESLint, strict TypeScript, Prettier-compatible formatting.
- CSS: design tokens in `:root`, reusable component classes, logical properties for RTL, visible focus rings.
- No API keys, personal chats, generated bundles or large datasets in Git.

## Adding a provider preset

1. Add a `ProviderPreset` in `src-tauri/src/providers.rs`.
2. If the API is truly OpenAI Chat Completions-compatible, use `OpenAiCompatible` and set `modelsPath`/`chatPath`.
3. If it is not compatible, add or extend a native adapter in `providers.rs` and `stream.rs`; do not label it OpenAI-compatible.
4. Update `docs/PROVIDERS.md` and the UI note. Add tests for URL/payload/stream parsing where applicable.
5. Bump the version and update `CHANGELOG.md`.
