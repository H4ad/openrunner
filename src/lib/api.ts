/**
 * Frontend API for Electron.
 * This module provides invoke/listen patterns for IPC communication.
 */

/**
 * Get the Electron API with type assertion
 */
function getElectronAPI(): ElectronAPI {
  if (!window.electron) {
    throw new Error('Electron API not available');
  }
  return window.electron;
}

/**
 * Convert snake_case to kebab-case for Electron IPC channels
 * The frontend stores use snake_case (legacy from Tauri), Electron uses kebab-case
 */
function toKebabCase(str: string): string {
  return str.replace(/_/g, '-');
}

/**
 * Invoke a command in the backend (main process).
 *
 * @param command - The command name (IPC channel), can be snake_case or kebab-case
 * @param args - Optional arguments object
 * @returns Promise with the result
 */
export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  // Convert snake_case to kebab-case for consistency
  const channel = toKebabCase(command);
  return getElectronAPI().invoke<T>(channel, args);
}

/**
 * Event listener callback type
 */
export type EventCallback<T> = (payload: T) => void;

/**
 * Unlisten function type
 */
export type UnlistenFn = () => void;

/**
 * Listen for events from the backend.
 *
 * @param event - The event name
 * @param callback - The callback function
 * @returns Promise that resolves to an unlisten function
 */
export async function listen<T>(
  event: string,
  callback: EventCallback<T>
): Promise<UnlistenFn> {
  const unlisten = getElectronAPI().on(event, (payload: unknown) => {
    callback(payload as T);
  });
  return unlisten;
}

/**
 * Listen for an event once.
 *
 * @param event - The event name
 * @param callback - The callback function
 */
export async function once<T>(event: string, callback: EventCallback<T>): Promise<void> {
  getElectronAPI().once(event, (payload: unknown) => {
    callback(payload as T);
  });
}

/**
 * Get the current platform.
 * Returns 'linux', 'darwin', 'win32', etc.
 */
export function platform(): string {
  return getElectronAPI().platform;
}
