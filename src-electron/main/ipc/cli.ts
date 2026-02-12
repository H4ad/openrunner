/**
 * IPC handlers for CLI installation/uninstallation
 */

import { ipcMain } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import {
  getCliStatus,
  installCli,
  uninstallCli,
} from '../services/cli-installer';

export function registerCliHandlers(): void {
  // Get CLI installation status
  ipcMain.handle(IPC_CHANNELS.CLI_GET_STATUS, () => {
    try {
      return getCliStatus();
    } catch (error) {
      console.error('[IPC] Get CLI status failed:', error);
      throw error;
    }
  });

  // Install CLI command
  ipcMain.handle(IPC_CHANNELS.CLI_INSTALL, async () => {
    try {
      return await installCli();
    } catch (error) {
      console.error('[IPC] Install CLI failed:', error);
      throw error;
    }
  });

  // Uninstall CLI command
  ipcMain.handle(IPC_CHANNELS.CLI_UNINSTALL, async () => {
    try {
      return await uninstallCli();
    } catch (error) {
      console.error('[IPC] Uninstall CLI failed:', error);
      throw error;
    }
  });
}
