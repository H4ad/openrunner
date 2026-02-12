/**
 * Settings-related IPC handlers
 * Ported from: src-tauri/src/commands/get_settings.rs, update_settings.rs, etc.
 */

import { ipcMain } from 'electron';
import { execSync } from 'child_process';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import type { AppSettings } from '../../shared/types';

/** List of common editors to check for */
const COMMON_EDITORS = [
  'code',
  'cursor',
  'zed',
  'windsurf',
  'nvim',
  'vim',
  'nano',
  'emacs',
  'sublime_text',
  'gedit',
  'kate',
];

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
 * Detect system editor from environment or common installations
 */
function detectEditor(): string | null {
  // Check VISUAL env var first
  const visual = process.env.VISUAL;
  if (visual) {
    return visual;
  }

  // Check EDITOR env var
  const editor = process.env.EDITOR;
  if (editor) {
    return editor;
  }

  // Check for common editors
  for (const editorCmd of COMMON_EDITORS) {
    if (commandExists(editorCmd)) {
      return editorCmd;
    }
  }

  return null;
}

export function registerSettingsHandlers(): void {
  // Get settings
  ipcMain.handle(IPC_CHANNELS.GET_SETTINGS, async (): Promise<AppSettings> => {
    const state = getState();
    return state.database.getSettings();
  });

  // Update settings
  ipcMain.handle(
    IPC_CHANNELS.UPDATE_SETTINGS,
    async (
      _,
      args: {
        settings: AppSettings;
      }
    ): Promise<void> => {
      const state = getState();
      state.database.updateSettings(args.settings);
    }
  );

  // Detect system editor
  ipcMain.handle(
    IPC_CHANNELS.DETECT_SYSTEM_EDITOR,
    async (): Promise<string | null> => {
      return detectEditor();
    }
  );
}
