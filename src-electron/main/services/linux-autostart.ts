/**
 * Linux autostart management using XDG autostart specification.
 * Creates/removes .desktop files in ~/.config/autostart/
 * 
 * This is necessary because Electron's app.setLoginItemSettings() does not
 * work reliably on Linux, especially for AppImage installations.
 */

import { app } from 'electron';
import { existsSync, mkdirSync, writeFileSync, unlinkSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

const AUTOSTART_DIR = join(homedir(), '.config', 'autostart');
const DESKTOP_FILENAME = 'openrunner.desktop';

/**
 * Get the path to the currently running executable.
 * Handles AppImage, packaged apps, and dev mode.
 */
function getExecutablePath(): string {
  // For AppImage, APPIMAGE env var contains the actual AppImage path
  if (process.env.APPIMAGE) {
    return process.env.APPIMAGE;
  }
  // For packaged apps (deb, tar.gz) or dev mode
  return app.getPath('exe');
}

/**
 * Check if we're running from an AppImage
 */
function isAppImage(): boolean {
  return !!process.env.APPIMAGE;
}

/**
 * Generate the .desktop file content for autostart
 */
function generateDesktopEntry(hidden: boolean): string {
  const execPath = getExecutablePath();
  
  // For AppImage, include necessary environment variables
  // --no-sandbox is required for some Electron features in sandboxed environments
  let execCommand: string;
  if (isAppImage()) {
    execCommand = `env DESKTOPINTEGRATION=1 "${execPath}" --no-sandbox`;
  } else {
    execCommand = `"${execPath}"`;
  }

  // Add --hidden flag if starting minimized to tray
  if (hidden) {
    execCommand += ' --hidden';
  }

  return `[Desktop Entry]
Type=Application
Name=OpenRunner
Comment=Desktop process manager for local development
Exec=${execCommand}
Terminal=false
Categories=Development;
StartupWMClass=openrunner
X-GNOME-Autostart-enabled=true
`;
}

/**
 * Enable auto-start on Linux by creating a .desktop file in ~/.config/autostart/
 * @param startHidden - If true, the app will start minimized to system tray
 */
export function enableAutostart(startHidden: boolean = false): void {
  try {
    // Ensure autostart directory exists
    if (!existsSync(AUTOSTART_DIR)) {
      mkdirSync(AUTOSTART_DIR, { recursive: true });
    }

    const desktopPath = join(AUTOSTART_DIR, DESKTOP_FILENAME);
    const content = generateDesktopEntry(startHidden);
    
    writeFileSync(desktopPath, content, { mode: 0o644 });
    console.log(`[AutoStart] Enabled autostart: ${desktopPath}`);
  } catch (error) {
    console.error('[AutoStart] Failed to enable autostart:', error);
    throw error;
  }
}

/**
 * Disable auto-start on Linux by removing the .desktop file from ~/.config/autostart/
 */
export function disableAutostart(): void {
  try {
    const desktopPath = join(AUTOSTART_DIR, DESKTOP_FILENAME);
    
    if (existsSync(desktopPath)) {
      unlinkSync(desktopPath);
      console.log(`[AutoStart] Disabled autostart: removed ${desktopPath}`);
    }
  } catch (error) {
    console.error('[AutoStart] Failed to disable autostart:', error);
    throw error;
  }
}

/**
 * Check if auto-start is currently enabled on Linux
 */
export function isAutostartEnabled(): boolean {
  const desktopPath = join(AUTOSTART_DIR, DESKTOP_FILENAME);
  return existsSync(desktopPath);
}

/**
 * Sync autostart state with settings.
 * This should be called on app startup to ensure the .desktop file
 * matches the database setting (handles cases where setting was enabled
 * before this feature was implemented, or the file was manually deleted).
 */
export function syncAutostart(enabled: boolean, startHidden: boolean = false): void {
  if (process.platform !== 'linux') {
    return;
  }

  const currentlyEnabled = isAutostartEnabled();
  
  if (enabled && !currentlyEnabled) {
    // Setting is enabled but .desktop file doesn't exist - create it
    console.log('[AutoStart] Syncing: creating missing autostart entry');
    enableAutostart(startHidden);
  } else if (!enabled && currentlyEnabled) {
    // Setting is disabled but .desktop file exists - remove it
    console.log('[AutoStart] Syncing: removing stale autostart entry');
    disableAutostart();
  } else if (enabled && currentlyEnabled) {
    // Both enabled - ensure the content is up to date (e.g., hidden flag might have changed)
    console.log('[AutoStart] Syncing: updating autostart entry');
    enableAutostart(startHidden);
  }
}
