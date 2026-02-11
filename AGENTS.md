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

**Known Bugs:**
- The `Switch` component should use the `model-value` with `@update:model-value` instead of `checked` with `@update:checked`.

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
- One command = One file (mandatory - no domain grouping)
- Location: `src-tauri/src/commands/<command_name>.rs`
- Register in `lib.rs` via `tauri::generate_handler![]`
- Return `Result<T, Error>` for error handling

### Rust Command Structure Rule

**Principle:** One command = One file. Never group multiple commands in a single file.

**File Organization:**
```
src-tauri/src/commands/
├── types.rs              # Shared types (re-exports from models, errors)
├── utils.rs              # Helper functions used by multiple commands
├── get_groups.rs         # Individual command file
├── create_group.rs       # Individual command file
├── ...                   # One file per command (47 total commands)
```

**Naming Convention:**
- File: `snake_case.rs` matching command function name
- Function: `pub fn command_name` with `#[tauri::command]`
- Registration: `commands::<command_name>::<command_name>,` in lib.rs

**Example:**
```rust
// src/commands/get_groups.rs
use crate::commands::types::{Error, Group};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_groups(state: State<'_, Arc<AppState>>) -> Result<Vec<Group>, Error> {
    // implementation
}
```

**Registration in lib.rs (grouped by domain):**
```rust
.invoke_handler(tauri::generate_handler![
    // Group commands
    commands::get_groups::get_groups,
    commands::create_group::create_group,
    // ... etc
])
```

**Helper Functions:**
- Place shared helpers in `commands/utils.rs`
- Import with `use crate::commands::utils::helper_function;`
- Keep imports explicit (no prelude module)

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

### YAML Sync Rule (MANDATORY)

**When adding ANY new feature that modifies groups or projects, you MUST ensure YAML sync is handled:**

Groups can have `sync_enabled: true` which means all modifications must be written to the `openrunner.yaml` file. The file watcher prevents infinite loops by tracking timestamps.

**Operations requiring YAML sync:**
- Group: create, rename, update directory, update env vars
- Project: create, update, delete (single or multiple), convert type
- Import/Export already handle this

**Pattern for Rust commands:**
```rust
// 1. Load group from database
// 2. Perform database operation
// 3. Update local group object
// 4. Call yaml_config function to sync
yaml_config::sync_yaml(&group, &state)?;
// OR for specific operations:
yaml_config::update_group_in_yaml(&group, &state)?;
yaml_config::add_project_to_yaml(&group, &project, &state)?;
yaml_config::update_project_in_yaml(&group, &project, &state)?;
yaml_config::remove_project_from_yaml(&group, &project_id, &state)?;
```

**Pattern for TypeScript config store:**
- Never call database services directly for mutations
- Always call Rust commands via `invoke()`
- This ensures proper YAML sync handling

**New commands requiring sync:**
- File location: `src-tauri/src/commands/<command_name>.rs`
- Must update `commands/mod.rs` and `lib.rs`
- Return updated `Group` so frontend state can be updated

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
  /commands             # Tauri command handlers (one file per command)
    types.rs            # Shared type re-exports
    utils.rs            # Command helper functions
  /process              # Process management module
    mod.rs              # Public API exports
    platform.rs         # Platform abstraction
    spawn.rs            # Process spawning (regular & interactive)
    lifecycle.rs        # Start/stop/kill operations
    watcher.rs          # Exit monitoring & auto-restart
    io.rs               # PTY I/O operations
  /cli                  # CLI functionality
    /detector           # Project auto-detection
      mod.rs            # detect_projects orchestrator
      npm.rs            # package.json detection
      makefile.rs       # Makefile detection
      configs.rs        # Docker Compose, Justfile, etc.
      languages.rs      # Python, Rust, Go, Docker detection
    commands.rs         # CLI command implementations
    ui.rs               # CLI user interface helpers
  /database             # Database operations
    mod.rs              # Public API exports
    schema.rs           # Table creation & migrations
    sessions.rs         # Session CRUD operations
    logs.rs             # Log storage & retrieval
    metrics.rs          # Metrics storage & queries
    maintenance.rs      # Cleanup & statistics
  lib.rs                # App setup, command registration
  state.rs              # AppState with shared mutable state
  models.rs             # Data structures with serde derives
  error.rs              # Error enum
  stats_collector.rs    # CPU/memory monitoring
  storage.rs            # JSON file persistence
```

### Module Organization Rules

**Large Modules (300+ lines) must be split:**
- `process_manager.rs` → `src/process/` (5 submodules)
- `cli/detector.rs` → `src/cli/detector/` (4 submodules)
- `database.rs` → `src/database/` (5 submodules)

**Module Structure Pattern:**
```
src/<module>/
  mod.rs              # Public API re-exports
  <submodule>.rs      # Individual concerns
```

**Re-export Pattern (mod.rs):**
```rust
pub mod schema;
pub mod sessions;

// Re-export for convenience
pub use schema::init_database;
pub use sessions::{create_session, end_session};
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
