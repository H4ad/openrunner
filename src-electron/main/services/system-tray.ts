/**
 * System tray service for Electron.
 * Manages the system tray icon and context menu.
 */

import { Tray, Menu, BrowserWindow, app } from 'electron';
import { getTrayIcon } from './app-icon';

let tray: Tray | null = null;
let mainWindowRef: BrowserWindow | null = null;
let isQuitting = false;

/**
 * Build the context menu for the tray
 */
function buildContextMenu(): Electron.Menu {
  return Menu.buildFromTemplate([
    {
      label: 'Show OpenRunner',
      click: () => {
        showMainWindow();
      },
    },
    {
      type: 'separator',
    },
    {
      label: 'Quit',
      click: () => {
        quitApp();
      },
    },
  ]);
}

/**
 * Show the main window
 */
function showMainWindow(): void {
  if (mainWindowRef) {
    if (mainWindowRef.isMinimized()) {
      mainWindowRef.restore();
    }
    mainWindowRef.show();
    mainWindowRef.focus();
  }
}

/**
 * Quit the application (sets flag to bypass minimize to tray)
 */
export function quitApp(): void {
  isQuitting = true;
  app.quit();
}

/**
 * Check if the app is in quitting state
 */
export function isAppQuitting(): boolean {
  return isQuitting;
}

/**
 * Set the quitting state (called before app quit)
 */
export function setQuitting(value: boolean): void {
  isQuitting = value;
}

/**
 * Initialize the system tray
 */
export function initSystemTray(mainWindow: BrowserWindow): Tray {
  mainWindowRef = mainWindow;

  const icon = getTrayIcon();
  tray = new Tray(icon);

  tray.setToolTip('OpenRunner');
  tray.setContextMenu(buildContextMenu());

  // Click on tray icon shows the window
  tray.on('click', () => {
    showMainWindow();
  });

  // Double click also shows the window
  tray.on('double-click', () => {
    showMainWindow();
  });

  return tray;
}

/**
 * Destroy the system tray
 */
export function destroySystemTray(): void {
  if (tray) {
    tray.destroy();
    tray = null;
  }
  mainWindowRef = null;
}

/**
 * Get the tray instance
 */
export function getTray(): Tray | null {
  return tray;
}
