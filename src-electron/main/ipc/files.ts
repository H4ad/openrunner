/**
 * File-related IPC handlers
 * Ported from: src-tauri/src/commands/read_project_logs.rs, open_file_in_editor.rs, etc.
 */

import * as fs from 'fs';
import * as path from 'path';
import { spawn, execSync } from 'child_process';
import { ipcMain, shell } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import type { LogMessage } from '../../shared/types';

/**
 * Check if a command exists on the system
 */
function commandExists(command: string): boolean {
  try {
    const checkCmd =
      process.platform === 'win32' ? `where ${command}` : `which ${command}`;
    execSync(checkCmd, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Resolve the working directory for a project
 */
function resolveWorkingDir(groupDir: string, projectCwd: string | null): string {
  if (!projectCwd) {
    return groupDir;
  }

  // If projectCwd is absolute, use it directly
  if (path.isAbsolute(projectCwd)) {
    return projectCwd;
  }

  // Otherwise, resolve relative to group directory
  return path.resolve(groupDir, projectCwd);
}

export function registerFileHandlers(): void {
  // Read project logs from file
  ipcMain.handle(
    IPC_CHANNELS.READ_PROJECT_LOGS,
    async (
      _,
      args: {
        groupId: string;
        projectId: string;
      }
    ): Promise<LogMessage[]> => {
      const state = getState();

      // Try to get logs from the most recent session
      const logs = state.database.getRecentLogs(args.projectId, 1000);
      if (logs.length > 0) {
        return logs;
      }

      // Fall back to log file if exists
      const logFilePath = state.logFilePath(args.projectId);
      if (fs.existsSync(logFilePath)) {
        try {
          const content = fs.readFileSync(logFilePath, 'utf-8');
          return [
            {
              projectId: args.projectId,
              stream: 'stdout',
              data: content,
              timestamp: Date.now(),
            },
          ];
        } catch {
          // Ignore read errors
        }
      }

      return [];
    }
  );

  // Clear project logs
  ipcMain.handle(
    IPC_CHANNELS.CLEAR_PROJECT_LOGS,
    async (
      _,
      args: {
        groupId: string;
        projectId: string;
      }
    ): Promise<void> => {
      const state = getState();

      // Clear logs from database
      state.database.clearProjectLogs(args.projectId);

      // Clear log file if exists
      const logFilePath = state.logFilePath(args.projectId);
      if (fs.existsSync(logFilePath)) {
        fs.writeFileSync(logFilePath, '');
      }
    }
  );

  // Resolve project working directory
  ipcMain.handle(
    IPC_CHANNELS.RESOLVE_PROJECT_WORKING_DIR,
    async (
      _,
      args: {
        groupId: string;
        projectId: string;
      }
    ): Promise<string> => {
      const state = getState();
      const result = state.findProject(args.projectId);

      if (!result) {
        throw new Error(`Project not found: ${args.projectId}`);
      }

      return resolveWorkingDir(result.group.directory, result.project.cwd);
    }
  );

  // Resolve working directory by project ID
  ipcMain.handle(
    IPC_CHANNELS.RESOLVE_WORKING_DIR_BY_PROJECT,
    async (
      _,
      args: {
        projectId: string;
      }
    ): Promise<string> => {
      const state = getState();
      const result = state.findProject(args.projectId);

      if (!result) {
        throw new Error(`Project not found: ${args.projectId}`);
      }

      return resolveWorkingDir(result.group.directory, result.project.cwd);
    }
  );

  // Open file in editor
  ipcMain.handle(
    IPC_CHANNELS.OPEN_FILE_IN_EDITOR,
    async (
      _,
      args: {
        filePath: string;
        line?: number;
        column?: number;
        workingDir?: string;
      }
    ): Promise<void> => {
      const state = getState();
      const settings = state.database.getSettings();

      let editor = settings.editor;
      if (!editor) {
        // Try to detect system editor
        const envEditor = process.env.VISUAL || process.env.EDITOR;
        if (envEditor) {
          editor = envEditor;
        } else {
          // Check for common editors
          const commonEditors = [
            'code',
            'cursor',
            'zed',
            'windsurf',
            'nvim',
            'vim',
          ];
          for (const e of commonEditors) {
            if (commandExists(e)) {
              editor = e;
              break;
            }
          }
        }
      }

      if (!editor) {
        throw new Error(
          'No editor configured. Set an editor in settings or install VS Code, Cursor, or Zed.'
        );
      }

      // Build editor arguments
      const editorArgs: string[] = [];

      // Handle line/column for common editors
      if (args.line !== undefined) {
        const editorLower = editor.toLowerCase();
        if (
          editorLower.includes('code') ||
          editorLower.includes('cursor') ||
          editorLower.includes('windsurf')
        ) {
          // VS Code / Cursor format: --goto file:line:column
          const location = args.column
            ? `${args.filePath}:${args.line}:${args.column}`
            : `${args.filePath}:${args.line}`;
          editorArgs.push('--goto', location);
        } else if (editorLower.includes('zed')) {
          // Zed format: file:line:column
          const location = args.column
            ? `${args.filePath}:${args.line}:${args.column}`
            : `${args.filePath}:${args.line}`;
          editorArgs.push(location);
        } else if (
          editorLower.includes('vim') ||
          editorLower.includes('nvim')
        ) {
          // Vim format: +line file
          editorArgs.push(`+${args.line}`, args.filePath);
        } else {
          // Default: just open the file
          editorArgs.push(args.filePath);
        }
      } else {
        editorArgs.push(args.filePath);
      }

      // Spawn editor
      const cwd = args.workingDir || process.cwd();
      spawn(editor, editorArgs, {
        cwd,
        detached: true,
        stdio: 'ignore',
      }).unref();
    }
  );

  // Open path in system file manager
  ipcMain.handle(
    IPC_CHANNELS.OPEN_PATH,
    async (
      _,
      args: {
        path: string;
      }
    ): Promise<void> => {
      await shell.openPath(args.path);
    }
  );

  // Open path in terminal
  ipcMain.handle(
    IPC_CHANNELS.OPEN_IN_TERMINAL,
    async (
      _,
      args: {
        path: string;
      }
    ): Promise<void> => {
      const resolvedPath = path.resolve(args.path);

      if (!fs.existsSync(resolvedPath)) {
        throw new Error(`Path not found: ${args.path}`);
      }

      if (process.platform === 'linux') {
        // Try common Linux terminals
        const terminals: [string, string[]][] = [
          ['kitty', ['--directory', resolvedPath]],
          ['alacritty', ['--working-directory', resolvedPath]],
          ['wezterm', ['start', '--cwd', resolvedPath]],
          ['gnome-terminal', ['--working-directory', resolvedPath]],
          ['konsole', ['--workdir', resolvedPath]],
          ['xfce4-terminal', ['--working-directory', resolvedPath]],
          ['mate-terminal', ['--working-directory', resolvedPath]],
          ['lxterminal', ['--working-directory', resolvedPath]],
          ['terminator', ['--working-directory', resolvedPath]],
          ['tilix', ['--working-directory', resolvedPath]],
        ];

        for (const [terminal, termArgs] of terminals) {
          if (commandExists(terminal)) {
            spawn(terminal, termArgs, {
              detached: true,
              stdio: 'ignore',
            }).unref();
            return;
          }
        }

        throw new Error(
          'No supported terminal found. Please install kitty, alacritty, wezterm, gnome-terminal, konsole, or xfce4-terminal.'
        );
      } else if (process.platform === 'darwin') {
        // macOS
        const iTermPath = '/Applications/iTerm.app';
        if (fs.existsSync(iTermPath)) {
          spawn('open', ['-a', 'iTerm', resolvedPath], {
            detached: true,
            stdio: 'ignore',
          }).unref();
          return;
        }

        const wezTermPath = '/Applications/WezTerm.app';
        if (fs.existsSync(wezTermPath)) {
          spawn('open', ['-a', 'WezTerm', resolvedPath], {
            detached: true,
            stdio: 'ignore',
          }).unref();
          return;
        }

        if (commandExists('kitty')) {
          spawn('kitty', ['--directory', resolvedPath], {
            detached: true,
            stdio: 'ignore',
          }).unref();
          return;
        }

        if (commandExists('alacritty')) {
          spawn('alacritty', ['--working-directory', resolvedPath], {
            detached: true,
            stdio: 'ignore',
          }).unref();
          return;
        }

        // Fall back to Terminal.app
        spawn('open', ['-a', 'Terminal', resolvedPath], {
          detached: true,
          stdio: 'ignore',
        }).unref();
      } else if (process.platform === 'win32') {
        // Windows
        if (commandExists('wt')) {
          spawn('wt', ['-d', resolvedPath], {
            detached: true,
            stdio: 'ignore',
          }).unref();
          return;
        }

        // Fall back to cmd.exe
        spawn('cmd', ['/C', 'start', 'cmd', '/K', 'cd', '/d', resolvedPath], {
          detached: true,
          stdio: 'ignore',
        }).unref();
      }
    }
  );
}
