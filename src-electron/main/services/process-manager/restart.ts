/**
 * Process restart functionality for file watcher integration.
 */

import type { BrowserWindow } from 'electron';
import { getState } from '../state';
import { stopProcess } from './lifecycle';
import { spawnProcess } from './index';
import { IPC_EVENTS } from '../../../shared/events';
import type { LogStream } from '../../../shared/types';

let mainWindowRef: BrowserWindow | null = null;

/**
 * Set the main window reference for restart operations
 */
export function setRestartMainWindow(window: BrowserWindow | null): void {
  mainWindowRef = window;
}

/**
 * Emit a log message to the renderer for a project
 */
function emitRestartLog(projectId: string, message: string): void {
  if (!mainWindowRef) return;

  const timestamp = Date.now();
  mainWindowRef.webContents.send(IPC_EVENTS.PROCESS_LOG, {
    projectId,
    stream: 'stdout' as LogStream,
    data: message,
    timestamp,
  });
}

/**
 * Restart a process gracefully (stop then start)
 * Used by file watcher when files change
 * @param projectId - The project to restart
 * @param changedFile - Optional path of the file that triggered the restart
 */
export async function restartProcess(projectId: string, changedFile?: string): Promise<void> {
  const state = getState();

  // Find the project and group
  let project: ReturnType<typeof state.database.getProject> = null;
  let groupId: string | null = null;

  for (const group of state.config.groups) {
    const p = group.projects.find((proj: { id: string }) => proj.id === projectId);
    if (p) {
      project = { ...p, groupId: group.id };
      groupId = group.id;
      break;
    }
  }

  if (!project || !groupId) {
    console.error(`[Restart] Project ${projectId} not found`);
    return;
  }

  const group = state.findGroup(groupId);
  if (!group) {
    console.error(`[Restart] Group ${groupId} not found`);
    return;
  }

  // Check if autoRestart is still enabled (might have been disabled)
  if (!project.autoRestart) {
    console.log(`[Restart] Auto-restart disabled for project ${projectId}, skipping restart`);
    return;
  }

  // Check if project is a service (only services should auto-restart on file changes)
  if (project.projectType !== 'service') {
    console.log(`[Restart] Project ${projectId} is a task, not restarting on file change`);
    return;
  }

  // Emit log message to renderer about the restart
  if (changedFile) {
    const logMessage = `[FileWatcher] File changed: ${changedFile}, triggering restart...\n`;
    emitRestartLog(projectId, logMessage);
  }

  console.log(`[Restart] Restarting project ${projectId} due to file change`);

  // Stop the process if running
  if (state.processes.has(projectId)) {
    try {
      await stopProcess(projectId);
      // Wait for process to fully terminate
      await new Promise((resolve) => setTimeout(resolve, 500));
    } catch (error) {
      console.error(`[Restart] Error stopping process ${projectId}:`, error);
    }
  }

  // Check if process was manually stopped while we were waiting
  const managed = state.processes.get(projectId);
  if (managed?.manuallyStopped) {
    console.log(`[Restart] Process ${projectId} was manually stopped, skipping restart`);
    return;
  }

  // Resolve working directory
  const path = await import('path');
  const workingDir = project.cwd
    ? path.isAbsolute(project.cwd)
      ? project.cwd
      : path.resolve(group.directory, project.cwd)
    : group.directory;

  // Merge environment variables
  const envVars = { ...group.envVars, ...project.envVars };

  // Spawn the process again
  try {
    await spawnProcess(
      mainWindowRef,
      projectId,
      groupId,
      project.command,
      workingDir,
      envVars,
      project.autoRestart,
      project.projectType,
      project.interactive
    );
    console.log(`[Restart] Successfully restarted project ${projectId}`);
  } catch (error) {
    console.error(`[Restart] Error restarting process ${projectId}:`, error);
  }
}
