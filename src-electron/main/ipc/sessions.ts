/**
 * Session-related IPC handlers
 * Ported from: src-tauri/src/commands/get_project_sessions.rs, etc.
 */

import { ipcMain } from 'electron';
import { IPC_CHANNELS } from '../../shared/events';
import { getState } from '../services/state';
import type {
  Session,
  SessionWithStats,
  LogMessage,
  MetricPoint,
} from '../../shared/types';

export function registerSessionHandlers(): void {
  // Get project sessions
  ipcMain.handle(
    IPC_CHANNELS.GET_PROJECT_SESSIONS,
    async (
      _,
      args: {
        projectId: string;
      }
    ): Promise<Session[]> => {
      const state = getState();
      return state.database.getProjectSessions(args.projectId);
    }
  );

  // Get project sessions with stats
  ipcMain.handle(
    IPC_CHANNELS.GET_PROJECT_SESSIONS_WITH_STATS,
    async (
      _,
      args: {
        projectId: string;
      }
    ): Promise<SessionWithStats[]> => {
      const state = getState();
      return state.database.getProjectSessionsWithStats(args.projectId);
    }
  );

  // Get a specific session
  ipcMain.handle(
    IPC_CHANNELS.GET_SESSION,
    async (
      _,
      args: {
        groupId: string;
        sessionId: string;
      }
    ): Promise<Session | null> => {
      const state = getState();
      return state.database.getSession(args.sessionId);
    }
  );

  // Get session logs
  ipcMain.handle(
    IPC_CHANNELS.GET_SESSION_LOGS,
    async (
      _,
      args: {
        sessionId: string;
      }
    ): Promise<string> => {
      const state = getState();
      // Return logs as concatenated string (matches Rust behavior)
      return state.database.getSessionLogsAsString(args.sessionId);
    }
  );

  // Get session metrics
  ipcMain.handle(
    IPC_CHANNELS.GET_SESSION_METRICS,
    async (
      _,
      args: {
        sessionId: string;
      }
    ): Promise<MetricPoint[]> => {
      const state = getState();
      return state.database.getSessionMetrics(args.sessionId);
    }
  );

  // Get last completed session
  ipcMain.handle(
    IPC_CHANNELS.GET_LAST_COMPLETED_SESSION,
    async (
      _,
      args: {
        projectId: string;
      }
    ): Promise<Session | null> => {
      const state = getState();
      return state.database.getLastCompletedSession(args.projectId);
    }
  );

  // Get recent logs
  ipcMain.handle(
    IPC_CHANNELS.GET_RECENT_LOGS,
    async (
      _,
      args: {
        projectId: string;
        limit: number;
      }
    ): Promise<LogMessage[]> => {
      const state = getState();
      return state.database.getRecentLogs(args.projectId, args.limit);
    }
  );

  // Get last metric
  ipcMain.handle(
    IPC_CHANNELS.GET_LAST_METRIC,
    async (
      _,
      args: {
        sessionId: string;
      }
    ): Promise<MetricPoint | null> => {
      const state = getState();
      return state.database.getLastMetric(args.sessionId);
    }
  );

  // Delete a session
  ipcMain.handle(
    IPC_CHANNELS.DELETE_SESSION,
    async (
      _,
      args: {
        groupId: string;
        sessionId: string;
      }
    ): Promise<void> => {
      const state = getState();
      state.database.deleteSession(args.sessionId);
    }
  );
}
