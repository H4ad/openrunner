# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is OpenRunner

A desktop process manager for local development, built with Tauri 2 (Rust backend + Vue 3 frontend). It lets users organize shell commands into groups, run/stop/restart them, view real-time logs via xterm.js, and monitor CPU/memory usage.

## Development Commands

```bash
# Run the full app in dev mode (starts both Vite frontend and Tauri backend)
pnpm tauri dev

# Build the production app
pnpm tauri build

# Frontend only (Vite dev server on port 1420, no Tauri shell)
pnpm dev

# Type-check frontend
pnpm build   # runs vue-tsc --noEmit && vite build

# Rust backend only (check/build/clippy)
cd src-tauri && cargo check
cd src-tauri && cargo build
cd src-tauri && cargo clippy
```

Package manager: **pnpm** (v10.18.3). No test suite exists currently.

## Architecture Overview

### Frontend ↔ Backend Communication

The app uses two communication channels between Vue and Rust:

1. **Tauri commands** (request/response): Vue calls `invoke("command_name", { args })` → Rust `#[tauri::command]` functions in `src-tauri/src/commands/`. Commands are registered in `lib.rs` via `tauri::generate_handler![]`.

2. **Tauri events** (push from backend): Rust emits events via `app_handle.emit("event-name", &payload)`, Vue listens via `listen<T>("event-name", callback)`. Key events:
   - `process-status-changed` — emitted when a process starts/stops/errors
   - `process-log` — emitted for each stdout/stderr chunk from a managed process
   - `process-stats-updated` — emitted every 2s with CPU/memory stats for all running processes

### Backend (Rust — `src-tauri/src/`)

- **`lib.rs`** — App setup: initializes plugins, loads config, creates SQLite DB, starts stats collector, registers all Tauri commands, handles `RunEvent::Exit` cleanup.
- **`state.rs`** — `AppState` holds all shared mutable state behind `Mutex`: config, running processes (`HashMap<project_id, ManagedProcess>`), process info, SQLite connection, active sessions. Managed via `app.manage(Arc<AppState>)`.
- **`models.rs`** — Shared data types (Group, Project, ProcessInfo, LogMessage, Session, MetricPoint, etc.). All use `#[serde(rename_all = "camelCase")]` to match JS conventions.
- **`process_manager.rs`** — Spawns processes via `sh -c`, sets up stdout/stderr readers that emit `process-log` events, handles graceful shutdown (SIGTERM → 5s timeout → SIGKILL on process group), auto-restart logic, and exit watching.
- **`stats_collector.rs`** — Background tokio task that runs every 2s, refreshes sysinfo with `ProcessesToUpdate::All`, aggregates CPU/memory across the process tree (BFS), writes metrics to SQLite, and emits `process-stats-updated`.
- **`database.rs`** — SQLite schema (sessions, logs, metrics tables) and all DB operations. Uses WAL mode. DB stored at `~/.config/runner-ui/runner-ui.db`.
- **`storage.rs`** — JSON file persistence for `config.json` and `settings.json` in the app data directory.
- **`error.rs`** — `Error` enum with `thiserror` + manual `Serialize` impl (serializes as string for Tauri).
- **`commands/`** — Tauri command handlers organized by domain: `groups.rs`, `projects.rs`, `processes.rs`, `logs.rs`, `sessions.rs`, `settings.rs`.

### Frontend (Vue 3 — `src/`)

- **Stores (Pinia — `src/stores/`)**: Each store wraps Tauri `invoke` calls and event listeners.
  - `config.ts` — Groups/projects CRUD
  - `processes.ts` — Process start/stop/restart + listens to status/stats events
  - `logs.ts` — In-memory log buffer per project, listens to `process-log` events
  - `sessions.ts` — Historical session data
  - `settings.ts` — App settings (max log lines)
  - `ui.ts` — UI state: selected group/project, view mode (`project | groupMonitor | settings | sessionDetail`), sidebar expansion

- **Components (`src/components/`)**:
  - `sidebar/` — Sidebar, GroupItem, ProjectItem
  - `main/` — MainPanel (view router based on `viewMode`), ProjectDetail, LogPanel (xterm.js), MonitorGraph (chart.js), GroupMonitor, SessionsList, SessionDetail, SettingsPage
  - `shared/` — Reusable dialogs and widgets (ProjectFormDialog, ConfirmDialog, EditDialog, EnvVarsEditor, StatusBadge, EmptyState)

- **Types (`src/types/index.ts`)**: TypeScript interfaces mirroring Rust models (camelCase).

### Data Model

Groups contain Projects. Each Project has an ID, name, shell command, optional env vars, optional CWD override, and auto-restart flag. When a process starts, a Session is created in SQLite to track logs and metrics historically.

### Key Patterns

- All Rust shared state access goes through `state.lock().unwrap()` (std Mutex, not tokio Mutex).
- Tauri commands receive `State<Arc<AppState>>` and `AppHandle` as injected parameters.
- Config is auto-saved to disk after every mutation in command handlers.
- Process commands run via `sh -c "<command>"` with `setpgid(0,0)` for process group management.
- Log output is dual-written: emitted as events for real-time display AND stored in SQLite for persistence.
- Frontend uses Tailwind CSS v4 (via `@tailwindcss/vite` plugin), dark theme throughout.

## FEATURES.md and PENDING.md Management

- **Always update FEATURES.md** when implementing a new feature. Add a clear description of what was added under the appropriate section.
- **Always update PENDING.md** when:
  - Completing a pending task: change its status to "Done" or remove it from the list.
  - Discovering new work that needs to be done: add it as a new pending item.
  - A task is partially complete: update its status and description to reflect current state.
- Read both files at the start of a session to understand existing features and pending work before making changes.
