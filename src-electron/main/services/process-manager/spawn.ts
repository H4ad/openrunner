/**
 * Process spawning functionality.
 * This is the equivalent of src-tauri/src/process/spawn.rs
 */

import { spawn, ChildProcess } from 'child_process';
import { appendFileSync, writeFileSync } from 'fs';
import type { BrowserWindow } from 'electron';
import * as pty from 'node-pty';
import type { IPty } from 'node-pty';
import type { ProcessStatus, LogStream, ManagedProcess } from '../../../shared/types';
import { getState } from '../state';
import { getPlatformManager } from '../../platform';
import { emitStatusUpdate } from './index';
import { IPC_EVENTS } from '../../../shared/events';

/**
 * Spawn a regular (non-interactive) process using pipes
 */
export async function spawnRegularProcess(
  mainWindow: BrowserWindow | null,
  projectId: string,
  groupId: string,
  command: string,
  workingDir: string,
  envVars: Record<string, string>,
  sessionId: string,
  logPath: string
): Promise<void> {
  const state = getState();
  const platform = getPlatformManager();
  const settings = state.database.getSettings();
  const { shell, args } = platform.getShellCommand(settings.shell);

  // Prepare environment with color forcing
  const env = {
    ...process.env,
    ...envVars,
    FORCE_COLOR: '1',
    CLICOLOR_FORCE: '1',
  };

  // Get platform-specific spawn options
  const platformOptions = platform.getSpawnOptions();

  // Spawn the process
  const child = spawn(shell, [...args, command], {
    cwd: workingDir,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
    ...platformOptions,
  } as Parameters<typeof spawn>[2]);

  const pid = child.pid ?? null;

  // Save the PID to disk for orphan cleanup on restart
  if (pid) {
    state.savePid(pid);
  }

  // Set up stdout reader
  if (child.stdout) {
    child.stdout.on('data', (data: Buffer) => {
      const text = data.toString();
      const timestamp = Date.now();

      // Append to log file
      try {
        appendFileSync(logPath, text);
      } catch {
        // Ignore errors
      }

      // Write to database (non-interactive only)
      try {
        state.database.insertLog(sessionId, 'stdout', text, timestamp);
      } catch {
        // Ignore errors
      }

      // Emit to renderer
      if (mainWindow) {
        mainWindow.webContents.send(IPC_EVENTS.PROCESS_LOG, {
          projectId,
          stream: 'stdout' as LogStream,
          data: text,
          timestamp,
        });
      }
    });
  }

  // Set up stderr reader
  if (child.stderr) {
    child.stderr.on('data', (data: Buffer) => {
      const text = data.toString();
      const timestamp = Date.now();

      // Append to log file
      try {
        appendFileSync(logPath, text);
      } catch {
        // Ignore errors
      }

      // Write to database (non-interactive only)
      try {
        state.database.insertLog(sessionId, 'stderr', text, timestamp);
      } catch {
        // Ignore errors
      }

      // Emit to renderer
      if (mainWindow) {
        mainWindow.webContents.send(IPC_EVENTS.PROCESS_LOG, {
          projectId,
          stream: 'stderr' as LogStream,
          data: text,
          timestamp,
        });
      }
    });
  }

  // Store process
  const managed: ManagedProcess = {
    child,
    ptyProcess: null,
    manuallyStopped: false,
    sessionId,
    groupId,
    isInteractive: false,
    realPid: pid,
  };
  state.processes.set(projectId, managed);

  // Update process info
  state.processInfos.set(projectId, {
    projectId,
    status: 'running' as ProcessStatus,
    pid,
    cpuUsage: 0,
    memoryUsage: 0,
  });

  // Emit status update
  emitStatusUpdate(mainWindow, projectId, 'running', pid);
}

/**
 * Spawn an interactive process using PTY
 */
export async function spawnInteractiveProcess(
  mainWindow: BrowserWindow | null,
  projectId: string,
  groupId: string,
  command: string,
  workingDir: string,
  envVars: Record<string, string>,
  sessionId: string,
  logPath: string,
  cols?: number,
  rows?: number
): Promise<void> {
  const state = getState();
  const platform = getPlatformManager();
  const settings = state.database.getSettings();
  const { shell, args } = platform.getShellCommand(settings.shell);

  // Prepare environment with color forcing
  const env = {
    ...process.env,
    ...envVars,
    FORCE_COLOR: '1',
    CLICOLOR_FORCE: '1',
    TERM: 'xterm-256color',
  };

  // Use provided dimensions or default to 80x24
  const initialCols = cols ?? 80;
  const initialRows = rows ?? 24;

  // Spawn the PTY process
  const ptyProcess = pty.spawn(shell, [...args, command], {
    name: 'xterm-256color',
    cols: initialCols,
    rows: initialRows,
    cwd: workingDir,
    env: env as Record<string, string>,
  });

  const pid = ptyProcess.pid;

  // Save the PID to disk for orphan cleanup on restart
  if (pid) {
    state.savePid(pid);
  }

  // Set up data reader
  ptyProcess.onData((data: string) => {
    const timestamp = Date.now();

    // Append to log file
    try {
      appendFileSync(logPath, data);
    } catch {
      // Ignore errors
    }

    // Note: PTY (interactive) process logs are NOT stored in SQLite
    // to avoid flooding the database with terminal output (e.g., vim, htop)

    // Emit to renderer
    if (mainWindow) {
      mainWindow.webContents.send(IPC_EVENTS.PROCESS_LOG, {
        projectId,
        stream: 'stdout' as LogStream,
        data,
        timestamp,
      });
    }
  });

  // Store process
  const managed: ManagedProcess = {
    child: null,
    ptyProcess,
    manuallyStopped: false,
    sessionId,
    groupId,
    isInteractive: true,
    realPid: pid,
  };
  state.processes.set(projectId, managed);

  // Update process info
  state.processInfos.set(projectId, {
    projectId,
    status: 'running' as ProcessStatus,
    pid,
    cpuUsage: 0,
    memoryUsage: 0,
  });

  // Emit status update
  emitStatusUpdate(mainWindow, projectId, 'running', pid);
}
