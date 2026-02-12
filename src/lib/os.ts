/**
 * OS API for Electron.
 * This module provides a unified API for OS-related functions.
 */

export type OsType = 'linux' | 'darwin' | 'windows_nt';

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
 * Get the operating system type.
 * Returns 'linux', 'darwin' (macOS), or 'windows_nt' (Windows).
 */
export async function type(): Promise<OsType> {
  const p = getElectronAPI().platform;
  switch (p) {
    case 'linux':
      return 'linux';
    case 'darwin':
      return 'darwin';
    case 'win32':
      return 'windows_nt';
    default:
      return 'linux';
  }
}
