/**
 * IPC handler registration for all command handlers.
 * This is the equivalent of registering Tauri commands in lib.rs
 */

import { registerGroupHandlers } from './groups';
import { registerProjectHandlers } from './projects';
import { registerProcessHandlers } from './processes';
import { registerSessionHandlers } from './sessions';
import { registerSettingsHandlers } from './settings';
import { registerStorageHandlers } from './storage';
import { registerFileHandlers } from './files';
import { registerWindowHandlers } from './window';
import { registerUpdatesHandlers } from './updates';
import { registerCliHandlers } from './cli';

/**
 * Register all IPC handlers for the application
 */
export function registerAllHandlers(): void {
  registerGroupHandlers();
  registerProjectHandlers();
  registerProcessHandlers();
  registerSessionHandlers();
  registerSettingsHandlers();
  registerStorageHandlers();
  registerFileHandlers();
  registerWindowHandlers();
  registerUpdatesHandlers();
  registerCliHandlers();
}
