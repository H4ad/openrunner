/**
 * Process-related IPC handlers
 * Ported from: src-tauri/src/commands/start_process.rs, stop_process.rs, etc.
 */

import * as path from 'path';
import { ipcMain, BrowserWindow } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import { spawnProcess } from '../services/process-manager';
import { stopProcess } from '../services/process-manager/lifecycle';
import { writeToProcessStdin, resizePty } from '../services/process-manager/io';
import type { ProcessInfo } from '../../shared/types';

// Store reference to main window for process spawning
let mainWindowRef: BrowserWindow | null = null;

export function setMainWindow(window: BrowserWindow): void {
  mainWindowRef = window;
}

/**
 * Resolve working directory for a project
 */
function resolveWorkingDir(
  groupDir: string,
  projectCwd: string | null
): string {
  if (!projectCwd) {
    return groupDir;
  }

  if (path.isAbsolute(projectCwd)) {
    return projectCwd;
  }

  return path.resolve(groupDir, projectCwd);
}

export function registerProcessHandlers(): void {
  // Start a process
  ipcMain.handle(
    IPC_CHANNELS.START_PROCESS,
    async (
      _,
      args: {
        groupId: string;
        projectId: string;
        cols?: number;
        rows?: number;
      }
    ): Promise<void> => {
      const state = getState();
      const { groupId, projectId, cols, rows } = args;

      // Find the group and project
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      const project = group.projects.find((p) => p.id === projectId);
      if (!project) {
        throw new Error(`Project not found: ${projectId}`);
      }

      // Resolve working directory
      const workingDir = resolveWorkingDir(group.directory, project.cwd);

      // Merge environment variables (group + project)
      const envVars = { ...group.envVars, ...project.envVars };

      // Spawn the process
      await spawnProcess(
        mainWindowRef,
        projectId,
        groupId,
        project.command,
        workingDir,
        envVars,
        project.autoRestart,
        project.projectType,
        project.interactive,
        cols,
        rows
      );
    }
  );

  // Stop a process
  ipcMain.handle(
    IPC_CHANNELS.STOP_PROCESS,
    async (
      _,
      args: {
        projectId: string;
      }
    ): Promise<void> => {
      await stopProcess(args.projectId);
    }
  );

  // Restart a process
  ipcMain.handle(
    IPC_CHANNELS.RESTART_PROCESS,
    async (
      _,
      args: {
        groupId: string;
        projectId: string;
      }
    ): Promise<void> => {
      const state = getState();
      const { groupId, projectId } = args;

      // Find the group and project
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      const project = group.projects.find((p) => p.id === projectId);
      if (!project) {
        throw new Error(`Project not found: ${projectId}`);
      }

      // Stop the process if running
      if (state.processes.has(projectId)) {
        await stopProcess(projectId);
        // Wait a bit for the process to fully terminate
        await new Promise((resolve) => setTimeout(resolve, 500));
      }

      // Resolve working directory
      const workingDir = resolveWorkingDir(group.directory, project.cwd);

      // Merge environment variables (group + project)
      const envVars = { ...group.envVars, ...project.envVars };

      // Spawn the process again
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
    }
  );

  // Get all process statuses
  ipcMain.handle(
    IPC_CHANNELS.GET_ALL_STATUSES,
    async (): Promise<ProcessInfo[]> => {
      const state = getState();
      return Array.from(state.processInfos.values());
    }
  );

  // Write to process stdin (for interactive processes)
  ipcMain.handle(
    IPC_CHANNELS.WRITE_TO_PROCESS_STDIN,
    async (
      _,
      args: {
        projectId: string;
        data: string;
      }
    ): Promise<void> => {
      const state = getState();
      // Silently ignore if process is not running (frontend will catch error anyway)
      if (!state.processes.has(args.projectId)) {
        return;
      }
      writeToProcessStdin(args.projectId, args.data);
    }
  );

  // Resize PTY (for interactive processes)
  ipcMain.handle(
    IPC_CHANNELS.RESIZE_PTY,
    async (
      _,
      args: {
        projectId: string;
        cols: number;
        rows: number;
      }
    ): Promise<void> => {
      const state = getState();
      // Silently ignore if process is not running
      if (!state.processes.has(args.projectId)) {
        return;
      }
      resizePty(args.projectId, args.cols, args.rows);
    }
  );
}
