/**
 * Storage-related IPC handlers
 * Ported from: src-tauri/src/commands/get_storage_stats.rs, cleanup_storage.rs, etc.
 */

import { ipcMain } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import type { StorageStats } from '../../shared/types';

export function registerStorageHandlers(): void {
  // Get storage stats
  ipcMain.handle(
    IPC_CHANNELS.GET_STORAGE_STATS,
    async (): Promise<StorageStats> => {
      const state = getState();
      return state.database.getStorageStats();
    }
  );

  // Cleanup storage older than N days
  ipcMain.handle(
    IPC_CHANNELS.CLEANUP_STORAGE,
    async (
      _,
      args: {
        days: number;
      }
    ): Promise<void> => {
      const state = getState();
      state.database.cleanupOldSessions(args.days);
    }
  );

  // Cleanup all storage
  ipcMain.handle(IPC_CHANNELS.CLEANUP_ALL_STORAGE, async (): Promise<void> => {
    const state = getState();
    state.database.cleanupAllSessions();
  });

  // Get database path
  ipcMain.handle(IPC_CHANNELS.GET_DATABASE_PATH, async (): Promise<string> => {
    const state = getState();
    return state.databasePath;
  });
}
