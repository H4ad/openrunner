import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke, listen, type UnlistenFn } from "@/lib/api";
import type { ProcessInfo } from "../types";

export const useProcessesStore = defineStore("processes", () => {
  const statuses = ref<Map<string, ProcessInfo>>(new Map());
  let initialized = false;
  let unlistenStatus: UnlistenFn | null = null;
  let unlistenStats: UnlistenFn | null = null;

  function getStatus(projectId: string): ProcessInfo | undefined {
    return statuses.value.get(projectId);
  }

  async function startProcess(groupId: string, projectId: string, cols?: number, rows?: number) {
    await invoke("start_process", { groupId, projectId, cols, rows });
  }

  async function stopProcess(projectId: string) {
    await invoke("stop_process", { projectId });
  }

  async function restartProcess(groupId: string, projectId: string, cols?: number, rows?: number) {
    await invoke("restart_process", { groupId, projectId, cols, rows });
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

    unlistenStatus = await listen<ProcessInfo>("process-status-changed", (payload) => {
      statuses.value.set(payload.projectId, payload);
    });

    unlistenStats = await listen<ProcessInfo[]>("process-stats-updated", (payload) => {
      for (const info of payload) {
        const existing = statuses.value.get(info.projectId);
        if (existing) {
          existing.cpuUsage = info.cpuUsage;
          existing.memoryUsage = info.memoryUsage;
        }
      }
    });
  }

  function cleanup() {
    unlistenStatus?.();
    unlistenStats?.();
    initialized = false;
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
    cleanup,
  };
});
