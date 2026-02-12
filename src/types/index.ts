export type ProjectType = "task" | "service";

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

export type ProcessStatus = "running" | "stopping" | "stopped" | "errored";

export interface ProcessInfo {
  projectId: string;
  status: ProcessStatus;
  pid: number | null;
  cpuUsage: number;
  memoryUsage: number;
}

export type LogStream = "stdout" | "stderr";

export interface LogMessage {
  projectId: string;
  stream: LogStream;
  data: string;
  timestamp: number;
}

export interface AppSettings {
  maxLogLines: number;
  editor: string | null;
  fullscreen: boolean | null;
}

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

export interface StorageStats {
  totalSize: number;
  logCount: number;
  metricCount: number;
  sessionCount: number;
}

export interface AppConfig {
  groups: Group[];
}
