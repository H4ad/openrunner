/**
 * Auto-updater service using electron-updater with GitHub Releases
 */

import type { UpdateInfo, ProgressInfo } from 'electron-updater';
const electronUpdater = require('electron-updater');
import { app, BrowserWindow, shell } from 'electron';
import { IPC_EVENTS } from '../../shared/events';

// Update state shared with IPC handlers
export interface UpdateState {
  checking: boolean;
  available: boolean;
  downloading: boolean;
  progress: number;
  downloaded: boolean;
  error: string | null;
  version: string | null;
  releaseNotes: string | null;
  releaseDate: string | null;
}

let updateState: UpdateState = {
  checking: false,
  available: false,
  downloading: false,
  progress: 0,
  downloaded: false,
  error: null,
  version: null,
  releaseNotes: null,
  releaseDate: null,
};

let mainWindow: BrowserWindow | null = null;

/**
 * Get the current update state
 */
export function getUpdateState(): UpdateState {
  return { ...updateState };
}

/**
 * Send update event to renderer process
 */
function sendUpdateEvent(event: string, data?: unknown): void {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(event, data);
  }
}

/**
 * Reset update state to initial values
 */
function resetState(): void {
  updateState = {
    checking: false,
    available: false,
    downloading: false,
    progress: 0,
    downloaded: false,
    error: null,
    version: null,
    releaseNotes: null,
    releaseDate: null,
  };
}

/**
 * Initialize the auto-updater service
 */
export function initAutoUpdater(window: BrowserWindow): void {
  mainWindow = window;

  // Configure auto-updater
  electronUpdater.autoUpdater.autoDownload = false; // User-initiated downloads
  electronUpdater.autoUpdater.autoInstallOnAppQuit = true;
  electronUpdater.autoUpdater.autoRunAppAfterInstall = true;

  // Disable auto-updater in development
  if (!app.isPackaged) {
    electronUpdater.autoUpdater.forceDevUpdateConfig = false;
    console.log('[AutoUpdater] Running in development mode, auto-updates disabled');
  }

  // Event: Checking for updates
  electronUpdater.autoUpdater.on('checking-for-update', () => {
    console.log('[AutoUpdater] Checking for updates...');
    updateState.checking = true;
    updateState.error = null;
    sendUpdateEvent(IPC_EVENTS.UPDATE_CHECKING);
  });

  // Event: Update available
  electronUpdater.autoUpdater.on('update-available', (info: UpdateInfo) => {
    console.log('[AutoUpdater] Update available:', info.version);
    updateState.checking = false;
    updateState.available = true;
    updateState.version = info.version;
    updateState.releaseDate = info.releaseDate ? String(info.releaseDate) : null;
    updateState.releaseNotes = typeof info.releaseNotes === 'string'
      ? info.releaseNotes
      : Array.isArray(info.releaseNotes)
        ? info.releaseNotes.map((n) => n.note).join('\n')
        : null;

    sendUpdateEvent(IPC_EVENTS.UPDATE_AVAILABLE, {
      version: info.version,
      releaseDate: updateState.releaseDate,
      releaseNotes: updateState.releaseNotes,
    });
  });

  // Event: No update available
  electronUpdater.autoUpdater.on('update-not-available', (info: UpdateInfo) => {
    console.log('[AutoUpdater] No update available. Current version:', info.version);
    updateState.checking = false;
    updateState.available = false;
    sendUpdateEvent(IPC_EVENTS.UPDATE_NOT_AVAILABLE, {
      version: info.version,
    });
  });

  // Event: Download progress
  electronUpdater.autoUpdater.on('download-progress', (progress: ProgressInfo) => {
    console.log(`[AutoUpdater] Download progress: ${progress.percent.toFixed(1)}%`);
    updateState.downloading = true;
    updateState.progress = progress.percent;
    sendUpdateEvent(IPC_EVENTS.UPDATE_DOWNLOAD_PROGRESS, {
      percent: progress.percent,
      bytesPerSecond: progress.bytesPerSecond,
      transferred: progress.transferred,
      total: progress.total,
    });
  });

  // Event: Update downloaded
  electronUpdater.autoUpdater.on('update-downloaded', (info: UpdateInfo) => {
    console.log('[AutoUpdater] Update downloaded:', info.version);
    updateState.downloading = false;
    updateState.downloaded = true;
    updateState.progress = 100;
    sendUpdateEvent(IPC_EVENTS.UPDATE_DOWNLOADED, {
      version: info.version,
      releaseNotes: updateState.releaseNotes,
    });
  });

  // Event: Error
  electronUpdater.autoUpdater.on('error', (error: Error) => {
    console.error('[AutoUpdater] Error:', error.message);
    updateState.checking = false;
    updateState.downloading = false;
    updateState.error = error.message;
    sendUpdateEvent(IPC_EVENTS.UPDATE_ERROR, {
      message: error.message,
    });
  });
}

/**
 * Check for updates manually
 * Returns the update state after checking
 */
export async function checkForUpdates(): Promise<UpdateState> {
  if (!app.isPackaged) {
    console.log('[AutoUpdater] Skipping update check in development mode');
    return getUpdateState();
  }

  resetState();
  updateState.checking = true;

  try {
    await electronUpdater.autoUpdater.checkForUpdates();
  } catch (error) {
    console.error('[AutoUpdater] Check for updates failed:', error);
    updateState.checking = false;
    updateState.error = error instanceof Error ? error.message : 'Unknown error';
  }

  return getUpdateState();
}

/**
 * Download the available update
 */
export async function downloadUpdate(): Promise<void> {
  if (!updateState.available) {
    throw new Error('No update available to download');
  }

  // On macOS without code signing, we can't auto-update
  // Open the GitHub releases page instead
  if (process.platform === 'darwin' && !app.isPackaged) {
    await openReleasePage();
    return;
  }

  updateState.downloading = true;
  updateState.progress = 0;

  try {
    await electronUpdater.autoUpdater.downloadUpdate();
  } catch (error) {
    updateState.downloading = false;
    updateState.error = error instanceof Error ? error.message : 'Download failed';
    throw error;
  }
}

/**
 * Install the downloaded update and restart the app
 */
export function installUpdate(): void {
  if (!updateState.downloaded) {
    throw new Error('No update downloaded to install');
  }

  electronUpdater.autoUpdater.quitAndInstall(false, true);
}

/**
 * Open the GitHub releases page for manual download
 * Used for macOS when auto-update is not available
 */
export async function openReleasePage(): Promise<void> {
  const releasesUrl = 'https://github.com/h4ad/openrunner/releases/latest';
  await shell.openExternal(releasesUrl);
}

/**
 * Get the current app version
 */
export function getAppVersion(): string {
  return app.getVersion();
}

/**
 * Check if auto-update is supported on the current platform
 * macOS requires code signing for auto-updates
 */
export function isAutoUpdateSupported(): boolean {
  // In development, auto-update is not supported
  if (!app.isPackaged) {
    return false;
  }

  // On macOS, auto-update requires code signing
  // For now, we'll return false and show manual download option
  if (process.platform === 'darwin') {
    return false;
  }

  return true;
}
