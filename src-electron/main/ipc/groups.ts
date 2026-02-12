/**
 * Group-related IPC handlers
 * Ported from: src-tauri/src/commands/get_groups.rs, create_group.rs, etc.
 */

import * as path from 'path';
import { ipcMain } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import * as yamlConfig from '../services/yaml-config';
import { getYamlWatcher } from '../services/yaml-watcher';
import type { Group } from '../../shared/types';

export function registerGroupHandlers(): void {
  // Get all groups
  ipcMain.handle(IPC_CHANNELS.GET_GROUPS, async (): Promise<Group[]> => {
    const state = getState();
    return state.config.groups;
  });

  // Create a new group
  ipcMain.handle(
    IPC_CHANNELS.CREATE_GROUP,
    async (
      _,
      args: {
        name: string;
        directory: string;
        syncEnabled: boolean;
      }
    ): Promise<Group> => {
      const state = getState();
      const { name, directory, syncEnabled } = args;

      // Check if openrunner.yaml or openrunner.yml exists
      const existingYamlPath = yamlConfig.findYamlFile(directory);

      let group: Group;

      if (existingYamlPath) {
        // Parse YAML and create group from it
        const config = yamlConfig.parseYaml(existingYamlPath);
        group = yamlConfig.yamlToGroup(config, directory, existingYamlPath);

        // Set sync_enabled - default to true if YAML exists
        group.syncEnabled = syncEnabled ?? true;
      } else if (syncEnabled) {
        // Create group with YAML sync enabled - create the YAML file
        group = {
          id: crypto.randomUUID(),
          name,
          directory,
          projects: [],
          envVars: {},
          syncFile: undefined,
          syncEnabled: true,
        };

        // Create YAML file
        const yamlPath = path.join(directory, 'openrunner.yaml');
        yamlConfig.writeYaml(group, yamlPath);
        group.syncFile = yamlPath;
      } else {
        // Create empty group without sync
        group = {
          id: crypto.randomUUID(),
          name,
          directory,
          projects: [],
          envVars: {},
          syncFile: undefined,
          syncEnabled: false,
        };
      }

      // Save to database
      state.database.createGroup(group);

      // Update in-memory state
      state.config.groups.push(group);

      // Start watching the YAML file if sync is enabled
      if (group.syncEnabled && group.syncFile) {
        getYamlWatcher().watchGroup(group);
      }

      return group;
    }
  );

  // Rename a group
  ipcMain.handle(
    IPC_CHANNELS.RENAME_GROUP,
    async (
      _,
      args: {
        groupId: string;
        name: string;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, name } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Update database
      state.database.renameGroup(groupId, name);

      // Update in-memory state
      group.name = name;

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );

  // Update group directory
  ipcMain.handle(
    IPC_CHANNELS.UPDATE_GROUP_DIRECTORY,
    async (
      _,
      args: {
        groupId: string;
        directory: string;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, directory } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Update database
      state.database.updateGroupDirectory(groupId, directory);

      // Update in-memory state
      group.directory = directory;

      // If sync is enabled, update the sync file path
      if (group.syncEnabled) {
        const newSyncFile = yamlConfig.getYamlPath(directory);
        group.syncFile = newSyncFile;
        state.database.updateGroupSync(groupId, newSyncFile, true);

        // Write YAML to new location
        yamlConfig.writeYaml(group, newSyncFile);
        getYamlWatcher().updateYamlTimestamp(newSyncFile);

        // Re-watch with new path
        await getYamlWatcher().unwatchGroup(groupId);
        getYamlWatcher().watchGroup(group);
      }

      return group;
    }
  );

  // Update group environment variables
  ipcMain.handle(
    IPC_CHANNELS.UPDATE_GROUP_ENV_VARS,
    async (
      _,
      args: {
        groupId: string;
        envVars: Record<string, string>;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, envVars } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Update database
      state.database.updateGroupEnvVars(groupId, envVars);

      // Update in-memory state
      group.envVars = envVars;

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );

  // Delete a group
  ipcMain.handle(
    IPC_CHANNELS.DELETE_GROUP,
    async (
      _,
      args: {
        groupId: string;
      }
    ): Promise<void> => {
      const state = getState();
      const { groupId } = args;

      // Find group in memory
      const groupIndex = state.config.groups.findIndex((g) => g.id === groupId);
      if (groupIndex === -1) {
        throw new Error(`Group not found: ${groupId}`);
      }

      const group = state.config.groups[groupIndex];

      // Stop all processes in this group
      // TODO: Implement process stopping

      // Stop watching YAML file
      await getYamlWatcher().unwatchGroup(groupId);

      // Delete from database
      state.database.deleteGroup(groupId);

      // Remove from in-memory state
      state.config.groups.splice(groupIndex, 1);
    }
  );

  // Toggle group sync
  ipcMain.handle(
    IPC_CHANNELS.TOGGLE_GROUP_SYNC,
    async (
      _,
      args: {
        groupId: string;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      const newSyncEnabled = !group.syncEnabled;

      if (newSyncEnabled) {
        // Enabling sync
        const yamlPath = yamlConfig.getYamlPath(group.directory);
        group.syncFile = yamlPath;
        group.syncEnabled = true;

        // Create/update YAML file
        yamlConfig.writeYaml(group, yamlPath);

        // Start watching
        getYamlWatcher().watchGroup(group);

        // Update database
        state.database.updateGroupSync(groupId, yamlPath, true);
      } else {
        // Disabling sync
        // Stop watching
        await getYamlWatcher().unwatchGroup(groupId);

        // Update state (keep sync_file for reference but disable sync)
        group.syncEnabled = false;

        // Update database
        state.database.updateGroupSync(groupId, group.syncFile ?? null, false);
      }

      return group;
    }
  );

  // Reload group from YAML
  ipcMain.handle(
    IPC_CHANNELS.RELOAD_GROUP_FROM_YAML,
    async (
      _,
      args: {
        groupId: string;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      if (!group.syncFile) {
        throw new Error('Group has no sync file');
      }

      // Parse YAML
      const config = yamlConfig.parseYaml(group.syncFile);

      // Update group from YAML (preserving IDs where possible)
      const updatedGroup = yamlConfig.updateGroupFromYaml(
        group,
        config,
        group.directory
      );

      // Replace group in database
      state.database.replaceGroup(updatedGroup);

      // Update in-memory state
      const groupIndex = state.config.groups.findIndex((g) => g.id === groupId);
      if (groupIndex !== -1) {
        state.config.groups[groupIndex] = updatedGroup;
      }

      return updatedGroup;
    }
  );

  // Export group to YAML
  ipcMain.handle(
    IPC_CHANNELS.EXPORT_GROUP,
    async (
      _,
      args: {
        groupId: string;
        filePath: string;
      }
    ): Promise<void> => {
      const state = getState();
      const { groupId, filePath } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Write YAML to specified path
      yamlConfig.writeYaml(group, filePath);
    }
  );

  // Import group from YAML
  ipcMain.handle(
    IPC_CHANNELS.IMPORT_GROUP,
    async (
      _,
      args: {
        filePath: string;
      }
    ): Promise<Group> => {
      const state = getState();
      const { filePath } = args;

      // Parse YAML
      const config = yamlConfig.parseYaml(filePath);

      // Create group from YAML
      const directory = path.dirname(filePath);
      const group = yamlConfig.yamlToGroup(config, directory, filePath);

      // Save to database
      state.database.createGroup(group);

      // Update in-memory state
      state.config.groups.push(group);

      // Start watching the YAML file
      getYamlWatcher().watchGroup(group);

      return group;
    }
  );
}
