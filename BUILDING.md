# Building OpenRunner

This guide covers how to build OpenRunner from source for development or distribution.

## Prerequisites

Before you start, you'll need:

- **Node.js** - Version 18 or later from [nodejs.org](https://nodejs.org/)
- **pnpm** - Version 10.18.3 or later from [pnpm.io](https://pnpm.io/)

Verify your installations:

```bash
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

# Install dependencies (also rebuilds native modules)
pnpm install

# Start development mode (GUI with hot reload)
pnpm dev
```

This opens a window with:
- Hot reload for Vue components
- Automatic Electron rebuild on changes
- DevTools enabled

### Frontend Only

If you're only working on the UI:

```bash
# Start Vite dev server on port 5173
pnpm frontend:dev
```

Note: Some features won't work without the Electron backend.

## Production Build

### GUI Application

To create a distributable desktop application:

```bash
# Build for your current platform
pnpm build
```

The built application will be in `out/` directory.

### Build Outputs

Depending on your platform:

| Platform | Output |
|----------|--------|
| macOS | `.dmg` installer, `.app` bundle |
| Windows | `.exe` installer |
| Linux | `.deb` package, `.AppImage`, `.rpm` |

## Platform-Specific Notes

### macOS

- macOS 10.13 or later required
- Code signing recommended for distribution

### Windows

- Windows 10 or later required
- Builds both 64-bit and ARM64 versions

### Linux

Tested on:
- Ubuntu 20.04+
- Debian 11+
- Fedora 35+

## TypeScript Development

When working on the backend:

```bash
# Type check Electron main process
pnpm typecheck

# Rebuild native modules if needed
pnpm rebuild
```

## Troubleshooting

### Build Failures

**"Module was compiled against a different Node.js version"**
- Run `pnpm rebuild` to rebuild native modules for Electron

**npm install failures**
- Ensure all prerequisites are installed
- Try deleting `node_modules` and `pnpm install` again

### Runtime Issues

**App crashes on startup**
- Check console output for errors
- Verify database permissions in `~/.config/openrunner/`

**Processes won't start**
- Ensure shell commands are valid
- Check working directory permissions

### Native Module Issues

```bash
# Rebuild native modules (better-sqlite3, node-pty)
pnpm rebuild
```

**Config file location**
The app reads/writes config from:
- **Linux**: `~/.config/openrunner/`
- **macOS**: `~/Library/Application Support/openrunner/`
- **Windows**: `%APPDATA%\openrunner\`

## Release Checklist

When preparing a release:

1. Update version in `package.json`
2. Update CHANGELOG.md
3. Run `pnpm build` on all target platforms
4. Test installers on clean systems
5. Create GitHub release with binaries

## Questions?

Open an issue if you run into problems building. Include:
- Your operating system and version
- Output of `node --version`, `pnpm --version`
- Full error messages
