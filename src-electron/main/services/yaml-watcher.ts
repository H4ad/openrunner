/**
 * YAML file watcher service using chokidar.
 * This is the equivalent of src-tauri/src/file_watcher.rs
 */

import * as path from 'path';
import chokidar from 'chokidar';
import type { FSWatcher } from 'chokidar';
import type { BrowserWindow } from 'electron';
import type { Group } from '../../shared/types';
import { IPC_EVENTS } from '../../shared/events';

/** Debounce duration to ignore events after our own writes (in milliseconds) */
const DEBOUNCE_MS = 500;

/**
 * Manages file watchers for YAML configuration files
 */
export class YamlWatcher {
  private watchers: Map<string, FSWatcher> = new Map();
  private lastWriteTimes: Map<string, number> = new Map();
  private mainWindow: BrowserWindow | null = null;

  /**
   * Set the main window for emitting events
   */
  setMainWindow(window: BrowserWindow): void {
    this.mainWindow = window;
  }

  /**
   * Update the timestamp for a YAML file to prevent self-triggering
   */
  updateYamlTimestamp(yamlPath: string): void {
    this.lastWriteTimes.set(yamlPath, Date.now());
  }

  /**
   * Check if we should ignore an event (because we recently wrote to the file)
   */
  private shouldIgnoreEvent(yamlPath: string): boolean {
    const lastWrite = this.lastWriteTimes.get(yamlPath);
    if (lastWrite) {
      return Date.now() - lastWrite < DEBOUNCE_MS;
    }
    return false;
  }

  /**
   * Start watching a group's YAML file
   */
  watchGroup(group: Group): void {
    // Only watch if sync is enabled and sync_file exists
    if (!group.syncEnabled || !group.syncFile) {
      return;
    }

    // Don't watch if already watching
    if (this.watchers.has(group.id)) {
      return;
    }

    const syncFile = group.syncFile;
    const parentDir = path.dirname(syncFile);
    const fileName = path.basename(syncFile);
    const groupId = group.id;

    const watcher = chokidar.watch(parentDir, {
      persistent: true,
      ignoreInitial: true,
      depth: 0, // Only watch the immediate directory
    });

    watcher.on('change', (changedPath) => {
      // Only react to changes to our specific file
      if (path.basename(changedPath) !== fileName) {
        return;
      }

      // Check if we should ignore this event (self-triggered)
      if (this.shouldIgnoreEvent(syncFile)) {
        return;
      }

      // Emit event to frontend
      if (this.mainWindow && !this.mainWindow.isDestroyed()) {
        this.mainWindow.webContents.send(IPC_EVENTS.YAML_FILE_CHANGED, {
          groupId,
          filePath: syncFile,
        });
      }
    });

    watcher.on('error', (error) => {
      console.error(`File watcher error for group ${groupId}:`, error);
    });

    this.watchers.set(groupId, watcher);
  }

  /**
   * Stop watching a group's YAML file
   */
  async unwatchGroup(groupId: string): Promise<void> {
    const watcher = this.watchers.get(groupId);
    if (watcher) {
      await watcher.close();
      this.watchers.delete(groupId);
    }
  }

  /**
   * Update watchers for all groups in config
   */
  syncWatchers(groups: Group[]): void {
    // Get current watched group IDs
    const currentIds = new Set(this.watchers.keys());

    // Get group IDs that should be watched (must have sync_file AND sync_enabled)
    const groupsWithSync = groups.filter((g) => g.syncFile && g.syncEnabled);
    const groupIdsWithSync = new Set(groupsWithSync.map((g) => g.id));

    // Unwatch groups that no longer have sync enabled
    for (const groupId of currentIds) {
      if (!groupIdsWithSync.has(groupId)) {
        this.unwatchGroup(groupId);
      }
    }

    // Watch new groups with sync enabled
    for (const group of groupsWithSync) {
      if (!currentIds.has(group.id)) {
        this.watchGroup(group);
      }
    }
  }

  /**
   * Close all watchers
   */
  async closeAll(): Promise<void> {
    const closePromises = Array.from(this.watchers.values()).map((w) =>
      w.close()
    );
    await Promise.all(closePromises);
    this.watchers.clear();
    this.lastWriteTimes.clear();
  }
}

// Singleton instance
let instance: YamlWatcher | null = null;

export function getYamlWatcher(): YamlWatcher {
  if (!instance) {
    instance = new YamlWatcher();
  }
  return instance;
}
