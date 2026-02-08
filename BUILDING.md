# Building OpenRunner

This guide covers how to build OpenRunner from source for development or distribution.

## Prerequisites

Before you start, you'll need:

- **Rust** - Install from [rustup.rs](https://rustup.rs/) (latest stable)
- **Node.js** - Version 18 or later from [nodejs.org](https://nodejs.org/)
- **pnpm** - Version 10.18.3 or later from [pnpm.io](https://pnpm.io/)

Verify your installations:

```bash
rustc --version    # Should show latest stable
node --version     # Should show v18+
pnpm --version     # Should show 10.18.3+
```

## Development Build

### GUI Mode (Desktop Application)

The fastest way to run OpenRunner during development:

```bash
# Clone the repository
git clone https://github.com/yourusername/openrunner.git
cd openrunner

# Install frontend dependencies
pnpm install

# Start development mode (GUI)
pnpm tauri dev
```

This opens a window with:
- Hot reload for Vue components
- Automatic Rust recompilation on changes
- DevTools enabled

### CLI Mode

To build and run the CLI:

```bash
# Build the CLI binary (release version)
cd src-tauri && cargo build --release

# Run the CLI
./target/release/openrunner --help
./target/release/openrunner new . --dry-run
```

For development/testing with faster builds:

```bash
# Debug build (faster compilation)
cd src-tauri && cargo build

# Run debug version
./target/debug/openrunner new . --dry-run
```

## Frontend Only

If you're only working on the UI:

```bash
# Start Vite dev server on port 1420
pnpm dev
```

Note: Some features won't work without the Rust backend.

## Production Build

### GUI Application

To create a distributable desktop application:

```bash
# Build for your current platform
pnpm tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

### Build Outputs

Depending on your platform:

| Platform | Output |
|----------|--------|
| macOS | `.dmg` installer, `.app` bundle |
| Windows | `.msi` installer, `.exe` |
| Linux | `.deb` package, `.AppImage`, `.rpm` |

### CLI Binary

To build just the CLI binary:

```bash
cd src-tauri

# Build release version
cargo build --release

# The binary will be at:
# - Linux/macOS: ./target/release/openrunner
# - Windows:     .\target\release\openrunner.exe
```

**Install CLI to your PATH:**

```bash
# Option 1: Copy to a directory in your PATH
# Linux/macOS:
sudo cp src-tauri/target/release/openrunner /usr/local/bin/

# Option 2: Add to PATH temporarily
export PATH="$PATH:$(pwd)/src-tauri/target/release"

# Option 3: Create a symlink (macOS/Linux)
ln -s $(pwd)/src-tauri/target/release/openrunner ~/.local/bin/openrunner
```

## Platform-Specific Notes

### macOS

- macOS 10.13 or later required
- Code signing recommended for distribution
- Notarized for Gatekeeper compliance

### Windows

- Windows 10 or later required
- Builds both 64-bit and ARM64 versions
- MSI installer supports silent installation

### Linux

Tested on:
- Ubuntu 20.04+
- Debian 11+
- Fedora 35+

Dependencies for `.deb`:
- libwebkit2gtk-4.0-37
- libgtk-3-0

## Rust Development

When working on the backend:

```bash
cd src-tauri

# Check for compilation errors
cargo check

# Build debug version
cargo build

# Run linter
cargo clippy

# Run tests (if any exist)
cargo test
```

## Troubleshooting

### Build Failures

**"Failed to run custom build command"**
- Ensure all prerequisites are installed
- Try deleting `node_modules` and `pnpm install` again

**Rust compilation errors**
- Update Rust: `rustup update`
- Clean build: `cargo clean` in `src-tauri/`

**Frontend won't load**
- Check if port 1420 is available
- Try: `pnpm dev` then refresh the Tauri window

### Runtime Issues

**App crashes on startup**
- Check console output for errors
- Verify database permissions in `~/.config/openrunner/`

**Processes won't start**
- Ensure shell commands are valid
- Check working directory permissions

### CLI Issues

**CLI binary not found**
```bash
# Make sure you built it
cd src-tauri && cargo build --release

# Check the binary exists
ls -la target/release/openrunner
```

**CLI shows "No projects detected"**
- Ensure you're in a directory with recognizable project files
- Run with `--dry-run` to see debug output
- Check that the files exist: `ls package.json Makefile Cargo.toml`

**Permission denied when running CLI**
```bash
# Make binary executable (Linux/macOS)
chmod +x src-tauri/target/release/openrunner
```

**Config file location**
The CLI reads/writes the same config as the GUI:
- **Linux**: `~/.config/openrunner/config.json`
- **macOS**: `~/Library/Application Support/openrunner/config.json`
- **Windows**: `%APPDATA%\openrunner\config.json`

## Cross-Compilation

OpenRunner uses Tauri's built-in cross-compilation support. See [Tauri's documentation](https://tauri.app/v1/guides/building/cross-platform/) for details on building for other platforms.

## Docker (Experimental)

Not currently supported. OpenRunner requires native desktop APIs that don't work well in containers.

## Release Checklist

When preparing a release:

1. Update version in `package.json`
2. Update version in `src-tauri/Cargo.toml`
3. Update version in `src-tauri/tauri.conf.json`
4. Update CHANGELOG.md
5. Run `pnpm tauri build` on all target platforms
6. **Build CLI binaries**: `cargo build --release` in `src-tauri/`
7. Test CLI functionality:
   ```bash
   ./src-tauri/target/release/openrunner --help
   ./src-tauri/target/release/openrunner new . --dry-run
   ```
8. Test installers on clean systems
9. Create GitHub release with binaries (both GUI and CLI)

## Questions?

Open an issue if you run into problems building. Include:
- Your operating system and version
- Output of `rustc --version`, `node --version`, `pnpm --version`
- Full error messages
