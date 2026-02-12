/**
 * Main process entry point for Electron.
 * This is the equivalent of src-tauri/src/lib.rs
 */

import * as Sentry from "@sentry/electron/main";

Sentry.init({
  dsn: "https://ef5067aac9111e8769d07679e969a8e1@o4505726434541568.ingest.us.sentry.io/4510872960499712",
  enableLogs: true,
  integrations: [
    Sentry.consoleLoggingIntegration({ levels: ["warn", "error"] }),
  ],
});

import { app, BrowserWindow, shell, ipcMain, dialog, globalShortcut, nativeImage } from 'electron';
import { join } from 'path';
import { electronApp, optimizer, is } from '@electron-toolkit/utils';
import { registerAllHandlers } from './ipc';
import { setMainWindow as setProcessMainWindow } from './ipc/processes';
import { setRestartMainWindow } from './services/process-manager/restart';
import { initializeState, getState, shutdownState } from './services/state';
import { startStatsCollection, stopStatsCollection } from './services/stats-collector';
import { getYamlWatcher } from './services/yaml-watcher';
import { initAutoUpdater, checkForUpdates } from './services/auto-updater';
import { IPC_EVENTS, IPC_CHANNELS } from '../shared/events';

// Keep a global reference to prevent garbage collection
let mainWindow: BrowserWindow | null = null;

// Set the app name to match the productName/StartupWMClass for proper Linux taskbar icon association
// This must be done before the app is ready for GNOME/KDE to correctly match the .desktop file
app.name = 'openrunner';

/**
 * Create the main application window
 */
