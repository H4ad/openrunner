/**
 * Type declarations for the preload API exposed to the renderer.
 * This file should be imported in the renderer to get proper types.
 */

export interface ElectronAPI {
  invoke: <T>(channel: string, ...args: unknown[]) => Promise<T>;
  on: (channel: string, callback: (...args: unknown[]) => void) => () => void;
  once: (channel: string, callback: (...args: unknown[]) => void) => void;
  send: (channel: string, ...args: unknown[]) => void;
  platform: NodeJS.Platform;
  versions: {
    node: string;
    electron: string;
    chrome: string;
  };
}

declare global {
  interface Window {
    electron: ElectronAPI;
  }
}
