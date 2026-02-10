import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Group, Project, AppConfig, ProjectType } from "../types";

export const useConfigStore = defineStore("config", () => {
  const groups = ref<Group[]>([]);
  const loading = ref(false);
  let initialized = false;

  async function loadGroups() {
    loading.value = true;
    try {
      groups.value = await invoke<Group[]>("get_groups");
    } finally {
      loading.value = false;
    }
  }

  async function createGroup(name: string, directory: string): Promise<Group> {
    const group = await invoke<Group>("create_group", { name, directory });
    groups.value.push(group);
    return group;
  }

  async function renameGroup(groupId: string, name: string) {
    const updated = await invoke<Group>("rename_group", { groupId, name });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updated;
  }

  async function updateGroupDirectory(groupId: string, directory: string) {
    const updated = await invoke<Group>("update_group_directory", {
      groupId,
      directory,
    });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updated;
  }

  async function updateGroupEnvVars(
    groupId: string,
    envVars: Record<string, string>,
  ) {
    const updated = await invoke<Group>("update_group_env_vars", {
      groupId,
      envVars,
    });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updated;
  }

  async function deleteGroup(groupId: string) {
    await invoke("delete_group", { groupId });
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
    const project = await invoke<Project>("create_project", {
      groupId,
      name,
      command,
      cwd: cwd || null,
      projectType: projectType || "service",
      interactive: interactive ?? false,
    });
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
    const updated = await invoke<Project>("update_project", {
      groupId,
      projectId,
      ...updates,
    });
    const group = groups.value.find((g) => g.id === groupId);
    if (group) {
      const idx = group.projects.findIndex((p) => p.id === projectId);
      if (idx !== -1) group.projects[idx] = updated;
    }
  }

  async function deleteProject(groupId: string, projectId: string) {
    await invoke("delete_project", { groupId, projectId });
    const group = groups.value.find((g) => g.id === groupId);
    if (group) {
      group.projects = group.projects.filter((p) => p.id !== projectId);
    }
  }

  async function deleteMultipleProjects(groupId: string, projectIds: string[]) {
    await invoke("delete_multiple_projects", { groupId, projectIds });
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
    const updatedGroup = await invoke<Group>("convert_multiple_projects", {
      groupId,
      projectIds,
      newType,
    });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) {
      groups.value[idx] = updatedGroup;
    }
  }

  async function exportGroup(groupId: string, filePath: string) {
    await invoke("export_group", { groupId, filePath });
  }

  async function importGroup(filePath: string): Promise<Group> {
    const group = await invoke<Group>("import_group", { filePath });
    groups.value.push(group);
    return group;
  }

  async function toggleGroupSync(groupId: string): Promise<Group> {
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

    listen<AppConfig>("config-reloaded", (event) => {
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
