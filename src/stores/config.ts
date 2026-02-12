import { defineStore } from "pinia";
import { ref, toRaw } from "vue";
import { invoke, listen, type UnlistenFn } from "@/lib/api";
import type { Group, Project, ProjectType } from "../types";

/**
 * Deep clone an object to remove Vue proxies and ensure it's serializable.
 * Uses structuredClone if available, falls back to JSON method.
 */
function deepClone<T>(obj: T): T {
  if (typeof structuredClone === 'function') {
    return structuredClone(obj);
  }
  // Fallback: JSON stringify/parse (faster but loses some types like Date, undefined)
  return JSON.parse(JSON.stringify(obj));
}

export const useConfigStore = defineStore("config", () => {
  const groups = ref<Group[]>([]);
  const loading = ref(false);
  let initialized = false;
  let unlistenConfigReload: UnlistenFn | null = null;
  let unlistenYamlChanged: UnlistenFn | null = null;

  async function loadGroups() {
    loading.value = true;
    try {
      groups.value = await invoke<Group[]>("get_groups");
    } finally {
      loading.value = false;
    }
  }

  async function createGroup(name: string, directory: string, syncEnabled?: boolean): Promise<Group> {
    const group = await invoke<Group>("create_group", { name, directory, syncEnabled });
    groups.value.push(group);
    return group;
  }

  async function renameGroup(groupId: string, name: string) {
    const updatedGroup = await invoke<Group>("rename_group", { groupId, name });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
  }

  async function updateGroupDirectory(groupId: string, directory: string) {
    const updatedGroup = await invoke<Group>("update_group_directory", { groupId, directory });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
  }

  async function updateGroupEnvVars(
    groupId: string,
    envVars: Record<string, string>,
  ) {
    const updatedGroup = await invoke<Group>("update_group_env_vars", { groupId, envVars });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
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
    envVars?: Record<string, string>,
    projectType?: ProjectType,
    interactive?: boolean,
    autoRestart?: boolean,
    watchPatterns?: string[],
  ): Promise<Project> {
    const project: Project = {
      id: crypto.randomUUID(),
      name,
      command,
      autoRestart: autoRestart ?? (projectType || "service") === "service",
      envVars: envVars ?? {},
      cwd: cwd || null,
      projectType: projectType || "service",
      interactive: interactive ?? false,
      watchPatterns,
    };

    const updatedGroup = await invoke<Group>("create_project", { groupId, project });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
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
      watchPatterns?: string[];
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
      if (updates.autoRestart === undefined) {
        project.autoRestart = updates.projectType === "service";
      }
    }
    if (updates.interactive !== undefined) project.interactive = updates.interactive;
    if ('watchPatterns' in updates) project.watchPatterns = updates.watchPatterns;

    // Save to database via command with YAML sync
    // Use deepClone to remove Vue's reactive proxies - required for Electron IPC serialization
    const projectToSend = deepClone(toRaw(project));
    const updatedGroup = await invoke<Group>("update_project", { groupId, project: projectToSend });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
  }

  async function deleteProject(groupId: string, projectId: string) {
    const updatedGroup = await invoke<Group>("delete_project", { groupId, projectId });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
  }

  async function deleteMultipleProjects(groupId: string, projectIds: string[]) {
    const updatedGroup = await invoke<Group>("delete_multiple_projects", { groupId, projectIds });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
  }

  async function convertMultipleProjects(
    groupId: string,
    projectIds: string[],
    newType: ProjectType,
  ) {
    const group = groups.value.find((g) => g.id === groupId);
    if (!group) return;

    // Update local state first
    for (const projectId of projectIds) {
      const project = group.projects.find((p) => p.id === projectId);
      if (project) {
        project.projectType = newType;
        project.autoRestart = newType === "service";
      }
    }

    // Save to database via command with YAML sync
    const updatedGroup = await invoke<Group>("convert_multiple_projects", { groupId, projectIds, newType });
    const idx = groups.value.findIndex((g) => g.id === groupId);
    if (idx !== -1) groups.value[idx] = updatedGroup;
  }

  // These functions use backend for file operations
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

    unlistenConfigReload = await listen<{ groups: Group[] }>("config-reloaded", (payload) => {
      groups.value = payload.groups;
    });

    unlistenYamlChanged = await listen<{ groupId: string; filePath: string }>("yaml-file-changed", (payload) => {
      console.log("YAML file changed, reloading group:", payload.groupId);
      reloadGroupFromYaml(payload.groupId);
    });
  }

  function cleanup() {
    unlistenConfigReload?.();
    unlistenYamlChanged?.();
    initialized = false;
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
    cleanup,
  };
});
