# Windows build

CinderDock produces Windows x64 artifacts on a real `windows-latest` GitHub Actions runner. Linux cannot produce or validate the Windows executable for this repository.

## Local Windows build

Prerequisites:

- Windows 10/11 x64.
- Node.js 22 and npm.
- Rust stable with the `x86_64-pc-windows-msvc` target.
- Microsoft C++ Build Tools / Visual Studio Build Tools.
- WebView2 Runtime for running the app.
- NSIS support is handled by Tauri during bundling.

```powershell
npm ci
npm run build
npm run tauri build
```

Outputs:

```text
src-tauri\target\release\CinderDock.exe
src-tauri\target\release\bundle\nsis\CinderDock_<version>_x64-setup.exe
```

## Smoke test

The executable has a hidden command-line smoke mode:

```powershell
$exe = ".\src-tauri\target\release\CinderDock.exe"
$p = Start-Process -FilePath $exe -ArgumentList '--cinderdock-smoke-test' -Wait -PassThru `
  -RedirectStandardOutput smoke.json -RedirectStandardError smoke.err -WindowStyle Hidden
if ($p.ExitCode -ne 0) { Get-Content smoke.err; throw 'Smoke test failed' }
Get-Content smoke.json
```

The report checks settings loading, SQLite migration, HTTP client initialization, absence of key-shaped fields in settings, and secret-backend configuration. CI also launches the GUI for eight seconds and verifies the process stays alive.

## WebView2 reality

The NSIS installer uses Tauri's silent WebView2 bootstrapper. The portable executable does not install WebView2. If the runtime is missing, the app will not render. CinderDock is therefore **not fully standalone**; the release notes and README state this explicitly.

## Troubleshooting

- **NSIS not found:** install NSIS or let Tauri download its bundled tooling on a networked runner.
- **WebView2 missing:** install the Evergreen Runtime or use the NSIS installer with bootstrapper.
- **Cargo build fails:** ensure the MSVC target and C++ build tools are installed.
- **No icon:** verify `src-tauri/icons/icon.ico` exists and is a valid multi-size ICO.
- **Blank window:** run the smoke test, then check that `npm run build` produced `dist/index.html`.
