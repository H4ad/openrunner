/**
 * IPC Event definitions for communication between main and renderer processes.
 * These mirror the Tauri events used in the original application.
 */

/**
 * Event channels for main -> renderer communication
 */
export const IPC_EVENTS = {
  // Process events
  PROCESS_STATUS_CHANGED: 'process-status-changed',
  PROCESS_STATS_UPDATED: 'process-stats-updated',
  PROCESS_LOG: 'process-log',

  // Config events
  CONFIG_RELOADED: 'config-reloaded',
  YAML_FILE_CHANGED: 'yaml-file-changed',

  // App lifecycle
  APP_CLOSING: 'app-closing',

  // Auto-update events
  UPDATE_CHECKING: 'update-checking',
  UPDATE_AVAILABLE: 'update-available',
  UPDATE_NOT_AVAILABLE: 'update-not-available',
  UPDATE_DOWNLOAD_PROGRESS: 'update-download-progress',
  UPDATE_DOWNLOADED: 'update-downloaded',
  UPDATE_ERROR: 'update-error',
} as const;

/**
 * IPC channels for renderer -> main communication (invoke pattern)
 * These are the command names that map to ipcMain.handle()
 */
export const IPC_CHANNELS = {
  // Group commands
  GET_GROUPS: 'get-groups',
  CREATE_GROUP: 'create-group',
  RENAME_GROUP: 'rename-group',
  UPDATE_GROUP_DIRECTORY: 'update-group-directory',
  UPDATE_GROUP_ENV_VARS: 'update-group-env-vars',
  DELETE_GROUP: 'delete-group',
  TOGGLE_GROUP_SYNC: 'toggle-group-sync',
  RELOAD_GROUP_FROM_YAML: 'reload-group-from-yaml',
  EXPORT_GROUP: 'export-group',
  IMPORT_GROUP: 'import-group',

  // Project commands
  CREATE_PROJECT: 'create-project',
  UPDATE_PROJECT: 'update-project',
  DELETE_PROJECT: 'delete-project',
  DELETE_MULTIPLE_PROJECTS: 'delete-multiple-projects',
  CONVERT_MULTIPLE_PROJECTS: 'convert-multiple-projects',

  // Process commands
  START_PROCESS: 'start-process',
  STOP_PROCESS: 'stop-process',
  RESTART_PROCESS: 'restart-process',
  GET_ALL_STATUSES: 'get-all-statuses',
  WRITE_TO_PROCESS_STDIN: 'write-to-process-stdin',
  RESIZE_PTY: 'resize-pty',

  // Session commands
  GET_PROJECT_SESSIONS: 'get-project-sessions',
  GET_PROJECT_SESSIONS_WITH_STATS: 'get-project-sessions-with-stats',
  GET_SESSION: 'get-session',
  GET_SESSION_LOGS: 'get-session-logs',
  GET_SESSION_METRICS: 'get-session-metrics',
  GET_LAST_COMPLETED_SESSION: 'get-last-completed-session',
  GET_RECENT_LOGS: 'get-recent-logs',
  GET_LAST_METRIC: 'get-last-metric',
  DELETE_SESSION: 'delete-session',

  // Settings commands
  GET_SETTINGS: 'get-settings',
  UPDATE_SETTINGS: 'update-settings',
  DETECT_SYSTEM_EDITOR: 'detect-system-editor',
  DETECT_SYSTEM_SHELL: 'detect-system-shell',

  // Storage commands
  GET_STORAGE_STATS: 'get-storage-stats',
  CLEANUP_STORAGE: 'cleanup-storage',
  CLEANUP_ALL_STORAGE: 'cleanup-all-storage',
  GET_DATABASE_PATH: 'get-database-path',

  // File commands
  READ_PROJECT_LOGS: 'read-project-logs',
  CLEAR_PROJECT_LOGS: 'clear-project-logs',
  RESOLVE_PROJECT_WORKING_DIR: 'resolve-project-working-dir',
  RESOLVE_WORKING_DIR_BY_PROJECT: 'resolve-working-dir-by-project',
  OPEN_FILE_IN_EDITOR: 'open-file-in-editor',
  OPEN_PATH: 'open-path',
  OPEN_IN_TERMINAL: 'open-in-terminal',

  // Dialog commands (Electron-specific)
  DIALOG_OPEN: 'dialog:open',
  DIALOG_SAVE: 'dialog:save',

  // Shell commands (Electron-specific)
  SHELL_OPEN_EXTERNAL: 'shell:open-external',

  // Window commands (Electron-specific)
  WINDOW_TOGGLE_FULLSCREEN: 'window:toggle-fullscreen',
  WINDOW_GET_FULLSCREEN: 'window:get-fullscreen',

  // Auto-update commands
  CHECK_FOR_UPDATES: 'check-for-updates',
  DOWNLOAD_UPDATE: 'download-update',
  INSTALL_UPDATE: 'install-update',
  GET_APP_VERSION: 'get-app-version',
  IS_DEV_MODE: 'is-dev-mode',
} as const;

export type IpcEvent = (typeof IPC_EVENTS)[keyof typeof IPC_EVENTS];
export type IpcChannel = (typeof IPC_CHANNELS)[keyof typeof IPC_CHANNELS];
