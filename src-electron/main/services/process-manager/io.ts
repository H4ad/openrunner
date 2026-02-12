/**
 * Process I/O operations (stdin, PTY resize).
 * This is the equivalent of src-tauri/src/process/io.rs
 */

import { getState } from '../state';

/**
 * Write data to a process's stdin (PTY only)
 */
export function writeToProcessStdin(projectId: string, data: string): void {
  const state = getState();
  const managed = state.processes.get(projectId);

  if (!managed) {
    throw new Error(`Process ${projectId} is not running`);
  }

  if (managed.ptyProcess) {
    managed.ptyProcess.write(data);
  } else {
    throw new Error(`Process ${projectId} is not an interactive process`);
  }
}

/**
 * Resize the PTY for an interactive process
 */
export function resizePty(
  projectId: string,
  cols: number,
  rows: number
): void {
  const state = getState();
  const managed = state.processes.get(projectId);

  if (!managed) {
    throw new Error(`Process ${projectId} is not running`);
  }

  if (managed.ptyProcess) {
    managed.ptyProcess.resize(cols, rows);
  } else {
    throw new Error(`Process ${projectId} is not an interactive process`);
  }
}
