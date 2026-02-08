# AGENTS.md

Guidelines for agentic coding agents working on Runner UI — a Tauri 2 desktop app (Rust backend + Vue 3 frontend).

## Build Commands

```bash
# Full app (frontend + Tauri backend)
pnpm tauri dev      # Development mode
pnpm tauri build    # Production build

# Frontend only (Vite dev server on port 1420)
pnpm dev
pnpm build          # Type-check with vue-tsc, then build

# Rust backend only
cd src-tauri && cargo check
cd src-tauri && cargo build
cd src-tauri && cargo clippy
```

**Package manager:** pnpm (v10.18.3)  
**No test suite exists** — tests are not required for this codebase.

## Code Style

### TypeScript / Vue 3

- **Framework:** Vue 3 with Composition API and `<script setup>` syntax
- **Style:** Tailwind CSS v4 with dark theme throughout
- **State:** Pinia stores in `src/stores/`
- **Types:** Mirror Rust models in `src/types/index.ts`

**Naming:**
- camelCase for variables, functions, properties
- PascalCase for components, interfaces, types
- SCREAMING_SNAKE_CASE for constants

**Imports:**
- Group: external libs (Vue, Tauri), then internal (@/stores, @/types), then components
- Use `@/` prefix for project imports (configured in vite.config.ts)

**Tauri Integration:**
- Commands: `invoke("command_name", { args })`
- Events: `listen<T>("event-name", callback)` with typed payloads
- Key events: `process-status-changed`, `process-log`, `process-stats-updated`

**Formatting:**
- No Prettier/ESLint configured — maintain consistency with existing files
- 2-space indentation
- Semicolons required
- Single quotes for strings

### Rust

**Naming:**
- snake_case for functions, variables, modules
- PascalCase for structs, enums, traits
- SCREAMING_SNAKE_CASE for constants

**Error Handling:**
- Use custom `Error` enum in `src-tauri/src/error.rs`
- Derive with `thiserror::Error`
- Manual `Serialize` impl to return string messages to frontend
- Prefer `?` operator for error propagation

**Serde:**
- Always use `#[serde(rename_all = "camelCase")]` for JS compatibility
- Match TypeScript interface names in `src/types/index.ts`

**State Management:**
- All shared state in `AppState` struct (`src-tauri/src/state.rs`)
- Access via `state.lock().unwrap()` (std Mutex, not tokio)
- Commands receive `State<Arc<AppState>>` and `AppHandle` parameters
- App state managed via `app.manage(Arc<AppState>)`

**Tauri Commands:**
- Organize by domain in `src-tauri/src/commands/`
- Register in `lib.rs` via `tauri::generate_handler![]`
- Return `Result<T, Error>` for error handling

**Formatting:**
- No rustfmt.toml or clippy.toml present
- Follow standard Rust conventions
- 4-space indentation

## Architecture Patterns

### Frontend → Backend Communication

1. **Commands (Request/Response):**
   - Vue calls `invoke("command", args)`
   - Rust `#[tauri::command]` functions process and return

2. **Events (Backend → Frontend):**
   - Rust: `app_handle.emit("event", &payload)`
   - Vue: `listen<T>("event", handler)` from `@tauri-apps/api/event`

### Project Structure

```
/src                    # Vue frontend
  /components           # Vue SFCs
    /sidebar            # GroupItem, ProjectItem
    /main               # ProjectDetail, LogPanel, MonitorGraph
    /shared             # Dialogs, form components
  /stores               # Pinia stores
  /types                # TypeScript interfaces

/src-tauri/src          # Rust backend
  /commands             # Tauri command handlers
  lib.rs                # App setup, command registration
  state.rs              # AppState with shared mutable state
  models.rs             # Data structures with serde derives
  error.rs              # Error enum
  process_manager.rs    # Process spawning and management
  stats_collector.rs    # CPU/memory monitoring
  database.rs           # SQLite operations
  storage.rs            # JSON file persistence
```

### Key Implementation Notes

- Config auto-saves to disk after every mutation
- Processes spawned via `sh -c "<command>"` with `setpgid(0,0)` for process groups
- Logs dual-written: emitted as Tauri events + stored in SQLite
- Database at `~/.config/runner-ui/runner-ui.db` in WAL mode
- Stats collected every 2 seconds via tokio task
- Graceful shutdown: SIGTERM → 5s timeout → SIGKILL on process group

## Task Completion

When implementing features:
1. Update **FEATURES.md** with what was added
2. Update **PENDING.md** (mark Done or remove completed items)
3. Run `pnpm build` to verify TypeScript compilation
4. Run `cd src-tauri && cargo clippy` to check Rust code
