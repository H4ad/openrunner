/**
 * Shell API for Electron.
 * This module provides a unified API for shell operations.
 */

/**
 * Get the Electron API
 */
function getElectronAPI(): ElectronAPI {
  if (!window.electron) {
    throw new Error('Electron API not available');
  }
  return window.electron;
}

/**
 * Open a URL or file in the default system application.
 * For URLs, this opens the default browser.
 * For files, this opens the associated application.
 */
export async function open(uri: string): Promise<void> {
  await getElectronAPI().invoke('shell:open-external', { uri });
}
