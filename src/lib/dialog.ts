/**
 * Dialog API for Electron.
 * This module provides a unified API for file/folder dialogs.
 */

export interface OpenDialogOptions {
  directory?: boolean;
  multiple?: boolean;
  filters?: { name: string; extensions: string[] }[];
  defaultPath?: string;
  title?: string;
}

export interface SaveDialogOptions {
  filters?: { name: string; extensions: string[] }[];
  defaultPath?: string;
  title?: string;
}

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
 * Open a file or directory picker dialog.
 * Returns the selected path(s) or null if cancelled.
 */
export async function open(options: OpenDialogOptions = {}): Promise<string | string[] | null> {
  return getElectronAPI().invoke<string | string[] | null>('dialog:open', options);
}

/**
 * Open a save file dialog.
 * Returns the selected path or null if cancelled.
 */
export async function save(options: SaveDialogOptions = {}): Promise<string | null> {
  return getElectronAPI().invoke<string | null>('dialog:save', options);
}
