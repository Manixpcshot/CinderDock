# Third-party notices

CinderDock is MIT-licensed. It uses open-source packages including:

- React, React DOM — MIT.
- Vite, Vitest, TypeScript, ESLint, Prettier — MIT or compatible permissive licenses.
- Tauri 2 and `tauri-plugin-dialog` — MIT/Apache-2.0 dual license.
- Rust crates such as `serde`, `tokio`, `reqwest`, `rusqlite`, `uuid`, `chrono`, `similar`, `walkdir`, `regex`, `thiserror`, `tracing` — MIT and/or Apache-2.0.
- `lucide-react` — ISC.
- `react-markdown`, `remark-gfm` — MIT.
- Vazirmatn variable font — SIL Open Font License 1.1.
- `@sparticuz/chromium`, Playwright Core and `@resvg/resvg-js` — development/screenshot tooling; see their package licenses.

The full dependency graph and exact versions are captured by `package-lock.json` and `src-tauri/Cargo.lock`. Each dependency's license text is included in its published package. Generated binaries are not committed to this repository.
