# Pending

## 1. Clickable Links in Logs Tab
- **Status:** Done
- **Description:** Render clickable links inside the logs tab (xterm.js). URLs in log output are detected via `@xterm/addon-web-links` and opened in the default browser using `@tauri-apps/plugin-shell`.

## 2. Process Survival / Reattach on Restart
- **Status:** Done
- **Description:** Child processes receive SIGTERM when the main process dies via `PR_SET_PDEATHSIG` on Linux. All started processes are killed on graceful exit via `RunEvent::Exit`. Approach: kill-on-exit (not reattach).

## 3. Fix CPU/Memory Detection Bug
- **Status:** Done
- **Description:** Fixed by using `ProcessesToUpdate::All` instead of `ProcessesToUpdate::Some` in the stats collector. The previous approach only refreshed known root PIDs and missed child/descendant processes that weren't previously tracked by sysinfo.

## 4. Custom Environment Variables per Group and per Project
- **Status:** Done
- **Description:** Added `env_vars` field to Group model. Created EnvVarsEditor component for managing key-value pairs. Project env vars editable in ProjectFormDialog. Group env vars editable via context menu. When spawning, group env vars are used as base and project env vars override them.

## 5. Search Logs (Ctrl+F)
- **Status:** Done
- **Description:** Integrated `@xterm/addon-search` into LogPanel. Added search bar with input field, next/prev/close buttons. Toggle via Ctrl+F keyboard shortcut or search icon button. Enter/Shift+Enter for next/previous match. Escape to close.

## 6. CPU/Memory Monitoring Over Time (Project Graph)
- **Status:** Done
- **Description:** Added chart.js and vue-chartjs. Created MonitorGraph component with real-time CPU and memory line charts. Toggle via Monitor button in project detail header. Charts update live every 2 seconds. Metrics stored in SQLite for historical analysis.

## 7. Group-Level Monitoring Dashboard
- **Status:** Done
- **Description:** Created GroupMonitor component showing all projects in a group with status badges, CPU/memory stats, and mini SVG sparkline charts. Accessible from group context menu ("Monitor"). Click project cards to navigate to detail view.

## 8. SQLite Storage for Logs and Metrics
- **Status:** Done
- **Description:** Added rusqlite with bundled feature. Created database.rs with schema for sessions, logs, and metrics tables. Logs and metrics written to SQLite in real-time. Sessions created on process start, ended on exit. Database stored at ~/.config/runner-ui/runner-ui.db. Backward-compatible fallback to temp file logs.

## 9. Sessions (Historical Logs/Metrics)
- **Status:** Done
- **Description:** Added sessions Tauri commands and Pinia store. Created SessionsList component showing session history with timestamps, duration, and exit status. Created SessionDetail component to view historical logs in read-only xterm.js terminal. Sessions button added to project detail header.

## 10. Configuration Full Page with Storage Management
- **Status:** Done
- **Description:** Created SettingsPage component replacing the old SettingsDialog. Shows general settings (max log lines) and storage management section with total size, session/log/metric counts, cleanup by age, and clear all data button. Settings gear button in sidebar now navigates to full settings page.

## 27. Home Dashboard Overview
- **Status:** Done
- **Description:** Added a full-width Home view that replaces the sidebar when active. Includes quick actions, health summary, storage stats, global CPU/MEM metrics with sparklines, recent activity, and per-group overview cards with Start All actions.

## 11. File Path Detection & Opening in Logs
- **Status:** Done
- **Description:** Added custom xterm.js link provider to detect file paths (with optional line:column) in log output. Paths are resolved against the project's working directory. Clicking opens the file in the user's editor ($VISUAL/$EDITOR), with line/column support for VS Code, Sublime, vim/nvim, JetBrains IDEs, and Emacs. Falls back to xdg-open/open for unknown editors.

## 12. Monitoring Icon in Group Sidebar
- **Status:** Done
- **Description:** Added a visible monitoring icon button to each group in the sidebar showing running/total project count (e.g. "2/5"). Green tint when projects are running, gray otherwise. Clicking opens the Group Monitor dashboard. Only shown for groups with projects.

## 13. Enhanced Group Monitor Dashboard
- **Status:** Done
- **Description:** Enhanced GroupMonitor to show PID, auto-restart badge, and recent log output preview (last 5 lines) for running projects. Stopped/errored projects now show last session info (timestamp, duration, exit status), last known CPU/MEM, and last log output. Added "No session history" fallback for never-run projects.

## 14. Last Session Info for Stopped Projects
- **Status:** Done
- **Description:** ProjectDetail now shows last session data (last run time, duration, exit status, last known CPU/MEM) when a project is not running, instead of showing nothing below the process controls.

## 15. Group Navbar and Start/Stop All
- **Status:** Done
- **Description:** Added group navigation bar at the top of the project detail page showing breadcrumb (Group / Project), running count badge, Start All / Stop All button, and link to group monitor. Added Start All / Stop All button to the group monitor header. Added individual play/stop buttons on each project card in the monitor dashboard.

