# Features

## Group Management
- Create groups with a name and working directory
- Rename groups via right-click context menu
- Change group working directory via right-click context menu (with folder picker)
- Delete groups (with confirmation dialog)
- Expand/collapse groups in the sidebar to show/hide projects
- Native folder picker dialog for selecting working directories
- Custom environment variables per group (inherited by all projects in the group)

## Project Management
- Create projects within a group with a name and shell command
- Edit project name, command, and working directory
- Delete projects (with confirmation dialog, stops running process first)
- Environment variables support per project (project-level overrides group-level)
- Auto-restart toggle: automatically restart the process if it exits unexpectedly
- Per-project CWD override for monorepo support (relative or absolute paths)

## Process Control
- Start/Stop/Restart processes with dedicated buttons
- Start All / Stop All: bulk control all processes in a group from the group navbar or monitor dashboard
- Graceful shutdown: sends SIGTERM with 5-second timeout before SIGKILL
- Process group management on Unix (setpgid) for clean termination of child processes
- Real-time process status tracking (running/stopped/errored)
- Status badge indicators in sidebar and project detail view
- PR_SET_PDEATHSIG on Linux: child processes receive SIGTERM if parent dies unexpectedly

## CPU & Memory Monitoring
- Real-time CPU usage percentage display (normalized by CPU core count for 0-100% range)
- Real-time memory usage display (auto-formatted: B/KB/MB/GB, capped at total system memory)
- Stats collected every 2 seconds via sysinfo crate (aggregated across process tree)
- CPU and memory info shown inline in sidebar next to project names
- When process is stopped, shows last session info: last run time, duration, exit status, and last known CPU/MEM from final metrics
- Process tree aggregation: stats include the main process and all child processes
- Fixed: uses ProcessesToUpdate::All for reliable child process detection

## CPU/Memory Monitoring Over Time
- Real-time CPU and memory line charts per project (chart.js + vue-chartjs)
- Toggle monitor graph button in project detail header — persists when switching between projects
- Charts update live every 2 seconds from process-stats-updated events
- When process is stopped, monitor shows historical metrics from the last session
- Up to 60 data points shown with smooth line interpolation
- Dark theme styling matching the application UI

## Group-Level Monitoring Dashboard
- Monitor all projects in a group from a single dashboard view
- Accessible from group right-click context menu ("Monitor") and sidebar monitoring icon
- Sidebar monitoring icon shows running/total project count with green tint when active
- Start All / Stop All button in the dashboard header for bulk process control
- Individual play/stop button on each project card for quick process control
- Shows project name, status badge, auto-restart badge, CPU %, memory usage, PID for running projects
- Mini SVG sparkline charts for CPU and memory per project
- Real-time updates via existing event system
- Recent log output preview (last 5 lines) for both running and stopped projects
- Stopped/errored projects show last session info: timestamp, duration, exit status, last known CPU/MEM
- "No session history" fallback for projects that have never been run
- Click any project card to navigate to its detail view

## Log Output
- Real-time log streaming from stdout and stderr via xterm.js terminal emulator
- Configurable scrollback buffer (default 10,000 lines, adjustable in settings)
- Batched writes using requestAnimationFrame for performance
- Clear logs button (clears terminal display, in-memory buffer, SQLite, and disk cache)
- Monospace font rendering (JetBrains Mono, Fira Code, Cascadia Code)
- Dark theme matching the application UI
- Logs persist when switching between projects (in-memory + SQLite-backed)
- Logs written to SQLite database for persistent storage across sessions
- ANSI color code support via FORCE_COLOR and CLICOLOR_FORCE environment variables
- Clickable URLs in log output — detected automatically and opened in the default browser on click
- Clickable file paths in log output — detects file paths with line:column references (e.g. `src/main.ts:42:10`) and opens them in the user's editor
- Clickable directory paths in log output — detects absolute directory paths (e.g. `/home/user/project`) and opens them in the file manager
- Configurable default editor in settings — auto-detects system editor ($VISUAL/$EDITOR or common editors) with manual override option
- Editor support includes: VS Code, Cursor, Zed, Sublime Text, vim/nvim, JetBrains IDEs (IntelliJ, GoLand, WebStorm, etc.), Emacs
- Search logs with Ctrl+F — search bar with next/prev navigation and match highlighting

## Sessions (Historical Logs/Metrics)
- Each process start creates a new session in SQLite
- Sessions track start time, end time, and exit status (stopped/errored)
- Browse previous sessions via Sessions button in project detail header
- Session list shows metadata: log count, log data size (formatted KB/MB), and metric count per session
- View historical logs from any past session in a read-only xterm.js terminal
- Historical CPU/memory charts in session detail view (collapsible, showing all data points)
- Search session logs with Ctrl+F — same search bar UI as live logs (next/prev/close)
- Clickable URLs and file paths in session logs — same detection as live logs (opens in browser/editor)
- Delete individual sessions and their associated logs/metrics
- Session metrics (CPU/memory) stored every 2 seconds for historical analysis

## User Interface
- Dark-themed UI built with Tailwind CSS
- Group navbar on project detail page: shows group breadcrumb, running count, start/stop all, and link to group monitor
- Resizable sidebar with drag handle (min 180px, max 480px)
- Main content panel with project details, controls, and log output
- Responsive terminal that resizes with the window
- Context menus on right-click for group operations
- Modal dialogs for create/edit/delete operations
- Empty state guidance when no groups exist

## Settings
- Full settings page accessible from sidebar header (gear icon)
- Configurable max log lines per project (1,000 - 100,000, default 10,000)
- Storage management dashboard showing total size, sessions, logs, and metrics counts
- Cleanup old data by age (delete data older than X days)
- Clear all data button with confirmation dialog
- Settings persisted to ~/.config/runner-ui/settings.json

## Data Persistence
- Configuration saved to platform-specific app data directory (~/.config/runner-ui/config.json)
- Settings saved to ~/.config/runner-ui/settings.json
- SQLite database for logs, metrics, and sessions (~/.config/runner-ui/runner-ui.db)
- Log files saved to temp directory (runner-ui-logs/) as backward-compatible fallback
- Auto-saves after every configuration change
- Loads saved configuration and settings on app startup

## Desktop Application
- Built with Tauri 2 (Rust backend + Vue 3 frontend)
- Minimum window size: 800x600, default: 1200x800
- Process cleanup on application exit (kills all managed processes)
- Native folder picker via tauri-plugin-dialog
- **Cross-platform process management**: Platform abstraction layer with OS-specific implementations for Linux, macOS, and Windows
- **Linux-specific**: Process groups with setpgid, PR_SET_PDEATHSIG for child cleanup on parent death
- **Windows-specific**: Job objects for process lifecycle management, CREATE_NEW_PROCESS_GROUP for graceful signal handling
- **macOS-specific**: Process groups with setpgid (POSIX-compliant)

## CI/CD & Distribution
- **GitHub Actions automated builds**: Builds for macOS (Apple Silicon + Intel), Windows, and Linux on every push to main/master
- **Automated releases**: Creates GitHub releases with signed artifacts when version tags (v*) are pushed
- **Multi-platform artifacts**: Native installers for each platform (.dmg, .msi, .deb, .AppImage)
