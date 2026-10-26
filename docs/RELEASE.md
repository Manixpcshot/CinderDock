# Release process

Releases are created by `.github/workflows/release.yml` on `main` or by `workflow_dispatch`. A `vX.Y.Z` tag is created only after checks, a real Windows build, and portable-executable smoke tests succeed.

## Version rules

Every `main` update must bump the semantic version. Run:

```bash
npm run version:bump 0.1.1
```

Then update `CHANGELOG.md` with:

```markdown
## [0.1.1] - YYYY-MM-DD

### Changes
- ...

### Known limitations
- ...
```

The version must match in:

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` package entry
- the changelog section

CI verifies consistency and, for `main`/PRs, verifies the version increased.

## Checks before tagging

- ESLint, TypeScript, Vitest UI tests, frontend build.
- Rust `fmt --check`, Clippy with warnings denied, Rust tests.
- Release version is newer than the latest `v*` tag.
- Windows `tauri build` on `windows-latest`.
- `--cinderdock-smoke-test` exit code and report.
- GUI process stays alive for an eight-second launch smoke test.

If any step fails, the workflow stops before tag creation.

## Exactly four uploaded assets

The release job creates a staging directory with exactly these files:

```text
CinderDock-vX.Y.Z-windows-x64-portable.exe
CinderDock-vX.Y.Z-windows-x64-setup.exe
CinderDock-vX.Y.Z-source.zip
SHA256SUMS.txt
```

`SHA256SUMS.txt` contains hashes for the first three files. The source archive is produced with `git archive` from the newly created tag, so binaries, source and tag all correspond to the same commit. Release notes are generated from the matching `CHANGELOG.md` section and include **Known limitations**.

GitHub's automatic “Source code” links may appear on the release page; they are repository links, not uploaded release assets.

## Manual dispatch

```bash
gh workflow run release.yml -f version=0.1.1
```

If no version is supplied, the workflow uses `package.json`. A release is never created for an unchanged version, an existing release, or a failed build.

## Verifying locally

```powershell
$version = "0.1.0"
Get-Content SHA256SUMS.txt
Get-FileHash ".\CinderDock-v$version-windows-x64-portable.exe" -Algorithm SHA256
Get-FileHash ".\CinderDock-v$version-windows-x64-setup.exe" -Algorithm SHA256
Get-FileHash ".\CinderDock-v$version-source.zip" -Algorithm SHA256
```

Compare lowercase hex digests. Do not run unsigned or unexpected executables.
