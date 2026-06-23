# TogoClient Desktop

This directory contains the current desktop client for `TogoClient`.

## Stack

- `Tauri 2`
- `Vue 3`
- `TypeScript`
- `Vue Router`
- `Vue I18n`
- `Vite`

## Current scope

The current implementation targets parity with the sibling `TogoSpace/frontend`
repository and adds desktop-only capabilities on top:

- console chat and task views
- settings sections for teams, roles, models, skills, and maintenance
- real-time room, agent, and task updates over WebSocket
- Tauri tray menu, close-to-tray behavior, and macOS notifications
- one-click start / stop for the local `TogoSpace` backend
- top-bar desktop status button with red / green connectivity indicator
- drag-and-drop file paths into the chat composer
- clickable local paths in chat with subtle path chips, Finder reveal, and copy actions
- packaged macOS app and DMG with icons generated from `res/icon/app-icon.png`

## Local development

Recommended entry point:

```bash
cd <frontend-repo>
./scripts/dev-desktop.sh
```

The helper script defaults `TOGOSPACE_ROOT` to the parent `TogoSpace`
repository, prepares `.env`, installs npm dependencies, and forwards any extra
arguments to `tauri dev`.
The checked-in `.env.example` only keeps `VITE_API_BASE_URL`; add
`TOGOSPACE_ROOT` locally only when your `TogoSpace` repository is not the parent
directory of this frontend workspace.

In source development mode, desktop-managed backend data is written to
`frontend/.desktop-dev-storage/` so the app does not need write access to the
parent `TogoSpace` repository root.

Examples:

```bash
./scripts/dev-desktop.sh --skip-install
./scripts/dev-desktop.sh -- --no-watch
```

Manual fallback:

1. Copy `.env.example` to `.env`
2. Set `VITE_API_BASE_URL` if you need a non-default backend address
3. Optionally set `TOGOSPACE_ROOT` if the local backend repo is not the parent `TogoSpace` directory
4. Install dependencies:

```bash
npm install
```

5. Start the desktop app:

```bash
npm run tauri dev
```

## Build

If `cargo` is not already available on the machine, install Rust with the
`rsproxy.cn` mirror first:

```bash
RUSTUP_DIST_SERVER=https://rsproxy.cn \
RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup \
curl --proto '=https' --tlsv1.2 -sSf https://rsproxy.cn/rustup-init.sh | sh -s -- -y
```

Then build the macOS app bundle and DMG:

```bash
cd <frontend-repo>
./scripts/build-desktop.sh
```

The helper script defaults `TOGOSPACE_ROOT` to the parent `TogoSpace`
repository, prepares `.env`, installs npm dependencies, and forwards any extra
arguments to `tauri build`.

Examples:

```bash
./scripts/build-desktop.sh -- --bundles dmg
./scripts/build-desktop.sh --skip-install
```

Current build outputs:

- `src-tauri/target/release/bundle/macos/TogoClient.app`
- `src-tauri/target/release/bundle/dmg/TogoClient_0.2.0_aarch64.dmg`

## Notes

- The desktop app defaults to `http://127.0.0.1:8080`.
- If `TOGOSPACE_ROOT` is not set, packaged desktop builds first use the internal `extensions/TogoSpace`, while source development falls back to the parent `TogoSpace` repository and legacy fallback paths.
- Desktop development only prepares an empty `src-tauri/resources/extensions/TogoSpace` placeholder directory to satisfy Tauri resources without copying the repo.
- Desktop packaging stages a filtered copy of the parent `TogoSpace` repository into `src-tauri/resources/extensions/TogoSpace`, excluding the root `frontend/` tree, and the packaged app resolves it from the app resources directory.
- In source development mode, desktop-triggered backend startup writes `setting.json`, `run/backend.pid`, logs, and database files under `frontend/.desktop-dev-storage/`.
- In packaged desktop mode, API requests go through a native Tauri HTTP bridge so the app can talk to the local backend without modifying `TogoSpace` CORS behavior.
- In packaged desktop mode, real-time events use a native Tauri bridge to avoid macOS WebView WebSocket origin restrictions.
- JSON write requests use a no-preflight content type so they can talk to the local Tornado backend without changing `TogoSpace`.
- The tray menu can show the main window, start or stop the backend, open the local `TogoSpace` directory, and quit.
- The in-app desktop status button hides backend controls behind a compact menu and shows a red / green badge for backend connectivity.
- Dropping files into the composer inserts absolute paths when the runtime exposes them, and falls back to file names otherwise.
- Local absolute paths rendered in chat, including inline-code paths, are recognized by regex or inline path detection, wrapped in subtle path chips, and provide Finder reveal plus copy actions on macOS desktop builds.
- Project-local cargo mirror config lives in `.cargo/config.toml`.
