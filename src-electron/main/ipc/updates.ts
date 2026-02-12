/**
 * IPC handlers for auto-update functionality
 */

import { ipcMain, app } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import {
  checkForUpdates,
  downloadUpdate,
  installUpdate,
  getAppVersion,
  getUpdateState,
  isAutoUpdateSupported,
  openReleasePage,
} from '../services/auto-updater';

export function registerUpdatesHandlers(): void {
  // Check for updates
  ipcMain.handle(IPC_CHANNELS.CHECK_FOR_UPDATES, async () => {
    try {
      const state = await checkForUpdates();
      return {
        ...state,
        currentVersion: getAppVersion(),
        autoUpdateSupported: isAutoUpdateSupported(),
      };
    } catch (error) {
      console.error('[IPC] Check for updates failed:', error);
      throw error;
    }
  });

  // Download the available update
  ipcMain.handle(IPC_CHANNELS.DOWNLOAD_UPDATE, async () => {
    try {
      // If auto-update is not supported (macOS without code signing),
      // open the releases page instead
      if (!isAutoUpdateSupported()) {
        await openReleasePage();
        return { openedReleasePage: true };
      }

      await downloadUpdate();
      return { success: true };
    } catch (error) {
      console.error('[IPC] Download update failed:', error);
      throw error;
    }
  });

  // Install the downloaded update and restart
  ipcMain.handle(IPC_CHANNELS.INSTALL_UPDATE, async () => {
    try {
      installUpdate();
      return { success: true };
    } catch (error) {
      console.error('[IPC] Install update failed:', error);
      throw error;
    }
  });

  // Get current app version and update state
  ipcMain.handle(IPC_CHANNELS.GET_APP_VERSION, () => {
    return {
      version: getAppVersion(),
      updateState: getUpdateState(),
      autoUpdateSupported: isAutoUpdateSupported(),
    };
  });

  // Check if running in development mode (app not packaged)
  ipcMain.handle(IPC_CHANNELS.IS_DEV_MODE, () => {
    return !app.isPackaged;
  });
}
