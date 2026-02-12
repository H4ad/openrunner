# Runner UI Features

This document tracks the features implemented in Runner UI.

## Core Features

- **Process Management**: Start, stop, and restart processes with graceful shutdown support
- **Project Organization**: Organize projects into groups for better management
- **Log Streaming**: Real-time log display with xterm.js terminal emulation
- **Process Monitoring**: CPU and memory usage tracking with charts
- **Auto-restart**: Automatic restart on crash for service projects
- **Task vs Service Projects**: Tasks run once and don't auto-restart, Services are long-running
- **Session History**: Track all process executions with logs and metrics
- **Search**: Log search with Ctrl+F navigation
- **File Links**: Click file paths in logs to open in editor
- **Group Export/Import**: Export groups to JSON files and import them to share configurations
- **YAML Auto-Sync**: Groups with sync enabled automatically write changes to `openrunner.yaml` file
- **Rust-Only Data Access**: Frontend reads and writes are routed through Rust commands, avoiding direct SQLite access from the UI
- **Home Dashboard**: Full-width home view with quick actions, global metrics, storage stats, health summary, recent activity, and group overview cards
- **Auto-Updater**: Check for updates from GitHub Releases, download and install with user-initiated flow

## Interactive Terminal Support

- **Per-project Interactive Mode**: Enable full PTY terminal support on a per-project basis
  - Toggle in project settings (disabled by default)
  - Enables stdin input for terminal applications
  - Supports full TUI apps: vim, htop, Claude Code, Node REPL, etc.
  - Proper terminal emulation with PTY (pseudo-terminal)
  - Terminal resize support
  - Visual indicator shows "PTY Mode" in log panel when active

### How It Works

When a project is configured with `interactive: true`:

1. **Process Spawning**: Uses `portable-pty` crate to create a PTY pair
2. **Input Handling**: Keystrokes from xterm.js are sent to the PTY master
3. **Output Streaming**: PTY output is read and displayed in the terminal
4. **Resize Events**: Terminal resize events are forwarded to the PTY

### Using Interactive Mode

1. Create or edit a project
2. Check "Interactive Terminal (PTY)" in the project settings
3. Start the project
4. The log panel will show "PTY Mode" badge
5. Type directly in the terminal - stdin is now active
6. Full TUI apps (vim, htop) work correctly

### Supported Terminal Apps

- **REPLs**: Node.js, Python, Ruby, etc.
- **Interactive CLI**: Claude Code, GitHub Copilot CLI, etc.
- **TUI Applications**: vim, nvim, htop, lazygit, etc.
- **Terminal multiplexers**: tmux, screen (with limitations)

### Technical Details

**Backend Changes:**
- Added `portable-pty` crate for cross-platform PTY support
- Modified `ManagedProcess` to store PTY master and writer handles
- New commands: `write_to_process_stdin` and `resize_pty`
- Interactive processes use PTY instead of regular pipes

**Frontend Changes:**
- Added `interactive` checkbox in ProjectFormDialog
- LogPanel accepts `interactive` prop to enable/disable stdin
- Visual "PTY Mode" badge when interactive mode is active
- Terminal resize events forwarded to backend

### Limitations

- Only works with running processes (cannot interact with stopped processes)
- Stdin input is not logged in session history (only output is logged)
- One PTY per project (no multi-terminal support for a single project)
