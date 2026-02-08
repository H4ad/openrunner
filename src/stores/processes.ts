import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ProcessInfo } from "../types";

export const useProcessesStore = defineStore("processes", () => {
  const statuses = ref<Map<string, ProcessInfo>>(new Map());
  let initialized = false;

  function getStatus(projectId: string): ProcessInfo | undefined {
    return statuses.value.get(projectId);
  }

  async function startProcess(groupId: string, projectId: string) {
    await invoke("start_process", { groupId, projectId });
  }

  async function stopProcess(projectId: string) {
    await invoke("stop_process", { projectId });
  }

  async function restartProcess(groupId: string, projectId: string) {
    await invoke("restart_process", { groupId, projectId });
  }

  async function startAllInGroup(groupId: string, projectIds: string[]) {
    for (const projectId of projectIds) {
      const info = statuses.value.get(projectId);
      if (!info || info.status !== "running") {
        await invoke("start_process", { groupId, projectId });
      }
    }
  }

  async function stopAllInGroup(projectIds: string[]) {
    for (const projectId of projectIds) {
      const info = statuses.value.get(projectId);
      if (info && info.status === "running") {
        await invoke("stop_process", { projectId });
      }
    }
  }

  async function loadStatuses() {
    const all = await invoke<ProcessInfo[]>("get_all_statuses");
    for (const info of all) {
      statuses.value.set(info.projectId, info);
    }
  }

  async function init() {
    if (initialized) return;
    initialized = true;

    await loadStatuses();

    listen<ProcessInfo>("process-status-changed", (event) => {
      statuses.value.set(event.payload.projectId, event.payload);
    });

    listen<ProcessInfo[]>("process-stats-updated", (event) => {
      for (const info of event.payload) {
        const existing = statuses.value.get(info.projectId);
        if (existing) {
          existing.cpuUsage = info.cpuUsage;
          existing.memoryUsage = info.memoryUsage;
        }
      }
    });
  }

  return {
    statuses,
    getStatus,
    startProcess,
    stopProcess,
    restartProcess,
    startAllInGroup,
    stopAllInGroup,
    loadStatuses,
    init,
  };
});
