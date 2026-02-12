/**
 * Preload script for Electron.
 * This script runs in an isolated context with access to Node.js APIs.
 * It exposes a safe API to the renderer process via contextBridge.
 */

import { contextBridge, ipcRenderer } from 'electron';
import type { IpcRendererEvent } from 'electron';

/**
 * The API exposed to the renderer process via window.electron
 */
const electronAPI = {
  /**
   * Invoke an IPC handler in the main process (request/response pattern)
   * This is the Electron equivalent of Tauri's invoke()
   */
  invoke: <T>(channel: string, ...args: unknown[]): Promise<T> => {
    return ipcRenderer.invoke(channel, ...args);
  },

  /**
   * Listen for events from the main process
   * This is the Electron equivalent of Tauri's listen()
   * Returns an unlisten function
   */
  on: (channel: string, callback: (...args: unknown[]) => void): (() => void) => {
    const subscription = (_event: IpcRendererEvent, ...args: unknown[]) => {
      callback(...args);
    };
    ipcRenderer.on(channel, subscription);

    // Return unlisten function
    return () => {
      ipcRenderer.removeListener(channel, subscription);
    };
  },

  /**
   * Listen for an event once
   */
  once: (channel: string, callback: (...args: unknown[]) => void): void => {
    ipcRenderer.once(channel, (_event: IpcRendererEvent, ...args: unknown[]) => {
      callback(...args);
    });
  },

  /**
   * Send a message to the main process (fire and forget)
   */
  send: (channel: string, ...args: unknown[]): void => {
    ipcRenderer.send(channel, ...args);
  },

  /**
   * Platform information
   */
  platform: process.platform,

  /**
   * Node.js version
   */
  versions: {
    node: process.versions.node,
    electron: process.versions.electron,
    chrome: process.versions.chrome,
  },
};

// Expose the API to the renderer process
contextBridge.exposeInMainWorld('electron', electronAPI);

// Type declarations for the renderer process
export type ElectronAPI = typeof electronAPI;
