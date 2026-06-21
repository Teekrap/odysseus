# Odysseus desktop

A small Tauri shell that wraps the Odysseus web UI in a native desktop window, so you can use it without a browser tab. It does **not** bundle the Python backend — it connects to an already-running Odysseus server (Docker, native, etc.) on your LAN or localhost.

## What it does

- First launch: shows a small settings card with a blank **Server URL** field (e.g. `http://10.0.0.29:7000`).
- Click **Connect**: the app probes `GET /api/companion/ping` on the server. If alive (2xx or 401), it navigates the window to the Odysseus UI and remembers the URL. On failure it shows an error and lets you try again.
- Subsequent launches: if a remembered URL still pings OK, the app loads it directly; otherwise it falls back to the settings card.
- The **Change server...** menu item (app menu / window menu) jumps back to the settings card so you can switch servers without quitting.
- Session cookies persist natively in the OS webview (WebView2 on Windows, WebKit on macOS, WebKitGTK on Linux), so you stay logged in across restarts.
- Window size and position are remembered across launches via `tauri-plugin-window-state`.

## Prerequisites

To build this yourself you need, on your client machine (not the server):

- **Node.js 18+** (we use the system Node here; 20+ recommended)
- **Rust stable** with `cargo` (https://rustup.rs)
- OS-specific webview runtime, installed by Tauri's getting-started guide:
  - Windows: Microsoft Edge WebView2 runtime (preinstalled on Windows 11; bundled with the build output otherwise)
  - macOS: Xcode Command Line Tools
  - Linux: `webkit2gtk-4.1`, `libgtk-3`, `libayatana-appindicator3`, etc. See https://tauri.app/start/prerequisites/

No Python, no Docker, no Odysseus backend on the client machine.

## Develop

```sh
cd desktop
npm install
npm run dev
```

This launches a Rust dev build with hot settings card. Edit `src/index.html`, `src/main.js`, `src/styles.css` and reload the window.

## Build an installer

```sh
npm run build
```

Outputs land in `src-tauri/target/release/bundle/`:

- Windows: `.msi` and `.exe` (NSIS)
- macOS: `.dmg` and `.app`
- Linux: `.deb`, `.rpm`, and AppImage

## Where settings live

Per-user, in the OS config dir:

- Windows: `%APPDATA%\app.odysseus.desktop\settings.json`
- macOS:   `~/Library/Application Support/app.odysseus.desktop/settings.json`
- Linux:   `~/.config/app.odysseus.desktop/settings.json`

It just stores `{"server_url": "http://10.0.0.29:7000"}`. Delete the file (or click *Change server...*) to reset.

## Why this exists

The repo's existing `launcher.py` / `Odysseus.spec` PyInstaller path bundles the **backend** and opens the system browser. This Tauri shell is the inverse: it bundles nothing on the backend side and instead gives the UI a real native window of its own, so it feels like the Claude desktop app rather than a browser tab. Pair it with a server you already trust (your Docker host, a Tailscale URL, etc.).

## Files

```
desktop/
  package.json
  README.md
  src/
    index.html      settings card
    main.js
    styles.css
  src-tauri/
    Cargo.toml
    build.rs
    tauri.conf.json
    icons/
    src/
      main.rs
```
