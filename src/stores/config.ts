import { defineStore } from "pinia";
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { Group, Project, ProjectType } from "../types";
import * as db from "../services/database";

export const useConfigStore = defineStore("config", () => {
  const groups = ref<Group[]>([]);
  const loading = ref(false);
  let initialized = false;

  async function loadGroups() {
    loading.value = true;
    try {
      groups.value = await db.getGroups();
    } finally {
      loading.value = false;
    }
  }

  async function createGroup(name: string, directory: string, syncEnabled?: boolean): Promise<Group> {
    const group: Group = {
      id: crypto.randomUUID(),
      name,
      directory,
      projects: [],
      envVars: {},
      syncEnabled: syncEnabled || false,
    };
    
    await db.createGroup(group);
    groups.value.push(group);
    return group;
  }

  async function renameGroup(groupId: string, name: string) {
    await db.updateGroup(groupId, { name });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx].name = name;
  }

  async function updateGroupDirectory(groupId: string, directory: string) {
    await db.updateGroup(groupId, { directory });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx].directory = directory;
  }

  async function updateGroupEnvVars(
    groupId: string,
    envVars: Record<string, string>,
  ) {
    await db.updateGroup(groupId, { envVars });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx].envVars = envVars;
  }

  async function deleteGroup(groupId: string) {
    await db.deleteGroup(groupId);
    groups.value = groups.value.filter((g) => g.id !== groupId);
  }

  async function createProject(
    groupId: string,
    name: string,
    command: string,
    cwd?: string,
    projectType?: ProjectType,
    interactive?: boolean,
  ): Promise<Project> {
    const project: Project = {
      id: crypto.randomUUID(),
      name,
      command,
      autoRestart: (projectType || "service") === "service",
      envVars: {},
      cwd: cwd || null,
      projectType: projectType || "service",
      interactive: interactive ?? false,
    };

    await db.createProject(groupId, project);
    const group = groups.value.find((g) => g.id === groupId);
    if (group) group.projects.push(project);
    return project;
  }

  async function updateProject(
    groupId: string,
    projectId: string,
    updates: {
      name?: string;
      command?: string;
      autoRestart?: boolean;
      envVars?: Record<string, string>;
      cwd?: string | null;
      projectType?: ProjectType;
      interactive?: boolean;
    },
  ) {
    const group = groups.value.find((g) => g.id === groupId);
    if (!group) return;

    const project = group.projects.find((p) => p.id === projectId);
    if (!project) return;

    // Update local project object
    if (updates.name !== undefined) project.name = updates.name;
    if (updates.command !== undefined) project.command = updates.command;
    if (updates.autoRestart !== undefined) project.autoRestart = updates.autoRestart;
    if (updates.envVars !== undefined) project.envVars = updates.envVars;
    if (updates.cwd !== undefined) project.cwd = updates.cwd || null;
    if (updates.projectType !== undefined) {
      project.projectType = updates.projectType;
      project.autoRestart = updates.projectType === "service";
    }
    if (updates.interactive !== undefined) project.interactive = updates.interactive;

    // Save to database
    await db.updateProject(project);
  }

  async function deleteProject(groupId: string, projectId: string) {
    await db.deleteProject(projectId);
    const group = groups.value.find((g) => g.id === groupId);
    if (group) {
      group.projects = group.projects.filter((p) => p.id !== projectId);
    }
  }

  async function deleteMultipleProjects(groupId: string, projectIds: string[]) {
    for (const projectId of projectIds) {
      await db.deleteProject(projectId);
    }
    const group = groups.value.find((g) => g.id === groupId);
    if (group) {
      group.projects = group.projects.filter((p) => !projectIds.includes(p.id));
    }
  }

  async function convertMultipleProjects(
    groupId: string,
    projectIds: string[],
    newType: ProjectType,
  ) {
    const group = groups.value.find((g) => g.id === groupId);
    if (!group) return;

    for (const projectId of projectIds) {
      const project = group.projects.find((p) => p.id === projectId);
      if (project) {
        project.projectType = newType;
        project.autoRestart = newType === "service";
        await db.updateProject(project);
      }
    }
  }

  // These functions still use Rust for file operations
  async function exportGroup(groupId: string, filePath: string) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("export_group", { groupId, filePath });
  }

  async function importGroup(filePath: string): Promise<Group> {
    const { invoke } = await import("@tauri-apps/api/core");
    const group = await invoke<Group>("import_group", { filePath });
    groups.value.push(group);
    return group;
  }

  async function toggleGroupSync(groupId: string): Promise<Group> {
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      const updatedGroup: Group = await invoke('toggle_group_sync', { groupId });
      const index = groups.value.findIndex((g: Group) => g.id === groupId);
      if (index !== -1) {
        groups.value[index] = updatedGroup;
      }
      return updatedGroup;
    } catch (error) {
      console.error('Failed to toggle group sync:', error);
      throw error;
    }
  }

  async function reloadGroupFromYaml(groupId: string) {
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      const updatedGroup = await invoke<Group>("reload_group_from_yaml", { groupId });
      const index = groups.value.findIndex((g: Group) => g.id === groupId);
      if (index !== -1) {
        groups.value[index] = updatedGroup;
      }
      return updatedGroup;
    } catch (error) {
      console.error("Failed to reload group from YAML:", error);
      throw error;
    }
  }

  async function init() {
    if (initialized) return;
    initialized = true;

    await loadGroups();

    listen<{ groups: Group[] }>("config-reloaded", (event) => {
      groups.value = event.payload.groups;
    });

    listen<{ groupId: string; filePath: string }>("yaml-file-changed", (event) => {
      console.log("YAML file changed, reloading group:", event.payload.groupId);
      reloadGroupFromYaml(event.payload.groupId);
    });
  }

  return {
    groups,
    loading,
    loadGroups,
    createGroup,
    renameGroup,
    updateGroupDirectory,
    updateGroupEnvVars,
    deleteGroup,
    createProject,
    updateProject,
    deleteProject,
    deleteMultipleProjects,
    convertMultipleProjects,
    exportGroup,
    importGroup,
    toggleGroupSync,
    reloadGroupFromYaml,
    init,
  };
});
