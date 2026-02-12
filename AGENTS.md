# AGENTS.md

Guidelines for agentic coding agents working on Runner UI — an Electron desktop app (Node.js/TypeScript backend + Vue 3 frontend).

## Build Commands

```bash
# Full app (frontend + Electron backend)
pnpm dev            # Development mode with hot reload
pnpm build          # Production build

# Frontend only (Vite dev server)
pnpm frontend:dev   # Start Vite dev server
pnpm frontend:build # Build frontend only

# Type checking
pnpm typecheck      # Type-check Electron backend

# Native module rebuild
pnpm rebuild        # Rebuild native modules (better-sqlite3, node-pty)
```

**Package manager:** pnpm (v10.18.3)  
**No test suite exists** — tests are not required for this codebase.

## Code Style

### TypeScript / Vue 3

- **Framework:** Vue 3 with Composition API and `<script setup>` syntax
- **Style:** Tailwind CSS v4 with dark theme throughout
- **State:** Pinia stores in `src/stores/`
- **Types:** Shared types in `src-electron/shared/types.ts` and `src/types/index.ts`

**Naming:**
- camelCase for variables, functions, properties
- PascalCase for components, interfaces, types
- SCREAMING_SNAKE_CASE for constants

**Imports:**
- Group: external libs (Vue, Electron), then internal (@/stores, @/types), then components
- Use `@/` prefix for project imports (configured in electron.vite.config.ts)

**Electron IPC Integration:**
- Commands: `invoke("command-name", { args })` via `@/lib/api`
- Events: `listen<T>("event-name", callback)` via `@/lib/api`
- Key events: `process-status-changed`, `process-log`, `process-stats-updated`

**Formatting:**
- No Prettier/ESLint configured — maintain consistency with existing files
- 2-space indentation
- Semicolons required
- Single quotes for strings

**Known Bugs:**
- The `Switch` component should use the `model-value` with `@update:model-value` instead of `checked` with `@update:checked`.

### TypeScript Backend (Electron Main Process)

**Naming:**
- camelCase for functions, variables, properties
- PascalCase for classes, interfaces, types
- SCREAMING_SNAKE_CASE for constants

**Error Handling:**
- Use `ElectronError` class in `src-electron/shared/errors.ts`
- Always wrap IPC handlers with try/catch
- Return meaningful error messages to frontend

**IPC Handlers:**
- Handlers grouped by domain in `src-electron/main/ipc/`
- Use kebab-case for IPC channel names (e.g., `get-groups`, `start-process`)
- Channel names defined in `src-electron/shared/events.ts`

**State Management:**
- All shared state in `AppState` singleton (`src-electron/main/services/state.ts`)
- Access via `getState()` function
- Database operations via `state.database` methods

## Architecture Patterns

### Frontend → Backend Communication

1. **Commands (Request/Response):**
   - Vue calls `invoke("command-name", args)` from `@/lib/api`
   - Electron `ipcMain.handle()` processes and returns

2. **Events (Backend → Frontend):**
   - Electron: `mainWindow.webContents.send("event-name", payload)`
   - Vue: `listen<T>("event-name", handler)` from `@/lib/api`

### YAML Sync Rule (MANDATORY)

**When adding ANY new feature that modifies groups or projects, you MUST ensure YAML sync is handled:**

Groups can have `syncEnabled: true` which means all modifications must be written to the `openrunner.yaml` file. The file watcher prevents infinite loops by tracking timestamps.

**Operations requiring YAML sync:**
- Group: create, rename, update directory, update env vars
- Project: create, update, delete (single or multiple), convert type
- Import/Export already handle this

**Pattern for IPC handlers:**
```typescript
// 1. Load group from database
// 2. Perform database operation
// 3. Update local group object
// 4. Call yaml-config function to sync
syncYaml(group, state);
// OR for specific operations:
updateGroupInYaml(group, state);
addProjectToYaml(group, project, state);
updateProjectInYaml(group, project, state);
removeProjectFromYaml(group, projectId, state);
```

**Pattern for TypeScript config store:**
- Never call database services directly for mutations
- Always call IPC handlers via `invoke()`
- This ensures proper YAML sync handling

### Project Structure

```
/src                      # Vue 3 frontend
  /components             # Vue SFCs
    /sidebar              # GroupItem, ProjectItem
    /main                 # ProjectDetail, LogPanel, MonitorGraph
    /shared               # Dialogs, form components
  /lib                    # API bridges for Electron
    api.ts                # invoke, listen, once, platform
    dialog.ts             # File dialogs
    shell.ts              # Open external URLs/files
    os.ts                 # OS type detection
  /stores                 # Pinia stores
  /types                  # TypeScript interfaces

/src-electron             # Electron backend
  /main                   # Main process code
    index.ts              # App entry, window creation
    /ipc                  # IPC handlers (grouped by domain)
      groups.ts           # Group CRUD operations
      projects.ts         # Project CRUD operations
      processes.ts        # Process start/stop/restart
      sessions.ts         # Session queries
      settings.ts         # Settings CRUD
      storage.ts          # Storage stats/cleanup
      files.ts            # File operations
    /services             # Core services
      database.ts         # SQLite via better-sqlite3
      process-manager/    # Process spawning with node-pty
      yaml-config.ts      # YAML read/write
      yaml-watcher.ts     # File watching with chokidar
      stats-collector.ts  # CPU/memory stats via pidusage
      state.ts            # AppState singleton
    /platform             # Platform-specific code
      linux.ts            # Linux process management
      macos.ts            # macOS process management
      windows.ts          # Windows process management
  /preload                # Preload scripts
    index.ts              # Context bridge for renderer
  /shared                 # Shared code between main/preload
    types.ts              # Type definitions
    events.ts             # IPC channel/event definitions
    errors.ts             # Error classes
```

### Module Organization Rules

**IPC Handlers:** Group related handlers in single files by domain (groups, projects, processes, etc.)

**Services:** Each service is a separate file or folder:
- Simple services: single file (e.g., `stats-collector.ts`)
- Complex services: folder with submodules (e.g., `process-manager/`)

**Module Structure Pattern for complex services:**
```
src-electron/main/services/<service>/
  index.ts            # Public API exports
  <submodule>.ts      # Individual concerns
```

### Key Implementation Notes

- Config auto-saves to disk after every mutation
- Processes spawned via `sh -c "<command>"` with `setpgid(0,0)` for process groups (Linux/macOS)
- Logs dual-written: emitted as IPC events + stored in SQLite
- Database at `~/.config/openrunner/runner-ui.db` in WAL mode
- Stats collected every 2 seconds via setInterval
- Graceful shutdown: SIGTERM → 5s timeout → SIGKILL on process group

## Task Completion

When implementing features:
1. Update **FEATURES.md** with what was added
2. Update **PENDING.md** (mark Done or remove completed items)
3. Run `pnpm build` to verify TypeScript compilation and Electron build
