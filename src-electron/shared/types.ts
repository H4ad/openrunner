/**
 * Shared types between main and renderer processes.
 * These mirror the types in src/types/index.ts but include main-process specific additions.
 */

import type { ChildProcess } from 'child_process';
import type { IPty } from 'node-pty';

// ============================================================================
// Project Types (mirrored from src/types/index.ts)
// ============================================================================

export type ProjectType = 'task' | 'service';

export interface Project {
  id: string;
  name: string;
  command: string;
  autoRestart: boolean;
  envVars: Record<string, string>;
  cwd: string | null;
  projectType: ProjectType;
  interactive: boolean;
  watchPatterns?: string[];
  autoStartOnLaunch: boolean;
}

export interface Group {
  id: string;
  name: string;
  directory: string;
  projects: Project[];
  envVars: Record<string, string>;
  syncFile?: string;
  syncEnabled: boolean;
}

export interface AppConfig {
  groups: Group[];
}

// ============================================================================
// Process Types
// ============================================================================

export type ProcessStatus = 'running' | 'stopping' | 'stopped' | 'errored';

export interface ProcessInfo {
  projectId: string;
  status: ProcessStatus;
  pid: number | null;
  cpuUsage: number;
  memoryUsage: number;
}

export type LogStream = 'stdout' | 'stderr';

export interface LogMessage {
  projectId: string;
  stream: LogStream;
  data: string;
  timestamp: number;
}

// ============================================================================
// Session Types
// ============================================================================

export interface Session {
  id: string;
  projectId: string;
  startedAt: number;
  endedAt: number | null;
  exitStatus: string | null;
}

export interface SessionWithStats {
  id: string;
  projectId: string;
  startedAt: number;
  endedAt: number | null;
  exitStatus: string | null;
  logCount: number;
  logSize: number;
  metricCount: number;
}

export interface MetricPoint {
  timestamp: number;
  cpuUsage: number;
  memoryUsage: number;
}

// ============================================================================
// Settings Types
// ============================================================================

export interface AppSettings {
  maxLogLines: number;
  editor: string | null;
  fullscreen: boolean | null;
  shell: string | null;
  minimizeToTray: boolean;
  autoLaunch: boolean;
}

// ============================================================================
// Storage Types
// ============================================================================

export interface StorageStats {
  totalSize: number;
  logCount: number;
  metricCount: number;
  sessionCount: number;
}

// ============================================================================
// Main Process Specific Types
// ============================================================================

/**
 * Represents a managed process in the main process.
 * This is NOT sent to the renderer - use ProcessInfo for that.
 */
export interface ManagedProcess {
  /** The child process handle (for regular processes) */
  child: ChildProcess | null;
  /** The PTY process handle (for interactive processes) */
  ptyProcess: IPty | null;
  /** Whether the process was manually stopped by the user */
  manuallyStopped: boolean;
  /** The current session ID for this process run */
  sessionId: string | null;
  /** The group this process belongs to */
  groupId: string;
  /** Whether this is an interactive (PTY) process */
  isInteractive: boolean;
  /** The real process ID (may differ from child.pid for PTY) */
  realPid: number | null;
}

/**
 * PTY dimensions for terminal resizing
 */
export interface PtyDimensions {
  rows: number;
  cols: number;
}

// ============================================================================
// IPC Event Payload Types
// ============================================================================

export interface ConfigReloadedPayload {
  groups: Group[];
}

export interface YamlFileChangedPayload {
  groupId: string;
  filePath: string;
}

// ============================================================================
// YAML Config Types (for openrunner.yaml)
// ============================================================================

export interface YamlProject {
  name: string;
  command: string;
  autoRestart?: boolean;
  cwd?: string;
  type?: 'task' | 'service';
  interactive?: boolean;
  env?: Record<string, string>;
  watchPatterns?: string[];
  autoStartOnLaunch?: boolean;
}

export interface YamlConfig {
  name?: string;
  env?: Record<string, string>;
  projects: YamlProject[];
}

// ============================================================================
// Database Row Types (for better-sqlite3)
// ============================================================================

export interface GroupRow {
  id: string;
  name: string;
  directory: string;
  sync_file: string | null;
  sync_enabled: number;
}

export interface ProjectRow {
  id: string;
  group_id: string;
  name: string;
  command: string;
  auto_restart: number;
  cwd: string | null;
  project_type: string;
  interactive: number;
  watch_patterns: string | null;
  auto_start_on_launch: number;
}

export interface EnvVarRow {
  key: string;
  value: string;
}

export interface SessionRow {
  id: string;
  project_id: string;
  started_at: number;
  ended_at: number | null;
  exit_status: string | null;
}

export interface LogRow {
  id: number;
  session_id: string;
  stream: string;
  data: string;
  timestamp: number;
}

export interface MetricRow {
  id: number;
  session_id: string;
  cpu_usage: number;
  memory_usage: number;
  timestamp: number;
}

export interface SettingRow {
  key: string;
  value: string;
}
