/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

// Electron API exposed by preload script
interface ElectronAPI {
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

interface Window {
  electron?: ElectronAPI;
}
