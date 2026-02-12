/**
 * Window-related IPC handlers
 * Handles window state operations like fullscreen toggle
 */

import { ipcMain, BrowserWindow } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getMainWindow } from '../index';

export function registerWindowHandlers(): void {
  // Toggle fullscreen mode
  ipcMain.handle(IPC_CHANNELS.WINDOW_TOGGLE_FULLSCREEN, async () => {
    const mainWindow = getMainWindow();
    if (mainWindow) {
      const isFullscreen = mainWindow.isFullScreen();
      mainWindow.setFullScreen(!isFullscreen);
      return !isFullscreen;
    }
    return false;
  });

  // Get current fullscreen state
  ipcMain.handle(IPC_CHANNELS.WINDOW_GET_FULLSCREEN, async () => {
    const mainWindow = getMainWindow();
    if (mainWindow) {
      return mainWindow.isFullScreen();
    }
    return false;
  });
}
