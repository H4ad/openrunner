/**
 * Database wrapper for SQLite operations.
 * This is the equivalent of src-tauri/src/database/*.rs
 */

import BetterSqlite3 from 'better-sqlite3';
import type { Database as BetterSqlite3Database } from 'better-sqlite3';
import { initSchema } from './database/schema';
import { runMigrations } from './database/migrations';
import type {
  Group,
  Project,
  Session,
  SessionWithStats,
  LogMessage,
  MetricPoint,
  AppSettings,
  StorageStats,
  GroupRow,
  ProjectRow,
  EnvVarRow,
  SessionRow,
  LogRow,
  MetricRow,
} from '../../shared/types';

export class Database {
  private db: BetterSqlite3Database;

  constructor(dbPath: string) {
    this.db = new BetterSqlite3(dbPath);

    // Enable WAL mode for better performance
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('foreign_keys = ON');
    this.db.pragma('synchronous = NORMAL');
    this.db.pragma('temp_store = MEMORY');
    this.db.pragma('cache_size = -64000'); // 64MB
    this.db.pragma('busy_timeout = 5000');

    // Initialize schema
    initSchema(this.db);

    // Run migrations for schema updates
    runMigrations(this.db);
  }

  /**
   * Close the database connection
   */
  close(): void {
    this.db.close();
  }

  // ============================================================================
  // Group Operations
  // ============================================================================

  /**
   * Get all groups with their projects and env vars
   */
  getAllGroups(): Group[] {
    const groupRows = this.db
      .prepare('SELECT * FROM groups ORDER BY name')
      .all() as GroupRow[];

    return groupRows.map((row) => this.hydrateGroup(row));
  }

  /**
   * Get a single group by ID
   */
  getGroup(groupId: string): Group | null {
    const row = this.db
      .prepare('SELECT * FROM groups WHERE id = ?')
      .get(groupId) as GroupRow | undefined;

    return row ? this.hydrateGroup(row) : null;
  }

  /**
   * Create a new group with projects and env vars
   */
  createGroup(group: Group): void {
    const transaction = this.db.transaction(() => {
      // Insert group
      this.db
        .prepare(
          'INSERT INTO groups (id, name, directory, sync_file, sync_enabled) VALUES (?, ?, ?, ?, ?)'
        )
        .run(
          group.id,
          group.name,
          group.directory,
          group.syncFile ?? null,
          group.syncEnabled ? 1 : 0
        );

      // Insert group env vars
      for (const [key, value] of Object.entries(group.envVars)) {
        this.db
          .prepare(
            'INSERT INTO group_env_vars (group_id, key, value) VALUES (?, ?, ?)'
          )
          .run(group.id, key, value);
      }

      // Insert projects
      for (const project of group.projects) {
        this.insertProject(group.id, project);
      }
    });

    transaction();
  }

  /**
   * Rename a group
   */
  renameGroup(groupId: string, name: string): void {
    this.db.prepare('UPDATE groups SET name = ? WHERE id = ?').run(name, groupId);
  }

  /**
   * Update group directory
   */
  updateGroupDirectory(groupId: string, directory: string): void {
    this.db
      .prepare('UPDATE groups SET directory = ? WHERE id = ?')
      .run(directory, groupId);
  }

  /**
   * Update group env vars (replaces all existing)
   */
  updateGroupEnvVars(groupId: string, envVars: Record<string, string>): void {
    const transaction = this.db.transaction(() => {
      // Delete existing env vars
      this.db
        .prepare('DELETE FROM group_env_vars WHERE group_id = ?')
        .run(groupId);

      // Insert new env vars
      for (const [key, value] of Object.entries(envVars)) {
        this.db
          .prepare(
            'INSERT INTO group_env_vars (group_id, key, value) VALUES (?, ?, ?)'
          )
          .run(groupId, key, value);
      }
    });

    transaction();
  }

  /**
   * Delete a group and all its data
   */
  deleteGroup(groupId: string): void {
    // Foreign keys with CASCADE should handle projects, env vars, etc.
    this.db.prepare('DELETE FROM groups WHERE id = ?').run(groupId);
  }

  /**
   * Update group sync settings
   */
  updateGroupSync(
    groupId: string,
    syncFile: string | null,
    syncEnabled: boolean
  ): void {
    this.db
      .prepare('UPDATE groups SET sync_file = ?, sync_enabled = ? WHERE id = ?')
      .run(syncFile, syncEnabled ? 1 : 0, groupId);
  }

