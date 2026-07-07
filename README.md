# Local Context Bridge

Local Context Bridge is a Windows-first desktop tool for moving local TRAE SOLO CN conversation context between accounts on the same machine.

It is designed for data portability: when you switch accounts but still have useful local conversation context, the app helps you discover the local database, inspect non-deleted conversations, preview a transfer plan, back up the live database, and reattach selected projects or sessions to the current local account.

The tool runs locally. It does not upload databases, messages, logs, account IDs, or telemetry.

> Local Context Bridge is an independent community tool. It is not affiliated with, endorsed by, or maintained by TRAE.

## Features

- Auto-discovers TRAE SOLO CN installation and local data paths.
- Lets users manually browse for `TRAE SOLO CN.exe` or `database.db` when automatic discovery fails.
- Detects current account candidates from local logs.
- Lists database users, projects, sessions, message counts, work mode, and update time.
- Supports project-level transfer.
- Supports session-level transfer with project reuse or project cloning when only part of a project is selected.
- Excludes deleted sessions and orphaned history rows by default.
- Requires preview before writeback.
- Backs up `database.db`, `database.db-wal`, and `database.db-shm` before applying changes.
- Recalculates encrypted-page HMAC after writeback.
- Provides rollback and frontend-log verification entry points.

## Safety Model

Local Context Bridge only works with local files on the user's own machine.

The first release follows these rules:

- Deleted conversations are not restored.
- Only `chat_session.deleted_at = 0` sessions are eligible.
- The app does not edit message, turn, or history IDs.
- Writeback requires TRAE SOLO CN to be closed.
- Every apply operation creates a backup first.
- Preview mode patches a temporary database copy before any live write.
- No network calls or telemetry are made by the app itself.

You are responsible for using this tool only on local data you are allowed to access.

## Screens

The main window is the product surface. There is no landing page:

- Top bar: app path, database path, current user, status.
- Left rail: database users and local conversation counts.
- Center table: non-deleted sessions grouped by project metadata.
- Right panel: target account, transfer mode, selected counts, preview actions, warnings.
- Bottom actions: rescan, close TRAE, preview transfer, apply transfer, verify frontend, open backup, rollback.

If automatic discovery cannot find the app or database, the top cards switch to `浏览选择` so users can browse for the correct files.

## Discovery Logic

The app path is discovered from:

1. Running `TRAE SOLO CN` process path.
2. Windows uninstall registry entries.
3. Common install locations such as Program Files and local app folders.
4. A manual `TRAE SOLO CN.exe` picker.

The database path is discovered from:

1. `%APPDATA%\TRAE SOLO CN\ModularData\ai-agent\database.db`.
2. A manual `database.db` picker.

When a user manually picks a database matching:

```text
...\TRAE SOLO CN\ModularData\ai-agent\database.db
```

the app derives the matching app-data and log directories from that path.

## Transfer Behavior

Project mode updates the owner user for selected projects.

Session mode moves only selected sessions:

- If all active sessions under a project are selected, the operation becomes a project-level transfer.
- If only part of a project is selected, the app reuses a matching target-user project or clones the project row, then moves the selected sessions to that target project.
- `chat_message`, `chat_turn`, and `history_v2` keep their original `session_id`; they follow the moved session naturally.

## Install

Download the Windows installer from GitHub Releases when available.

For a portable run, advanced users can run the release `local-context-bridge.exe` directly. WebView2 Runtime is required; most Windows 10/11 systems already include it.

Because early builds are unsigned, Windows SmartScreen may show a warning.

## Development

Prerequisites:

- Windows 10/11
- Node.js 22+
- Rust stable
- WebView2 Runtime

Install dependencies:

```powershell
npm install
```

Run the frontend preview:

```powershell
npm run dev
```

Run the desktop app:

```powershell
npm run tauri:dev
```

Run Rust tests:

```powershell
cargo test -p trae_core
```

Build the frontend:

```powershell
npm run build
```

Build Windows bundles:

```powershell
npm run tauri:build
```

## Repository Layout

```text
.
├─ src/                 React UI
├─ src-tauri/           Tauri commands and Windows packaging
├─ crates/trae_core/    Rust discovery, database, backup, transfer, and crypto logic
├─ docs/                Architecture notes
└─ .github/workflows/   Release build workflow
```

## Status

This is an early `0.1.0` release focused on Windows + TRAE SOLO CN.

Known constraints:

- The schema and local database format may change when TRAE updates.
- Real-data writeback should be tested carefully with backups.
- The app is currently provider-specific, although the architecture leaves room for additional providers later.
- Code signing is not configured yet.

## License

Apache-2.0
