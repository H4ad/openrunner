import { defineStore } from "pinia";
import { ref } from "vue";
import { listen, type UnlistenFn } from "@/lib/api";
import { useSettingsStore } from "./settings";
import type { LogMessage } from "../types";

export const useLogsStore = defineStore("logs", () => {
  const logs = ref<Map<string, LogMessage[]>>(new Map());
  let initialized = false;
  let unlistenLog: UnlistenFn | null = null;

  function getProjectLogs(projectId: string): LogMessage[] {
    return logs.value.get(projectId) ?? [];
  }

  function clearProjectLogs(projectId: string) {
    logs.value.set(projectId, []);
  }

  async function init() {
    if (initialized) return;
    initialized = true;

    const settings = useSettingsStore();

    unlistenLog = await listen<LogMessage>("process-log", (msg) => {
      let buffer = logs.value.get(msg.projectId);
      if (!buffer) {
        buffer = [];
        logs.value.set(msg.projectId, buffer);
      }
      buffer.push(msg);
      const maxLines = settings.maxLogLines;
      if (buffer.length > maxLines) {
        buffer.splice(0, buffer.length - maxLines);
      }
    });
  }

  function cleanup() {
    unlistenLog?.();
    initialized = false;
  }

  return {
    logs,
    getProjectLogs,
    clearProjectLogs,
    init,
    cleanup,
  };
});