  /**
   * Replace a group entirely (for YAML sync)
   */
  replaceGroup(group: Group): void {
    const transaction = this.db.transaction(() => {
      // Delete existing projects and their env vars (cascade)
      this.db
        .prepare('DELETE FROM projects WHERE group_id = ?')
        .run(group.id);

      // Delete existing group env vars
      this.db
        .prepare('DELETE FROM group_env_vars WHERE group_id = ?')
        .run(group.id);

      // Update group fields
      this.db
        .prepare(
          'UPDATE groups SET name = ?, directory = ?, sync_file = ?, sync_enabled = ? WHERE id = ?'
        )
        .run(
          group.name,
          group.directory,
          group.syncFile ?? null,
          group.syncEnabled ? 1 : 0,
          group.id
        );

      // Insert new env vars
      for (const [key, value] of Object.entries(group.envVars)) {
        this.db
          .prepare(
            'INSERT INTO group_env_vars (group_id, key, value) VALUES (?, ?, ?)'
          )
          .run(group.id, key, value);
      }

      // Insert new projects
      for (const project of group.projects) {
        this.insertProject(group.id, project);
      }
    });

    transaction();
  }

  /**
   * Hydrate a group row with projects and env vars
   */
  private hydrateGroup(row: GroupRow): Group {
    const projects = this.getProjectsForGroup(row.id);
    const envVars = this.getGroupEnvVars(row.id);

    return {
      id: row.id,
      name: row.name,
      directory: row.directory,
      projects,
      envVars,
      syncFile: row.sync_file ?? undefined,
      syncEnabled: row.sync_enabled === 1,
    };
  }

  /**
   * Get projects for a group
   */
  private getProjectsForGroup(groupId: string): Project[] {
    const rows = this.db
      .prepare('SELECT * FROM projects WHERE group_id = ? ORDER BY name')
      .all(groupId) as ProjectRow[];

    return rows.map((row) => {
      const envVars = this.getProjectEnvVars(row.id);
      return {
        id: row.id,
        name: row.name,
        command: row.command,
        autoRestart: row.auto_restart === 1,
        envVars,
        cwd: row.cwd,
        projectType: row.project_type as 'task' | 'service',
        interactive: row.interactive === 1,
        watchPatterns: row.watch_patterns ? JSON.parse(row.watch_patterns) : undefined,
      };
    });
  }

  /**
   * Get environment variables for a group
   */
  private getGroupEnvVars(groupId: string): Record<string, string> {
    const rows = this.db
      .prepare('SELECT key, value FROM group_env_vars WHERE group_id = ?')
      .all(groupId) as EnvVarRow[];

    const envVars: Record<string, string> = {};
    for (const row of rows) {
      envVars[row.key] = row.value;
    }
    return envVars;
  }

  /**
   * Get environment variables for a project
   */
  private getProjectEnvVars(projectId: string): Record<string, string> {
    const rows = this.db
      .prepare('SELECT key, value FROM project_env_vars WHERE project_id = ?')
      .all(projectId) as EnvVarRow[];

    const envVars: Record<string, string> = {};
    for (const row of rows) {
      envVars[row.key] = row.value;
    }
    return envVars;
  }

  // ============================================================================
  // Project Operations
  // ============================================================================