function createWindow(): void {
  // Load app icon - check multiple paths for dev vs production
  let appIcon: Electron.NativeImage | undefined;
  const possiblePaths = [
    // Development: relative to out/main/index.js
    join(__dirname, '../../build/icon.png'),
    // Production: in resources directory
    join(process.resourcesPath, 'build/icon.png'),
  ];
  
  for (const iconPath of possiblePaths) {
    try {
      const tempIcon = nativeImage.createFromPath(iconPath);
      if (!tempIcon.isEmpty()) {
        appIcon = tempIcon;
        break;
      }
    } catch {
      // Try next path
    }
  }

  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    show: false,
    autoHideMenuBar: true,
    title: 'OpenRunner',
    icon: appIcon,
    webPreferences: {
      preload: join(__dirname, '../preload/index.mjs'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  mainWindow.on('ready-to-show', () => {
    mainWindow?.show();
    
    // Restore fullscreen state from settings
    const state = getState();
    const settings = state.database.getSettings();
    if (settings.fullscreen) {
      mainWindow?.setFullScreen(true);
    }
  });

  // Register F11 shortcut for fullscreen toggle
  mainWindow.on('focus', () => {
    globalShortcut.register('F11', () => {
      if (mainWindow) {
        const isFullscreen = mainWindow.isFullScreen();
        mainWindow.setFullScreen(!isFullscreen);
      }
    });
  });

  mainWindow.on('blur', () => {
    globalShortcut.unregister('F11');
  });

  // Save fullscreen state when changed
  mainWindow.on('enter-full-screen', () => {
    const state = getState();
    const settings = state.database.getSettings();
    settings.fullscreen = true;
    state.database.updateSettings(settings);
  });

  mainWindow.on('leave-full-screen', () => {
    const state = getState();
    const settings = state.database.getSettings();
    settings.fullscreen = false;
    state.database.updateSettings(settings);
  });

  // Handle external links
  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url);
    return { action: 'deny' };
  });

  // Configure CSP for Sentry Session Replay (requires WebWorker support)
  mainWindow.webContents.session.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; " +
          "script-src 'self' 'unsafe-inline' 'unsafe-eval'; " +
          "style-src 'self' 'unsafe-inline'; " +
          "img-src 'self' data: blob:; " +
          "font-src 'self'; " +
          "connect-src 'self' https://*.sentry.io https://*.ingest.sentry.io; " +
          "worker-src 'self' blob:; " +
          "child-src 'self' blob:;"
        ]
      }
    });
  });

  // Load the app
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL']);
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'));
  }

  // Start stats collection when window is ready
  mainWindow.webContents.on('did-finish-load', () => {
    if (mainWindow) {
      startStatsCollection(mainWindow);
      
      // Set main window for process manager and yaml watcher
      setProcessMainWindow(mainWindow);
      setRestartMainWindow(mainWindow);
      getYamlWatcher().setMainWindow(mainWindow);
      
      // Sync YAML watchers for existing groups
      const state = getState();
      getYamlWatcher().syncWatchers(state.config.groups);

      // Initialize auto-updater
      initAutoUpdater(mainWindow);

      // Check for updates after a delay (10 seconds)
      setTimeout(() => {
        checkForUpdates().catch((error) => {
          console.error('[AutoUpdater] Initial update check failed:', error);
        });
      }, 10000);
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

/**
 * Register Electron-specific IPC handlers (dialogs, shell, etc.)
 */
function registerElectronHandlers(): void {
  // Dialog: Open file/folder picker
  ipcMain.handle('dialog:open', async (_, options: {
    directory?: boolean;
    multiple?: boolean;
    filters?: { name: string; extensions: string[] }[];
    defaultPath?: string;
  }) => {
    const properties: ('openFile' | 'openDirectory' | 'multiSelections')[] = [];
    
    if (options.directory) {
      properties.push('openDirectory');
    } else {
      properties.push('openFile');
    }
    
    if (options.multiple) {
      properties.push('multiSelections');
    }

    const result = await dialog.showOpenDialog({
      properties,
      filters: options.filters,
      defaultPath: options.defaultPath,
    });

    if (result.canceled) {
      return null;
    }

    return options.multiple ? result.filePaths : result.filePaths[0];
  });

  // Dialog: Save file picker
  ipcMain.handle('dialog:save', async (_, options: {
    filters?: { name: string; extensions: string[] }[];
    defaultPath?: string;
  }) => {
    const result = await dialog.showSaveDialog({
      filters: options.filters,
      defaultPath: options.defaultPath,
    });

    return result.canceled ? null : result.filePath;
  });

  // Shell: Open external URL
  ipcMain.handle('shell:open-external', async (_, uri: string) => {
    await shell.openExternal(uri);
  });
}

/**
 * Application initialization
 */
app.whenReady().then(async () => {
  // Set app user model id for Windows
  electronApp.setAppUserModelId('com.openrunner.app');

  // Initialize application state
  await initializeState();

  // Register IPC handlers
  registerAllHandlers();
  registerElectronHandlers();

  // Default open or close DevTools by F12 in development
  // and ignore CommandOrControl + R in production
  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window);
  });

  // Create the main window
  createWindow();

  // macOS: Re-create window when dock icon is clicked
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

/**
 * Handle window-all-closed event
 */
app.on('window-all-closed', () => {
  // On macOS, apps usually stay active until explicitly quit
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

/**
 * Handle app quit - graceful shutdown
 */
app.on('before-quit', async (event) => {
  // Notify renderer that app is closing
  if (mainWindow) {
    mainWindow.webContents.send(IPC_EVENTS.APP_CLOSING);
  }

  // Stop stats collection
  stopStatsCollection();

  // Graceful shutdown of all processes
  try {
    const state = getState();
    if (state) {
      // Import lifecycle dynamically to avoid circular deps
      const { gracefulShutdownAll } = await import('./services/process-manager/lifecycle');
      await gracefulShutdownAll(mainWindow);
      shutdownState();
    }
  } catch (error) {
    console.error('Error during shutdown:', error);
  }
});

/**
 * Export mainWindow getter for other modules
 */
export function getMainWindow(): BrowserWindow | null {
  return mainWindow;
}
