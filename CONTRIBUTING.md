# Contributing to OpenRunner

First off, thanks for your interest in contributing! OpenRunner is built by developers for developers, and we welcome all kinds of contributions.

## How to Contribute

### Reporting Bugs

Found something broken? We'd love to know:

1. Check if the issue already exists in our issue tracker
2. If not, open a new issue with:
   - A clear description of what you expected vs what happened
   - Steps to reproduce the problem
   - Your operating system and OpenRunner version
   - Screenshots if relevant

### Suggesting Features

Have an idea? We'd love to hear it:

1. Open a new issue with the "feature request" label
2. Describe what problem it solves
3. Explain how you'd expect it to work

### Contributing Code

Want to fix a bug or add a feature? Here's how:

1. **Fork the repository** and create a branch: `git checkout -b feature/your-feature-name`
2. **Make your changes** - see the Development section below
3. **Test your changes** - run the app and make sure everything works
4. **Commit your changes** with a clear message
5. **Push to your fork** and open a Pull Request

#### Pull Request Guidelines

- Keep changes focused - one feature or fix per PR
- Update the README if you're adding user-facing features
- Make sure the app builds and runs without errors
- Be open to feedback and code review

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) - latest stable version
- [Node.js](https://nodejs.org/) - v18 or later
- [pnpm](https://pnpm.io/) - v10.18.3 or later

### Running Locally

```bash
# Clone your fork
git clone https://github.com/yourusername/openrunner.git
cd openrunner

# Install dependencies
pnpm install

# Start development mode
pnpm tauri dev
```

The app will open in a window with hot reload enabled for both frontend and backend changes.

### Project Structure

```
/openrunner
├── src/                    # Vue 3 frontend
│   ├── components/         # Vue SFCs
│   ├── stores/            # Pinia state management
│   ├── types/             # TypeScript interfaces
│   └── App.vue            # Root component
├── src-tauri/src          # Rust backend
│   ├── commands/          # Tauri command handlers
│   ├── lib.rs             # App setup and initialization
│   ├── state.rs           # Shared mutable state
│   ├── models.rs          # Data structures
│   ├── error.rs           # Error handling
│   ├── process_manager.rs # Process spawning and management
│   ├── stats_collector.rs # CPU/memory monitoring
│   ├── database.rs        # SQLite operations
│   └── storage.rs         # JSON persistence
└── logo.svg               # App logo
```

### Tech Stack

- **Frontend**: Vue 3 with Composition API, Tailwind CSS v4
- **Backend**: Rust with Tauri 2
- **State**: Pinia for frontend, std Mutex for backend
- **Terminal**: xterm.js with custom dark theme
- **Charts**: Chart.js with vue-chartjs
- **Database**: SQLite with rusqlite

## Code Style

We don't have strict formatting rules configured, but please try to match the existing code style:

### TypeScript / Vue
- Use camelCase for variables and functions
- Use PascalCase for components and types
- Use 2-space indentation
- Prefer single quotes

### Rust
- Use snake_case for functions and variables
- Use PascalCase for structs and enums
- Use 4-space indentation
- Follow standard Rust conventions

## Questions?

Join the conversation in our GitHub Discussions or open an issue. We're happy to help!

Thanks for contributing! 🚀