  /**
   * Insert a project (internal helper)
   */
  private insertProject(groupId: string, project: Project): void {
    this.db
      .prepare(
        `INSERT INTO projects (id, group_id, name, command, auto_restart, cwd, project_type, interactive, watch_patterns)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run(
        project.id,
        groupId,
        project.name,
        project.command,
        project.autoRestart ? 1 : 0,
        project.cwd,
        project.projectType,
        project.interactive ? 1 : 0,
        project.watchPatterns ? JSON.stringify(project.watchPatterns) : null
      );

    // Insert project env vars
    for (const [key, value] of Object.entries(project.envVars)) {
      this.db
        .prepare(
          'INSERT INTO project_env_vars (project_id, key, value) VALUES (?, ?, ?)'
        )
        .run(project.id, key, value);
    }
  }

  /**
   * Create a new project in a group
   */
  createProject(groupId: string, project: Project): void {
    const transaction = this.db.transaction(() => {
      this.insertProject(groupId, project);
    });

    transaction();
  }

  /**
   * Get a single project by ID
   */
  getProject(projectId: string): (Project & { groupId: string }) | null {
    const row = this.db
      .prepare('SELECT * FROM projects WHERE id = ?')
      .get(projectId) as ProjectRow | undefined;

    if (!row) return null;

    const envVars = this.getProjectEnvVars(row.id);
    return {
      id: row.id,
      name: row.name,
      command: row.command,
      autoRestart: row.auto_restart === 1,
      envVars,
      cwd: row.cwd,
      projectType: row.project_type as 'task' | 'service',
      interactive: row.interactive === 1,
      watchPatterns: row.watch_patterns ? JSON.parse(row.watch_patterns) : undefined,
      groupId: row.group_id,
    };
  }

  /**
   * Update an existing project
   */
  updateProject(project: Project): void {
    const transaction = this.db.transaction(() => {
      // Update project fields
      this.db
        .prepare(
          `UPDATE projects SET name = ?, command = ?, auto_restart = ?, cwd = ?, project_type = ?, interactive = ?, watch_patterns = ?
           WHERE id = ?`
        )
        .run(
          project.name,
          project.command,
          project.autoRestart ? 1 : 0,
          project.cwd,
          project.projectType,
          project.interactive ? 1 : 0,
          project.watchPatterns ? JSON.stringify(project.watchPatterns) : null,
          project.id
        );

      // Replace env vars
      this.db
        .prepare('DELETE FROM project_env_vars WHERE project_id = ?')
        .run(project.id);

      for (const [key, value] of Object.entries(project.envVars)) {
        this.db
          .prepare(
            'INSERT INTO project_env_vars (project_id, key, value) VALUES (?, ?, ?)'
          )
          .run(project.id, key, value);
      }
    });

    transaction();
  }

  /**
   * Delete a project
   */
  deleteProject(projectId: string): void {
    // Foreign keys should cascade delete env vars
    this.db.prepare('DELETE FROM projects WHERE id = ?').run(projectId);
  }

  /**
   * Delete multiple projects
   */
  deleteProjects(projectIds: string[]): void {
    const transaction = this.db.transaction(() => {
      for (const id of projectIds) {
        this.deleteProject(id);
      }
    });

    transaction();
  }

  /**
   * Convert project types (task/service)
   */
  convertProjects(
    conversions: Array<{ projectId: string; projectType: 'task' | 'service' }>
  ): void {
    const transaction = this.db.transaction(() => {
      for (const { projectId, projectType } of conversions) {
        this.db
          .prepare('UPDATE projects SET project_type = ? WHERE id = ?')
          .run(projectType, projectId);
      }
    });

    transaction();
  }

  // ============================================================================
  // Session Operations
  // ============================================================================

  /**
   * Create a new session
   */
  createSession(projectId: string): string {
    const id = crypto.randomUUID();
    const startedAt = Date.now();

    this.db
      .prepare(
        'INSERT INTO sessions (id, project_id, started_at) VALUES (?, ?, ?)'
      )
      .run(id, projectId, startedAt);

    return id;
  }

  /**
   * End a session
   */
  endSession(sessionId: string, exitStatus: string): void {
    const endedAt = Date.now();

    this.db
      .prepare(
        'UPDATE sessions SET ended_at = ?, exit_status = ? WHERE id = ?'
      )
      .run(endedAt, exitStatus, sessionId);
  }

  /**
   * Get sessions for a project
   */
  getProjectSessions(projectId: string): Session[] {
    const rows = this.db
      .prepare(
        'SELECT * FROM sessions WHERE project_id = ? ORDER BY started_at DESC'
      )
      .all(projectId) as SessionRow[];

    return rows.map((row) => ({
      id: row.id,
      projectId: row.project_id,
      startedAt: row.started_at,
      endedAt: row.ended_at,
      exitStatus: row.exit_status,
    }));
  }

  /**
   * Get sessions for a project with stats (log count, log size, metric count)
   */
  getProjectSessionsWithStats(projectId: string): SessionWithStats[] {
    const rows = this.db
      .prepare(
        `SELECT 
          s.*,
          (SELECT COUNT(*) FROM logs WHERE session_id = s.id) as log_count,
          (SELECT COALESCE(SUM(LENGTH(data)), 0) FROM logs WHERE session_id = s.id) as log_size,
          (SELECT COUNT(*) FROM metrics WHERE session_id = s.id) as metric_count
        FROM sessions s
        WHERE s.project_id = ?
        ORDER BY s.started_at DESC`
      )
      .all(projectId) as (SessionRow & {
      log_count: number;
      log_size: number;
      metric_count: number;
    })[];

    return rows.map((row) => ({
      id: row.id,
      projectId: row.project_id,
      startedAt: row.started_at,
      endedAt: row.ended_at,
      exitStatus: row.exit_status,
      logCount: row.log_count,
      logSize: row.log_size,
      metricCount: row.metric_count,
    }));
  }

  /**
   * Get a single session by ID
   */
  getSession(sessionId: string): Session | null {
    const row = this.db
      .prepare('SELECT * FROM sessions WHERE id = ?')
      .get(sessionId) as SessionRow | undefined;

    if (!row) return null;

    return {
      id: row.id,
      projectId: row.project_id,
      startedAt: row.started_at,
      endedAt: row.ended_at,
      exitStatus: row.exit_status,
    };
  }

  /**
   * Get the last completed session for a project
   */
  getLastCompletedSession(projectId: string): Session | null {
    const row = this.db
      .prepare(
        `SELECT * FROM sessions 
         WHERE project_id = ? AND ended_at IS NOT NULL
         ORDER BY ended_at DESC LIMIT 1`
      )
      .get(projectId) as SessionRow | undefined;

    if (!row) return null;

    return {
      id: row.id,
      projectId: row.project_id,
      startedAt: row.started_at,
      endedAt: row.ended_at,
      exitStatus: row.exit_status,
    };
  }

  /**
   * Delete a session and all its data
   */
  deleteSession(sessionId: string): void {
    // Foreign keys should cascade delete logs and metrics
    this.db.prepare('DELETE FROM sessions WHERE id = ?').run(sessionId);
  }

  // ============================================================================
  // Log Operations
  // ============================================================================

  /**
   * Insert a log entry
   */
  insertLog(
    sessionId: string,
    stream: 'stdout' | 'stderr',
    data: string,
    timestamp: number
  ): void {
    this.db
      .prepare(
        'INSERT INTO logs (session_id, stream, data, timestamp) VALUES (?, ?, ?, ?)'
      )
      .run(sessionId, stream, data, timestamp);
  }

  /**
   * Get logs for a session
   */
  getSessionLogs(sessionId: string): LogMessage[] {
    const rows = this.db
      .prepare(
        'SELECT * FROM logs WHERE session_id = ? ORDER BY timestamp ASC'
      )
      .all(sessionId) as LogRow[];

    // We need the project_id from the session
    const session = this.db
      .prepare('SELECT project_id FROM sessions WHERE id = ?')
      .get(sessionId) as { project_id: string } | undefined;

    const projectId = session?.project_id ?? '';

    return rows.map((row) => ({
      projectId,
      stream: row.stream as 'stdout' | 'stderr',
      data: row.data,
      timestamp: row.timestamp,
    }));
  }

  /**
   * Get logs for a session as a single concatenated string
   */
  getSessionLogsAsString(sessionId: string): string {
    const logs = this.getSessionLogs(sessionId);
    return logs.map((log) => log.data).join('');
  }

  /**
   * Get recent logs for a project (from the most recent session)
   */
  getRecentLogs(projectId: string, limit: number): LogMessage[] {
    // Get the most recent session for this project
    const session = this.db
      .prepare(
        'SELECT id FROM sessions WHERE project_id = ? ORDER BY started_at DESC LIMIT 1'
      )
      .get(projectId) as { id: string } | undefined;

    if (!session) return [];

    const rows = this.db
      .prepare(
        `SELECT * FROM logs WHERE session_id = ? ORDER BY timestamp DESC LIMIT ?`
      )
      .all(session.id, limit) as LogRow[];

    // Reverse to get chronological order
    return rows.reverse().map((row) => ({
      projectId,
      stream: row.stream as 'stdout' | 'stderr',
      data: row.data,
      timestamp: row.timestamp,
    }));
  }

  /**
   * Clear logs for a project (all sessions)
   */
  clearProjectLogs(projectId: string): void {
    this.db
      .prepare(
        `DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE project_id = ?)`
      )
      .run(projectId);
  }

  // ============================================================================
  // Metric Operations
  // ============================================================================

  /**
   * Insert a metric entry
   */
  insertMetric(sessionId: string, cpuUsage: number, memoryUsage: number): void {
    const timestamp = Date.now();

    this.db
      .prepare(
        'INSERT INTO metrics (session_id, cpu_usage, memory_usage, timestamp) VALUES (?, ?, ?, ?)'
      )
      .run(sessionId, cpuUsage, memoryUsage, timestamp);
  }

  /**
   * Get metrics for a session
   */
  getSessionMetrics(sessionId: string): MetricPoint[] {
    const rows = this.db
      .prepare(
        'SELECT * FROM metrics WHERE session_id = ? ORDER BY timestamp ASC'
      )
      .all(sessionId) as MetricRow[];

    return rows.map((row) => ({
      timestamp: row.timestamp,
      cpuUsage: row.cpu_usage,
      memoryUsage: row.memory_usage,
    }));
  }

  /**
   * Get the last metric for a session
   */
  getLastMetric(sessionId: string): MetricPoint | null {
    const row = this.db
      .prepare(
        'SELECT * FROM metrics WHERE session_id = ? ORDER BY timestamp DESC LIMIT 1'
      )
      .get(sessionId) as MetricRow | undefined;

    if (!row) return null;

    return {
      timestamp: row.timestamp,
      cpuUsage: row.cpu_usage,
      memoryUsage: row.memory_usage,
    };
  }

  // ============================================================================
  // Settings Operations
  // ============================================================================

  /**
   * Get application settings
   */
  getSettings(): AppSettings {
    const maxLogLines = this.getSetting('max_log_lines', '10000');
    const editor = this.getSetting('editor', '');
    const linuxGpuOpt = this.getSetting('linux_gpu_optimization', '');
    const fullscreen = this.getSetting('fullscreen', '');

    return {
      maxLogLines: parseInt(maxLogLines, 10) || 10000,
      editor: editor || null,
      linuxGpuOptimization:
        linuxGpuOpt === '' ? null : linuxGpuOpt === 'true',
      fullscreen: fullscreen === '' ? null : fullscreen === 'true',
    };
  }

  /**
   * Update application settings
   */
  updateSettings(settings: AppSettings): void {
    this.setSetting('max_log_lines', settings.maxLogLines.toString());
    this.setSetting('editor', settings.editor ?? '');
    this.setSetting(
      'linux_gpu_optimization',
      settings.linuxGpuOptimization === null
        ? ''
        : settings.linuxGpuOptimization.toString()
    );
    this.setSetting(
      'fullscreen',
      settings.fullscreen === null ? '' : settings.fullscreen.toString()
    );
  }

  private getSetting(key: string, defaultValue: string): string {
    const row = this.db
      .prepare('SELECT value FROM settings WHERE key = ?')
      .get(key) as { value: string } | undefined;

    return row?.value ?? defaultValue;
  }

  private setSetting(key: string, value: string): void {
    this.db
      .prepare(
        'INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)'
      )
      .run(key, value);
  }

  // ============================================================================
  // Storage/Maintenance Operations
  // ============================================================================

  /**
   * Get storage statistics
   */
  getStorageStats(): StorageStats {
    const stats = this.db
      .prepare(
        `SELECT 
          (SELECT COUNT(*) FROM sessions) as session_count,
          (SELECT COUNT(*) FROM logs) as log_count,
          (SELECT COALESCE(SUM(LENGTH(data)), 0) FROM logs) as log_size,
          (SELECT COUNT(*) FROM metrics) as metric_count
        `
      )
      .get() as {
      session_count: number;
      log_count: number;
      log_size: number;
      metric_count: number;
    };

    return {
      sessionCount: stats.session_count,
      logCount: stats.log_count,
      totalSize: stats.log_size,
      metricCount: stats.metric_count,
    };
  }

  /**
   * Cleanup sessions older than N days
   */
  cleanupOldSessions(days: number): void {
    const cutoffTime = Date.now() - days * 24 * 60 * 60 * 1000;

    this.db
      .prepare('DELETE FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?')
      .run(cutoffTime);
  }

  /**
   * Cleanup all sessions
   */
  cleanupAllSessions(): void {
    this.db.prepare('DELETE FROM sessions WHERE ended_at IS NOT NULL').run();
  }
}
