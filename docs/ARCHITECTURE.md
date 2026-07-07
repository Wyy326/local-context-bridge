# Architecture

Local Context Bridge has three layers:

- React UI for selection, preview, and status.
- Tauri commands for desktop integration and short-lived plan storage.
- Rust `trae_core` for database discovery, analysis, migration planning, backups, and SQLCipher page handling.

## Data Flow

1. `scan_installations` discovers app/cache/log paths and current user candidates.
2. `analyze_database` copies the database to a workspace, checkpoints WAL frames into the copy, decrypts it when needed, and queries account/session summaries.
3. `preview_transfer` patches a plaintext copy only, computes changed pages, and returns a plan.
4. `apply_transfer` requires TRAE to be closed, backs up live files, checkpoints WAL into the live database, patches a plaintext copy, encrypts changed pages back into the live database, recalculates HMAC, and writes a JSON report.
5. `verify_frontend` scans the latest TRAE logs for list-session success lines and malformed database errors.

## Storage

- Config: `%APPDATA%\Local Context Bridge\config.json` in a later settings pass.
- Workspace: `%LOCALAPPDATA%\Local Context Bridge\workspace`.
- Backups: `%USERPROFILE%\Documents\Local Context Bridge\Backups`.

## Security Model

The app runs entirely locally and does not make network requests. Database files remain on disk. The UI never displays full message content in the first release; it only shows metadata needed to choose sessions safely.
