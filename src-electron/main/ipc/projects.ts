/**
 * Project-related IPC handlers
 * Ported from: src-tauri/src/commands/create_project.rs, update_project.rs, etc.
 */

import { ipcMain } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import * as yamlConfig from '../services/yaml-config';
import { getYamlWatcher } from '../services/yaml-watcher';
import { startFileWatcher, stopFileWatcher } from '../services/file-watcher';
import { restartProcess } from '../services/process-manager/restart';
import * as path from 'path';
import type { Project, Group, ProjectType } from '../../shared/types';

export function registerProjectHandlers(): void {
  // Create a new project
  ipcMain.handle(
    IPC_CHANNELS.CREATE_PROJECT,
    async (
      _event,
      args: {
        groupId: string;
        project: Omit<Project, 'id'>;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, project: projectData } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Create project with new ID
      const project: Project = {
        ...projectData,
        id: crypto.randomUUID(),
      };

      // Save to database
      state.database.createProject(groupId, project);

      // Update in-memory state
      group.projects.push(project);

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );

  // Update an existing project
  ipcMain.handle(
    IPC_CHANNELS.UPDATE_PROJECT,
    async (
      _event,
      args: {
        groupId: string;
        project: Project;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, project } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Find project index
      const projectIndex = group.projects.findIndex((p) => p.id === project.id);
      if (projectIndex === -1) {
        throw new Error(`Project not found: ${project.id}`);
      }

      // Get the old project to check if file watcher needs restart
      const oldProject = group.projects[projectIndex];
      const wasRunning = state.processes.has(project.id);

      // Update database
      state.database.updateProject(project);

      // Reload project from database to ensure all fields are correct
      // (handles undefined values that may have been stripped during IPC)
      const updatedProject = state.database.getProject(project.id);
      if (!updatedProject) {
        throw new Error(`Failed to reload project after update: ${project.id}`);
      }
      // Remove groupId from the project returned by getProject
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { groupId: _groupId, ...projectWithoutGroupId } = updatedProject;

      // Update in-memory state with the reloaded project
      group.projects[projectIndex] = projectWithoutGroupId;

      // Restart file watcher if patterns changed and process is running
      // Use projectWithoutGroupId (reloaded from DB) for accurate comparison
      if (wasRunning && projectWithoutGroupId.projectType === 'service' && projectWithoutGroupId.autoRestart) {
        const patternsChanged = JSON.stringify(oldProject.watchPatterns) !== JSON.stringify(projectWithoutGroupId.watchPatterns);
        const autoRestartChanged = oldProject.autoRestart !== projectWithoutGroupId.autoRestart;
        const projectTypeChanged = oldProject.projectType !== projectWithoutGroupId.projectType;

        if (patternsChanged || autoRestartChanged || projectTypeChanged) {
          // Stop existing watcher
          stopFileWatcher(projectWithoutGroupId.id);

          // Start new watcher with updated patterns
          const workingDir = projectWithoutGroupId.cwd ? path.resolve(group.directory, projectWithoutGroupId.cwd) : group.directory;
          startFileWatcher({
            projectId: projectWithoutGroupId.id,
            watchDir: workingDir,
            groupDir: group.directory,
            patterns: projectWithoutGroupId.watchPatterns,
            onChange: (changedFile) => {
              restartProcess(projectWithoutGroupId.id, changedFile);
            },
          });
        }
      } else if (wasRunning && (!projectWithoutGroupId.autoRestart || projectWithoutGroupId.projectType !== 'service')) {
        // Stop watcher if autoRestart was disabled or project type changed to task
        stopFileWatcher(projectWithoutGroupId.id);
      }

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );

  // Delete a project
  ipcMain.handle(
    IPC_CHANNELS.DELETE_PROJECT,
    async (
      _,
      args: {
        groupId: string;
        projectId: string;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, projectId } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Find project index
      const projectIndex = group.projects.findIndex((p) => p.id === projectId);
      if (projectIndex === -1) {
        throw new Error(`Project not found: ${projectId}`);
      }

      // Delete from database
      state.database.deleteProject(projectId);

      // Remove from in-memory state
      group.projects.splice(projectIndex, 1);

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );

  // Delete multiple projects
  ipcMain.handle(
    IPC_CHANNELS.DELETE_MULTIPLE_PROJECTS,
    async (
      _,
      args: {
        groupId: string;
        projectIds: string[];
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, projectIds } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Delete from database
      state.database.deleteProjects(projectIds);

      // Remove from in-memory state
      group.projects = group.projects.filter((p) => !projectIds.includes(p.id));

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );

  // Convert multiple projects to a different type
  ipcMain.handle(
    IPC_CHANNELS.CONVERT_MULTIPLE_PROJECTS,
    async (
      _,
      args: {
        groupId: string;
        projectIds: string[];
        newType: ProjectType;
      }
    ): Promise<Group> => {
      const state = getState();
      const { groupId, projectIds, newType } = args;

      // Find group in memory
      const group = state.findGroup(groupId);
      if (!group) {
        throw new Error(`Group not found: ${groupId}`);
      }

      // Build conversions array
      const conversions = projectIds.map((projectId) => ({
        projectId,
        projectType: newType,
      }));

      // Update database
      state.database.convertProjects(conversions);

      // Update in-memory state
      for (const project of group.projects) {
        if (projectIds.includes(project.id)) {
          project.projectType = newType;
        }
      }

      // Sync to YAML if enabled
      if (group.syncEnabled && group.syncFile) {
        yamlConfig.writeYaml(group, group.syncFile);
        getYamlWatcher().updateYamlTimestamp(group.syncFile);
      }

      return group;
    }
  );
}
