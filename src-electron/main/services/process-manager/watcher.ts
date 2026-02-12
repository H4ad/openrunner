/**
 * Process exit watching and auto-restart functionality.
 * This is the equivalent of src-tauri/src/process/watcher.rs
 */

import type { BrowserWindow } from 'electron';
import type { ProcessStatus, ProjectType } from '../../../shared/types';
import { getState } from '../state';
import { getPlatformManager } from '../../platform';
import { emitStatusUpdate, spawnProcess } from './index';
import { stopFileWatcher } from '../file-watcher';

/**
 * Watch for process exit and handle auto-restart
 */
export function watchExit(
  mainWindow: BrowserWindow | null,
  projectId: string,
  groupId: string,
  command: string,
  workingDir: string,
  envVars: Record<string, string>,
  autoRestart: boolean,
  projectType: ProjectType
): void {
  const state = getState();
  const platform = getPlatformManager();

  // Start polling for exit
  const checkInterval = setInterval(async () => {
    const managed = state.processes.get(projectId);

    if (!managed) {
      // Process was already removed
      clearInterval(checkInterval);
      return;
    }

    let hasExited = false;
    let exitSuccess = false;

    if (managed.isInteractive) {
      // For PTY processes, check if the real process is still running
      if (managed.manuallyStopped) {
        hasExited = true;
        exitSuccess = true;
      } else if (managed.realPid) {
        if (!platform.isProcessRunning(managed.realPid)) {
          hasExited = true;
          exitSuccess = false; // Can't determine exit code for PTY
        }
      }
    } else if (managed.child) {
      // For regular processes, check if child has exited
      // Node's ChildProcess doesn't have try_wait, so we check if it's still running
      if (managed.child.exitCode !== null) {
        hasExited = true;
        exitSuccess = managed.child.exitCode === 0;
      } else if (managed.manuallyStopped) {
        // Check if process is still running
        const pid = managed.child.pid;
        if (pid && !platform.isProcessRunning(pid)) {
          hasExited = true;
          exitSuccess = true;
        }
      }
    }

    if (!hasExited) {
      return;
    }

    // Process has exited
    clearInterval(checkInterval);

    // Stop file watcher when process exits
    stopFileWatcher(projectId);

    const manuallyStopped = managed.manuallyStopped;
    const sessionId = managed.sessionId;
    const pid = managed.isInteractive
      ? managed.realPid
      : managed.child?.pid ?? managed.realPid;

    // Remove from processes map
    state.processes.delete(projectId);

    // Remove the PID from the PID file
    if (pid) {
      state.removePid(pid);
    }

    const status: ProcessStatus =
      manuallyStopped || exitSuccess ? 'stopped' : 'errored';

    // End session in database
    if (sessionId) {
      const exitStatusStr = manuallyStopped
        ? 'stopped'
        : exitSuccess
          ? 'stopped'
          : 'errored';
      try {
        state.database.endSession(sessionId, exitStatusStr);
      } catch {
        // Ignore errors
      }

      // Remove from active sessions
      state.activeSessions.delete(projectId);
    }

    // Update process info
    const info = state.processInfos.get(projectId);
    if (info) {
      info.status = status;
      info.pid = null;
      info.cpuUsage = 0;
      info.memoryUsage = 0;
    }

    emitStatusUpdate(mainWindow, projectId, status, null);

    // Auto-restart if enabled, not manually stopped, exited successfully, and project type is Service
    // Tasks should never auto-restart, and crashed processes should not auto-restart
    if (
      autoRestart &&
      !manuallyStopped &&
      exitSuccess &&
      projectType === 'service'
    ) {
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // Re-check settings from database (they may have changed)
      const project = state.database.getProject(projectId);

      // Only restart if project exists and is still a Service type with autoRestart enabled
      if (project && project.autoRestart && project.projectType === 'service') {
        // Check process isn't already running (e.g., user restarted manually)
        if (!state.processes.has(projectId)) {
          try {
            await spawnProcess(
              mainWindow,
              projectId,
              groupId,
              command,
              workingDir,
              envVars,
              project.autoRestart,
              project.projectType,
              project.interactive
            );
          } catch {
            // Ignore errors on restart
          }
        }
      }
    }
  }, 100);
}
