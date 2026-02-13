/**
 * Settings-related IPC handlers
 * Ported from: src-tauri/src/commands/get_settings.rs, update_settings.rs, etc.
 */

import { ipcMain, app } from 'electron';
import { execSync } from 'child_process';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import { detectUserShell } from '../platform';
import { enableAutostart, disableAutostart } from '../services/linux-autostart';
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
      const oldSettings = state.database.getSettings();
      
      // Update auto-launch setting if changed
      if (args.settings.autoLaunch !== oldSettings.autoLaunch) {
        if (process.platform === 'linux') {
          // Linux: Use XDG autostart (Electron's native API doesn't work reliably on Linux)
          if (args.settings.autoLaunch) {
            enableAutostart(args.settings.minimizeToTray);
          } else {
            disableAutostart();
          }
        } else {
          // macOS/Windows: Use Electron's native API
          app.setLoginItemSettings({
            openAtLogin: args.settings.autoLaunch,
            // Start hidden (minimized to tray) if minimizeToTray is enabled
            openAsHidden: args.settings.minimizeToTray,
          });
        }
      }
      
      // On Linux, also update autostart if minimizeToTray changes while autoLaunch is enabled
      // This ensures the --hidden flag is updated in the desktop file
      if (process.platform === 'linux' && 
          args.settings.autoLaunch && 
          args.settings.minimizeToTray !== oldSettings.minimizeToTray) {
        enableAutostart(args.settings.minimizeToTray);
      }
      
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

  // Detect system shell
  ipcMain.handle(
    IPC_CHANNELS.DETECT_SYSTEM_SHELL,
    async (): Promise<string> => {
      return detectUserShell();
    }
  );
}
