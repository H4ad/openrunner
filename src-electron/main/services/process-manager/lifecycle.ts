/**
 * Process lifecycle management (stop, kill, shutdown).
 * This is the equivalent of src-tauri/src/process/lifecycle.rs
 */

import type { BrowserWindow } from 'electron';
import type { ProcessStatus } from '../../../shared/types';
import { getState } from '../state';
import { getPlatformManager } from '../../platform';
import { emitStatusUpdate } from './index';

/**
 * Stop a running process
 */
export async function stopProcess(projectId: string): Promise<void> {
  const state = getState();
  const managed = state.processes.get(projectId);

  if (!managed) {
    throw new Error(`Process ${projectId} is not running`);
  }

  managed.manuallyStopped = true;

  const platform = getPlatformManager();
  const pidToKill = managed.isInteractive
    ? managed.realPid
    : managed.child?.pid ?? managed.realPid;

  if (pidToKill) {
    platform.gracefulShutdown(pidToKill);

    // Force kill after timeout
    setTimeout(() => {
      if (platform.isProcessRunning(pidToKill)) {
        platform.forceKill(pidToKill);
      }
    }, 5000);
  }
}

/**
 * Kill all running processes immediately
 */
export function killAllProcesses(): void {
  const state = getState();
  const platform = getPlatformManager();

  for (const [projectId, managed] of state.processes) {
    managed.manuallyStopped = true;

    // End session in database
    if (managed.sessionId) {
      try {
        state.database.endSession(managed.sessionId, 'stopped');
      } catch {
        // Ignore errors
      }
    }

    // Remove from active sessions
    state.activeSessions.delete(projectId);

    // Get PID to kill
    const pidToKill = managed.isInteractive
      ? managed.realPid
      : managed.child?.pid ?? managed.realPid;

    if (pidToKill) {
      platform.forceKill(pidToKill);
    }

    // Kill PTY process if exists
    if (managed.ptyProcess) {
      try {
        managed.ptyProcess.kill();
      } catch {
        // Already dead
      }
    }
  }

  state.processes.clear();
  state.clearPidFile();
}

/**
 * Gracefully shutdown all processes with UI feedback.
 * Sends graceful shutdown signal, waits for processes to exit, then force kills if needed.
 */
export async function gracefulShutdownAll(
  mainWindow: BrowserWindow | null
): Promise<void> {
  const state = getState();
  const platform = getPlatformManager();

  // Collect all running process info
  const processInfo: Array<{
    projectId: string;
    groupId: string;
    pid: number | null;
    sessionId: string | null;
    isInteractive: boolean;
  }> = [];

  for (const [projectId, managed] of state.processes) {
    managed.manuallyStopped = true;
    const pid = managed.isInteractive
      ? managed.realPid
      : managed.child?.pid ?? managed.realPid;

    processInfo.push({
      projectId,
      groupId: managed.groupId,
      pid: pid ?? null,
      sessionId: managed.sessionId,
      isInteractive: managed.isInteractive,
    });
  }

  if (processInfo.length === 0) {
    return;
  }

  // Update UI to show "stopping" status for all processes
  for (const info of processInfo) {
    emitStatusUpdate(mainWindow, info.projectId, 'stopping', info.pid);
  }

  // Send graceful shutdown to all processes
  for (const info of processInfo) {
    if (info.pid) {
      platform.gracefulShutdown(info.pid);
    }
  }

  // Wait for processes to exit (up to 5 seconds)
  const timeout = 5000;
  const start = Date.now();

  while (Date.now() - start < timeout) {
    await new Promise((resolve) => setTimeout(resolve, 100));

    let allExited = true;
    const toRemove: string[] = [];

    for (const [projectId, managed] of state.processes) {
      const pid = managed.isInteractive
        ? managed.realPid
        : managed.child?.pid ?? managed.realPid;

      if (pid && platform.isProcessRunning(pid)) {
        allExited = false;
      } else {
        toRemove.push(projectId);
      }
    }

    for (const projectId of toRemove) {
      state.processes.delete(projectId);
    }

    if (allExited) {
      break;
    }
  }

  // Force kill any remaining processes
  for (const info of processInfo) {
    if (info.pid && platform.isProcessRunning(info.pid)) {
      platform.forceKill(info.pid);
    }
  }

  // End all sessions in database
  for (const info of processInfo) {
    if (info.sessionId) {
      try {
        state.database.endSession(info.sessionId, 'stopped');
      } catch {
        // Ignore errors
      }
    }

    // Remove from active sessions
    state.activeSessions.delete(info.projectId);

    // Update UI to show stopped
    emitStatusUpdate(mainWindow, info.projectId, 'stopped', null);
  }

  // Clear process map and PID file
  state.processes.clear();
  state.clearPidFile();
}

/**
 * Kill any orphaned processes from previous app runs
 */
export function killOrphanedProcesses(): void {
  const state = getState();
  const pids = state.readStoredPids();
  getPlatformManager().killOrphanedProcesses(pids);
  state.clearPidFile();
}
