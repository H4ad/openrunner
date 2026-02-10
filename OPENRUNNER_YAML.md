# OpenRunner YAML Configuration

## Overview

OpenRunner now supports importing and syncing groups from YAML configuration files. This allows you to:

1. Define your projects in a version-controlled YAML file
2. Automatically import all projects when creating a group
3. Keep changes synchronized between the YAML file and config.json
4. Enable/disable sync at any time

## YAML File Format

Create an `openrunner.yaml` or `openrunner.yml` file in your project directory:

```yaml
version: "1.0"
name: "My Project Group"
envVars:
  NODE_ENV: development
  PORT: "3000"
projects:
  - name: "Web Server"
    command: "npm run dev"
    type: service          # Options: service, task
    autoRestart: true
    cwd: "./frontend"
    interactive: false
    envVars:
      DEBUG: "true"

  - name: "API Server"
    command: "cargo run"
    type: service
    autoRestart: true
    cwd: "./backend"

  - name: "Database Migration"
    command: "npm run migrate"
    type: task             # Tasks run once without auto-restart
    autoRestart: false
    cwd: "./database"
```

## Field Reference

### Group Fields

- `version`: (string) YAML schema version, currently "1.0"
- `name`: (string) Group display name
- `envVars`: (object) Environment variables applied to all projects

### Project Fields

- `name`: (string) Project display name (required)
- `command`: (string) Command to execute (required)
- `type`: (string) "service" for long-running processes, "task" for one-time commands
- `autoRestart`: (boolean) Auto-restart on crash (services only)
- `cwd`: (string) Working directory relative to group directory (optional)
- `interactive`: (boolean) Enable interactive mode (optional, default: false)
- `envVars`: (object) Project-specific environment variables

## Usage

### Via UI

1. Click "New Group" in the sidebar
2. Select a directory
3. Choose whether to enable YAML sync:
   - **Enable sync**: If `openrunner.yaml` exists, it will be imported. If not, an empty one will be created.
   - **Disable sync**: Create a regular group without YAML file
4. The group will show a sync indicator when synced with YAML

### Toggling Sync for Existing Groups

You can enable or disable YAML sync for any existing group:

- **Enable sync**: The app will create `openrunner.yaml` from the current group configuration
- **Disable sync**: The group continues to work normally, but changes won't update the YAML file

### Via CLI

```bash
# Check current directory for openrunner.yaml first
openrunner new .

# Or specify a directory
openrunner new /path/to/project

# With custom name
openrunner new . --name "My Projects"

# Dry run to preview without creating
openrunner new . --dry-run
```

If `openrunner.yaml` exists, it will be used as the source of truth. Otherwise, the CLI will auto-detect projects from package.json, Cargo.toml, etc.

## Sync Behavior

When a group is synced with a YAML file (`syncEnabled: true`):

1. **Two-way sync**: Changes in the UI update both `config.json` and the YAML file
2. **YAML as source of truth**: When loading, the YAML file takes precedence
3. **File watching**: The app watches the YAML file for external changes
4. **Safe deletion**: When removing a project, it is stopped first before removal
5. **Auto-create**: If sync is enabled but YAML doesn't exist, it will be created automatically

When sync is disabled (`syncEnabled: false`):

1. Changes are saved only to `config.json`
2. The YAML file is not updated
3. The file watcher is disabled
4. Existing YAML file is preserved but not used

## Example Scenarios

### Microservices Setup

```yaml
version: "1.0"
name: "E-commerce Platform"
envVars:
  LOG_LEVEL: info
projects:
  - name: "Frontend"
    command: "npm run dev"
    type: service
    cwd: "./frontend"
    
  - name: "API Gateway"
    command: "go run main.go"
    type: service
    cwd: "./gateway"
    envVars:
      PORT: "8080"
      
  - name: "User Service"
    command: "cargo run"
    type: service
    cwd: "./services/user"
    
  - name: "Order Service"
    command: "cargo run"
    type: service
    cwd: "./services/order"
    
  - name: "Database Setup"
    command: "docker-compose up -d postgres"
    type: task
    autoRestart: false
```

### Full-Stack Project

```yaml
version: "1.0"
name: "SaaS Dashboard"
projects:
  - name: "Vite Dev Server"
    command: "npm run dev"
    type: service
    cwd: "./client"
    
  - name: "Backend API"
    command: "python -m uvicorn main:app --reload"
    type: service
    cwd: "./server"
    envVars:
      DATABASE_URL: "postgresql://localhost:5432/myapp"
```

## Migration from Existing Groups

To migrate an existing group to use YAML sync:

1. Open the group in the app
2. Toggle YAML sync on (the app will create the YAML file automatically)
3. The YAML file is created from the current group configuration

Or manually:

1. Export the group: Right-click → Export Group
2. Convert the JSON to YAML format
3. Save as `openrunner.yaml` in the group directory
4. Delete and recreate the group (it will auto-import from YAML)

## Notes

- The `syncFile` field in the group indicates which YAML file it's synced with
- The `syncEnabled` field controls whether sync is active
- If you manually edit the YAML file while sync is enabled, the app will detect changes automatically
- Disabling sync preserves the YAML file but stops updating it
- Re-enabling sync will update the YAML file with current group state
- YAML files are human-readable and perfect for version control
