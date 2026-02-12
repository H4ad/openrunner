import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Session, SessionWithStats, MetricPoint } from "../types";

export const useSessionsStore = defineStore("sessions", () => {
  const sessions = ref<Session[]>([]);
  const sessionsWithStats = ref<SessionWithStats[]>([]);
  const loading = ref(false);

  async function loadSessions(_groupId: string, projectId: string) {
    loading.value = true;
    try {
      sessions.value = await invoke<Session[]>("get_project_sessions", { projectId });
    } finally {
      loading.value = false;
    }
  }

  async function loadSessionsWithStats(_groupId: string, projectId: string) {
    loading.value = true;
    try {
      sessionsWithStats.value = await invoke<SessionWithStats[]>(
        "get_project_sessions_with_stats",
        { projectId },
      );
    } finally {
      loading.value = false;
    }
  }

  async function getSessionLogs(_groupId: string, sessionId: string): Promise<string> {
    return await invoke<string>("get_session_logs", { sessionId });
  }

  async function getSessionMetrics(_groupId: string, sessionId: string): Promise<MetricPoint[]> {
    return await invoke<MetricPoint[]>("get_session_metrics", { sessionId });
  }

  async function deleteSession(groupId: string, sessionId: string) {
    await invoke("delete_session", { groupId, sessionId });
    sessions.value = sessions.value.filter((s) => s.id !== sessionId);
  }

  async function getLastSession(
    _groupId: string,
    projectId: string,
  ): Promise<Session | null> {
    return await invoke<Session | null>("get_last_completed_session", { projectId });
  }

  async function getRecentLogs(
    _groupId: string,
    projectId: string,
    limit: number,
  ): Promise<string> {
    return await invoke<string>("get_recent_logs", { projectId, limit });
  }

  async function getLastMetric(
    _groupId: string,
    sessionId: string,
  ): Promise<MetricPoint | null> {
    return await invoke<MetricPoint | null>("get_last_metric", { sessionId });
  }

  return {
    sessions,
    sessionsWithStats,
    loading,
    loadSessions,
    loadSessionsWithStats,
    getSessionLogs,
    getSessionMetrics,
    deleteSession,
    getLastSession,
    getRecentLogs,
    getLastMetric,
  };
});
