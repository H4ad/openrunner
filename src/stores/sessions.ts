import { defineStore } from "pinia";
import { ref } from "vue";
import type { Session, SessionWithStats, MetricPoint } from "../types";
import * as db from "../services/database";

export const useSessionsStore = defineStore("sessions", () => {
  const sessions = ref<Session[]>([]);
  const sessionsWithStats = ref<SessionWithStats[]>([]);
  const loading = ref(false);

  async function loadSessions(groupId: string, projectId: string) {
    loading.value = true;
    try {
      sessions.value = await db.getProjectSessions(groupId, projectId);
    } finally {
      loading.value = false;
    }
  }

  async function loadSessionsWithStats(groupId: string, projectId: string) {
    loading.value = true;
    try {
      sessionsWithStats.value = await db.getProjectSessionsWithStats(groupId, projectId);
    } finally {
      loading.value = false;
    }
  }

  async function getSessionLogs(groupId: string, sessionId: string): Promise<string> {
    return await db.getSessionLogs(groupId, sessionId);
  }

  async function getSessionMetrics(groupId: string, sessionId: string): Promise<MetricPoint[]> {
    return await db.getSessionMetrics(groupId, sessionId);
  }

  async function deleteSession(groupId: string, sessionId: string) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("delete_session", { groupId, sessionId });
    sessions.value = sessions.value.filter((s) => s.id !== sessionId);
  }

  async function getLastSession(
    groupId: string,
    projectId: string,
  ): Promise<Session | null> {
    return await db.getLastCompletedSession(groupId, projectId);
  }

  async function getRecentLogs(
    groupId: string,
    projectId: string,
    limit: number,
  ): Promise<string> {
    return await db.getRecentLogs(groupId, projectId, limit);
  }

  async function getLastMetric(
    groupId: string,
    sessionId: string,
  ): Promise<MetricPoint | null> {
    return await db.getLastMetric(groupId, sessionId);
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
