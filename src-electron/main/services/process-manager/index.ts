/**
 * Process manager module for spawning, managing, and monitoring processes.
 * This is the equivalent of src-tauri/src/process/mod.rs
 */

// Re-export all submodules
export * from './lifecycle';
export * from './spawn';
export * from './watcher';
export * from './io';

import type { BrowserWindow } from 'electron';
import type { ProcessInfo, ProcessStatus, ProjectType } from '../../../shared/types';
import { getState } from '../state';
import { spawnRegularProcess, spawnInteractiveProcess } from './spawn';
import { watchExit } from './watcher';
import { IPC_EVENTS } from '../../../shared/events';
import { startFileWatcher, stopFileWatcher } from '../file-watcher';
import { restartProcess } from './restart';

/**
 * Spawn a process for a project
 */
export async function spawnProcess(
  mainWindow: BrowserWindow | null,
  projectId: string,
  groupId: string,
  command: string,
  workingDir: string,
  envVars: Record<string, string>,
  autoRestart: boolean,
  projectType: ProjectType,
  interactive: boolean,
  cols?: number,
  rows?: number
): Promise<void> {
  const state = getState();

  // Check if already running
  if (state.processes.has(projectId)) {
    throw new Error(`Process ${projectId} is already running`);
  }

  // Create a new session in the database
  const sessionId = state.database.createSession(projectId);

  // Track active session
  state.activeSessions.set(projectId, sessionId);

  // Clear log file for this project on fresh start
  const logPath = state.logFilePath(projectId);
  const fs = await import('fs');
  try {
    fs.writeFileSync(logPath, '');
  } catch {
    // Ignore errors
  }

  if (interactive) {
    // Spawn using PTY for interactive mode
    await spawnInteractiveProcess(
      mainWindow,
      projectId,
      groupId,
      command,
      workingDir,
      envVars,
      sessionId,
      logPath,
      cols,
      rows
    );
  } else {
    // Spawn using regular pipes for non-interactive mode
    await spawnRegularProcess(
      mainWindow,
      projectId,
      groupId,
      command,
      workingDir,
      envVars,
      sessionId,
      logPath
    );
  }

  // Spawn exit watcher
  watchExit(
    mainWindow,
    projectId,
    groupId,
    command,
    workingDir,
    envVars,
    autoRestart,
    projectType
  );

  // Start file watcher if autoRestart is enabled and project is a service
  if (autoRestart && projectType === 'service') {
    // Find the project to get watch patterns and the group for groupDir (from database)
    const project = state.database.getProject(projectId);
    const group = state.database.getGroup(groupId);
    if (project && group) {
      startFileWatcher({
        projectId,
        watchDir: workingDir,
        groupDir: group.directory,
        patterns: project.watchPatterns,
        onChange: (changedFile) => {
          // Trigger restart when files change
          restartProcess(projectId, changedFile);
        },
      });
    }
  }
}

/**
 * Emit a status update event to the renderer
 */
export function emitStatusUpdate(
  mainWindow: BrowserWindow | null,
  projectId: string,
  status: ProcessStatus,
  pid: number | null
): void {
  if (!mainWindow) return;

  const info: ProcessInfo = {
    projectId,
    status,
    pid,
    cpuUsage: 0,
    memoryUsage: 0,
  };

  mainWindow.webContents.send(IPC_EVENTS.PROCESS_STATUS_CHANGED, info);
}
