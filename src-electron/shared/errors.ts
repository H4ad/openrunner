/**
 * Application error handling for the Electron main process.
 * Mirrors the error types from src-tauri/src/error.rs
 */

export class AppError extends Error {
  constructor(
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'AppError';
  }

  /**
   * Convert to a serializable object for IPC
   */
  toJSON(): { code: string; message: string } {
    return {
      code: this.code,
      message: this.message,
    };
  }
}

/**
 * Error factory functions matching the Rust error enum
 */
export const errors = {
  groupNotFound: (id: string) =>
    new AppError('GROUP_NOT_FOUND', `Group not found: ${id}`),

  projectNotFound: (id: string) =>
    new AppError('PROJECT_NOT_FOUND', `Project not found: ${id}`),

  processAlreadyRunning: (id: string) =>
    new AppError('PROCESS_RUNNING', `Process already running for project: ${id}`),

  processNotRunning: (id: string) =>
    new AppError('PROCESS_NOT_RUNNING', `No process running for project: ${id}`),

  spawnError: (msg: string) =>
    new AppError('SPAWN_ERROR', `Failed to spawn process: ${msg}`),

  storageError: (msg: string) =>
    new AppError('STORAGE_ERROR', `Storage error: ${msg}`),

  ioError: (msg: string) =>
    new AppError('IO_ERROR', `IO error: ${msg}`),

  ptyError: (msg: string) =>
    new AppError('PTY_ERROR', `PTY error: ${msg}`),

  databaseError: (msg: string) =>
    new AppError('DATABASE_ERROR', `Database error: ${msg}`),

  fileNotFound: (path: string) =>
    new AppError('FILE_NOT_FOUND', `File not found: ${path}`),

  yamlConfigError: (msg: string) =>
    new AppError('YAML_CONFIG', `YAML config error: ${msg}`),
};

/**
 * Type guard to check if an error is an AppError
 */
export function isAppError(error: unknown): error is AppError {
  return error instanceof AppError;
}

/**
 * Wrap an error for IPC transmission
 * Converts any error to a string message that can be sent to the renderer
 */
export function wrapError(error: unknown): string {
  if (isAppError(error)) {
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