## 16. Fix CPU/Memory Stats Accuracy
- **Status:** Done
- **Description:** CPU usage now normalized by logical CPU count (divides aggregate per-core usage by `available_parallelism()`) so values stay in 0-100% range. Memory capped at total system memory to prevent misleading values from RSS shared-page double-counting across process trees.

## 17. Session Detail Charts (Historical CPU/Memory)
- **Status:** Done
- **Description:** SessionDetail now fetches and displays historical CPU/memory charts using chart.js. Charts are collapsible via a toggle button (default: open). Shows all data points from the session's metrics.

## 18. Monitor Toggle Persists Across Projects
- **Status:** Done
- **Description:** Moved `showMonitor` from local ref in ProjectDetail to the UI store, so it persists when switching between projects. Monitor graph now shows for stopped projects too, loading historical metrics from the last session.

## 19. Session List Metadata (Size + Duration)
- **Status:** Done
- **Description:** Added `SessionWithStats` model and `get_project_sessions_with_stats` backend command that JOINs sessions with aggregated log/metric stats. SessionsList now displays log count, log data size (formatted as KB/MB), and metric count per session.

## 20. Ctrl+F Search in Session Detail Logs
- **Status:** Done
- **Description:** Added search bar UI and Ctrl+F keyboard handler to SessionDetail, matching LogPanel's search functionality. Includes input field, next/prev/close buttons, Enter/Shift+Enter navigation, and Escape to close.

## 21. File Path Detection in Session Detail Logs
- **Status:** Done
- **Description:** Added WebLinksAddon for clickable URLs and custom file path link provider to SessionDetail, matching LogPanel's implementation. File paths with line:column references are detected and open in the user's editor.

## 22. Directory Path Detection in Logs
- **Status:** Done
- **Description:** Added regex pattern to detect absolute directory paths (e.g., `/home/user/project`) in both LogPanel and SessionDetail. Clicking opens the directory in the system file manager.

## 23. Configurable Default Editor
- **Status:** Done
- **Description:** Added `editor` field to AppSettings. SettingsPage now has an Editor section where users can configure their preferred editor. Auto-detects system editor from $VISUAL/$EDITOR or common editors (code, cursor, vim, nvim, zed, etc.). The `open_file_in_editor` command now uses the configured editor with proper line/column support for each editor type.

## Recently Completed

### Platform Abstraction Layer
- **Status:** Done
- **Description:** Refactored native process management code into a cross-platform abstraction layer (`src/platform/`). Split implementations into OS-specific subfolders: `linux.rs`, `macos.rs`, and `windows.rs`. Added Windows support using Windows Job Objects for process lifecycle management and CREATE_NEW_PROCESS_GROUP for graceful signal handling.

### Fix CPU/Memory Stats Collection
- **Status:** Done
- **Description:** Fixed stats collection to aggregate CPU and memory across the entire process tree (parent shell + all child processes), not just the shell PID.

### Resizable Sidebar and Main Content
- **Status:** Done
- **Description:** Added drag handle between sidebar and main content. Sidebar resizable between 180px and 480px.

### CPU/Memory Info in Sidebar
- **Status:** Done
- **Description:** CPU and memory usage now shown inline next to project names in the sidebar when the process is running.

### Folder Search/Select for Group Creation
- **Status:** Done
- **Description:** Added native folder picker button (via tauri-plugin-dialog) to the working directory field in group creation and edit dialogs.

### CWD Override per Project (Monorepo Support)
- **Status:** Done
- **Description:** Added optional CWD field when creating/editing projects. Supports relative paths (resolved against group directory) and absolute paths.

### Settings Button for Log Line Limit
- **Status:** Done
- **Description:** Added settings gear button in sidebar header. Settings dialog allows configuring max log lines (1k-100k, default 10k). Settings persisted to disk.

### Persist Logs When Switching Projects
- **Status:** Done
- **Description:** Logs are now written to temp files on disk. When switching back to a project, logs are loaded from disk if the in-memory buffer is empty. Clear button also clears disk cache.

### ANSI/Syntax Highlighting in Logs
- **Status:** Done
- **Description:** xterm.js already supports ANSI codes natively. Added FORCE_COLOR=1 and CLICOLOR_FORCE=1 environment variables to spawned processes so tools emit colored output even when piped.

## 24. GitHub Actions CI/CD
- **Status:** Done
- **Description:** Created GitHub Actions workflow for automated building and releasing. Builds triggered on pushes to main/master and version tags (v*). Supports macOS (Apple Silicon + Intel), Windows, and Linux. Creates GitHub releases with artifacts when version tags are pushed.

## 25. YAML Auto-Sync for Groups
- **Status:** Done
- **Description:** Created new Rust commands (rename_group, update_group_directory, update_group_env_vars, create_project, update_project, delete_project, delete_multiple_projects, convert_multiple_projects) that perform database operations and sync changes to openrunner.yaml when sync_enabled is true. Updated src/stores/config.ts to call Rust commands instead of direct database calls. Added YAML Sync Rule documentation to AGENTS.md to ensure future features handle YAML sync.

## 26. Rust-Only Data Access for UI
- **Status:** Done
- **Description:** Moved UI reads to Rust commands (sessions, logs, metrics, settings) and removed tauri-plugin-sql along with unused frontend database services to eliminate duplicate data access paths.
