import Database from '@tauri-apps/plugin-sql';
import { configDir } from '@tauri-apps/api/path';
import type { Group, Project, AppSettings, Session, SessionWithStats, MetricPoint } from '../types';

// Database paths
let configDbPath: string | null = null;
const GROUPS_DIR = 'groups';

async function getConfigDbPath(): Promise<string> {
  if (!configDbPath) {
    const cfgDir = await configDir();
    configDbPath = `sqlite:${cfgDir}/openrunner/config.db`;
  }
  return configDbPath;
}

// Config database instance
let configDb: Database | null = null;

// Group database cache (LRU with 10 entries)
const groupDbCache = new Map<string, Database>();
const MAX_CACHE_SIZE = 10;

/**
 * Initialize the config database connection
 */
export async function initConfigDb(): Promise<Database> {
  if (!configDb) {
    const dbPath = await getConfigDbPath();
    configDb = await Database.load(dbPath);
  }
  return configDb;
}



/**
 * Get or create a group database connection
 */
export async function getGroupDb(groupId: string): Promise<Database> {
  // Check cache first
  if (groupDbCache.has(groupId)) {
    const db = groupDbCache.get(groupId)!;
    // Move to end (most recently used)
    groupDbCache.delete(groupId);
    groupDbCache.set(groupId, db);
    return db;
  }

  // Create new connection
  const dbPath = `sqlite:${GROUPS_DIR}/${groupId}.db`;
  const db = await Database.load(dbPath);

  // Add to cache
  groupDbCache.set(groupId, db);

  // Evict oldest if needed
  if (groupDbCache.size > MAX_CACHE_SIZE) {
    const firstKey = groupDbCache.keys().next().value;
    if (firstKey) {
      const oldDb = groupDbCache.get(firstKey);
      if (oldDb) {
        await oldDb.close();
      }
      groupDbCache.delete(firstKey);
    }
  }

  return db;
}

/**
 * Close all database connections
 */
export async function closeAllDbs(): Promise<void> {
  if (configDb) {
    await configDb.close();
    configDb = null;
  }

  for (const [_, db] of groupDbCache) {
    await db.close();
  }
  groupDbCache.clear();
}

// ============================================================================
// Group Operations
// ============================================================================

/**
 * Get all groups from the database
 */
export async function getGroups(): Promise<Group[]> {
  const db = await initConfigDb();
  
  const groups = await db.select<{
    id: string;
    name: string;
    directory: string;
    sync_file?: string;
    sync_enabled: number;
  }[]>(`SELECT id, name, directory, sync_file, sync_enabled FROM groups ORDER BY name`);

  return Promise.all(groups.map(async (g) => ({
    id: g.id,
    name: g.name,
    directory: g.directory,
    projects: await getProjects(g.id),
    envVars: await getGroupEnvVars(g.id),
    syncFile: g.sync_file,
    syncEnabled: Boolean(g.sync_enabled),
  })));
}

/**
 * Get a single group by ID
 */
export async function getGroup(groupId: string): Promise<Group | null> {
  const db = await initConfigDb();
  
  const groups = await db.select<{
    id: string;
    name: string;
    directory: string;
    sync_file?: string;
    sync_enabled: number;
  }[]>(`SELECT id, name, directory, sync_file, sync_enabled FROM groups WHERE id = $1`, [groupId]);

  if (groups.length === 0) return null;

  const g = groups[0];
  return {
    id: g.id,
    name: g.name,
    directory: g.directory,
    projects: await getProjects(g.id),
    envVars: await getGroupEnvVars(g.id),
    syncFile: g.sync_file,
    syncEnabled: Boolean(g.sync_enabled),
  };
}

/**
 * Create a new group
 */
export async function createGroup(group: Group): Promise<void> {
  const db = await initConfigDb();
  
  await db.execute(
    `INSERT INTO groups (id, name, directory, sync_file, sync_enabled) 
     VALUES ($1, $2, $3, $4, $5)`,
    [group.id, group.name, group.directory, group.syncFile || null, group.syncEnabled ? 1 : 0]
  );

  // Insert env vars
  for (const [key, value] of Object.entries(group.envVars)) {
    await db.execute(
      `INSERT INTO group_env_vars (group_id, key, value) VALUES ($1, $2, $3)`,
      [group.id, key, value]
    );
  }

  // Insert projects
  for (const project of group.projects) {
    await createProject(group.id, project);
  }
}

/**
 * Update a group
 */
export async function updateGroup(groupId: string, updates: Partial<Group>): Promise<void> {
  const db = await initConfigDb();
  
  if (updates.name !== undefined) {
    await db.execute(`UPDATE groups SET name = $1 WHERE id = $2`, [updates.name, groupId]);
  }
  
  if (updates.directory !== undefined) {
    await db.execute(`UPDATE groups SET directory = $1 WHERE id = $2`, [updates.directory, groupId]);
  }
  
  if (updates.syncFile !== undefined) {
    await db.execute(`UPDATE groups SET sync_file = $1 WHERE id = $2`, [updates.syncFile, groupId]);
  }
  
  if (updates.syncEnabled !== undefined) {
    await db.execute(`UPDATE groups SET sync_enabled = $1 WHERE id = $2`, [updates.syncEnabled ? 1 : 0, groupId]);
  }

  if (updates.envVars !== undefined) {
    // Delete old env vars
    await db.execute(`DELETE FROM group_env_vars WHERE group_id = $1`, [groupId]);
    
    // Insert new env vars
    for (const [key, value] of Object.entries(updates.envVars)) {
      await db.execute(
        `INSERT INTO group_env_vars (group_id, key, value) VALUES ($1, $2, $3)`,
        [groupId, key, value]
      );
    }
  }
}

/**
 * Delete a group and all its data
 */
export async function deleteGroup(groupId: string): Promise<void> {
  const db = await initConfigDb();
  
  // Delete from config database (cascade will handle related data)
  await db.execute(`DELETE FROM groups WHERE id = $1`, [groupId]);
  
  // Close and remove from cache
  if (groupDbCache.has(groupId)) {
    const groupDb = groupDbCache.get(groupId)!;
    await groupDb.close();
    groupDbCache.delete(groupId);
  }
}

/**
 * Get group environment variables
 */
async function getGroupEnvVars(groupId: string): Promise<Record<string, string>> {
  const db = await initConfigDb();
  
  const rows = await db.select<{ key: string; value: string }[]>(
    `SELECT key, value FROM group_env_vars WHERE group_id = $1`,
    [groupId]
  );

  return Object.fromEntries(rows.map(r => [r.key, r.value]));
}

// ============================================================================
// Project Operations
// ============================================================================

/**
 * Get all projects for a group
 */
export async function getProjects(groupId: string): Promise<Project[]> {
  const db = await initConfigDb();
  
  const projects = await db.select<{
    id: string;
    name: string;
    command: string;
    auto_restart: number;
    cwd: string | null;
    project_type: string;
    interactive: number;
  }[]>(
    `SELECT id, name, command, auto_restart, cwd, project_type, interactive 
     FROM projects WHERE group_id = $1 ORDER BY name`,
    [groupId]
  );

  return Promise.all(projects.map(async (p) => ({
    id: p.id,
    name: p.name,
    command: p.command,
    autoRestart: Boolean(p.auto_restart),
    cwd: p.cwd,
    projectType: p.project_type as 'task' | 'service',
    interactive: Boolean(p.interactive),
    envVars: await getProjectEnvVars(p.id),
  })));
}

/**
 * Get a single project by ID
 */
export async function getProject(projectId: string): Promise<Project | null> {
  const db = await initConfigDb();
  
  const projects = await db.select<{
    id: string;
    name: string;
    command: string;
    auto_restart: number;
    cwd: string | null;
    project_type: string;
    interactive: number;
  }[]>(
    `SELECT id, name, command, auto_restart, cwd, project_type, interactive 
     FROM projects WHERE id = $1`,
    [projectId]
  );

  if (projects.length === 0) return null;

  const p = projects[0];
  return {
    id: p.id,
    name: p.name,
    command: p.command,
    autoRestart: Boolean(p.auto_restart),
    cwd: p.cwd,
    projectType: p.project_type as 'task' | 'service',
    interactive: Boolean(p.interactive),
    envVars: await getProjectEnvVars(p.id),
  };
}

/**
 * Create a new project
 */
export async function createProject(groupId: string, project: Project): Promise<void> {
  const db = await initConfigDb();
  
  await db.execute(
    `INSERT INTO projects (id, group_id, name, command, auto_restart, cwd, project_type, interactive) 
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
    [
      project.id,
      groupId,
      project.name,
      project.command,
      project.autoRestart ? 1 : 0,
      project.cwd || null,
      project.projectType,
      project.interactive ? 1 : 0,
    ]
  );

  // Insert env vars
  for (const [key, value] of Object.entries(project.envVars)) {
    await db.execute(
      `INSERT INTO project_env_vars (project_id, key, value) VALUES ($1, $2, $3)`,
      [project.id, key, value]
    );
  }
}

/**
 * Update a project
 */
export async function updateProject(project: Project): Promise<void> {
  const db = await initConfigDb();
  
  await db.execute(
    `UPDATE projects SET name = $1, command = $2, auto_restart = $3, cwd = $4, project_type = $5, interactive = $6 
     WHERE id = $7`,
    [
      project.name,
      project.command,
      project.autoRestart ? 1 : 0,
      project.cwd || null,
      project.projectType,
      project.interactive ? 1 : 0,
      project.id,
    ]
  );

  // Update env vars
  await db.execute(`DELETE FROM project_env_vars WHERE project_id = $1`, [project.id]);
  
  for (const [key, value] of Object.entries(project.envVars)) {
    await db.execute(
      `INSERT INTO project_env_vars (project_id, key, value) VALUES ($1, $2, $3)`,
      [project.id, key, value]
    );
  }
}

/**
 * Delete a project
 */
export async function deleteProject(projectId: string): Promise<void> {
  const db = await initConfigDb();
  await db.execute(`DELETE FROM projects WHERE id = $1`, [projectId]);
}

/**
 * Get project environment variables
 */
async function getProjectEnvVars(projectId: string): Promise<Record<string, string>> {
  const db = await initConfigDb();
  
  const rows = await db.select<{ key: string; value: string }[]>(
    `SELECT key, value FROM project_env_vars WHERE project_id = $1`,
    [projectId]
  );

  return Object.fromEntries(rows.map(r => [r.key, r.value]));
}

// ============================================================================
// Settings Operations
// ============================================================================

/**
 * Get application settings
 */
export async function getSettings(): Promise<AppSettings> {
  const db = await initConfigDb();
  
  const settings = await db.select<{ key: string; value: string }[]>(
    `SELECT key, value FROM settings`
  );

  const result: AppSettings = {
    maxLogLines: 10000,
    editor: null,
  };

  for (const setting of settings) {
    switch (setting.key) {
      case 'max_log_lines':
        result.maxLogLines = parseInt(setting.value, 10) || 10000;
        break;
      case 'editor':
        if (setting.value) {
          result.editor = setting.value;
        }
        break;
    }
  }

  return result;
}

/**
 * Update application settings
 */
export async function updateSettings(settings: AppSettings): Promise<void> {
  const db = await initConfigDb();
  
  await db.execute(
    `INSERT OR REPLACE INTO settings (key, value) VALUES ('max_log_lines', $1)`,
    [settings.maxLogLines.toString()]
  );
  
  await db.execute(
    `INSERT OR REPLACE INTO settings (key, value) VALUES ('editor', $1)`,
    [settings.editor || '']
  );
}

// ============================================================================
// Session Operations (Group DB)
// ============================================================================

/**
 * Create a new session
 */
export async function createSession(groupId: string, projectId: string): Promise<string> {
  const db = await getGroupDb(groupId);
  const sessionId = crypto.randomUUID();
  const now = Date.now();

  await db.execute(
    `INSERT INTO sessions (id, project_id, started_at) VALUES ($1, $2, $3)`,
    [sessionId, projectId, now]
  );

  return sessionId;
}

/**
 * End a session
 */
export async function endSession(groupId: string, sessionId: string, exitStatus: string): Promise<void> {
  const db = await getGroupDb(groupId);
  const now = Date.now();

  await db.execute(
    `UPDATE sessions SET ended_at = $1, exit_status = $2 WHERE id = $3`,
    [now, exitStatus, sessionId]
  );
}

/**
 * Get all sessions for a project
 */
export async function getProjectSessions(groupId: string, projectId: string): Promise<Session[]> {
  const db = await getGroupDb(groupId);

  const rows = await db.select<{
    id: string;
    project_id: string;
    started_at: number;
    ended_at: number | null;
    exit_status: string | null;
  }[]>(
    `SELECT id, project_id, started_at, ended_at, exit_status 
     FROM sessions WHERE project_id = $1 ORDER BY started_at DESC`,
    [projectId]
  );

  return rows.map(row => ({
    id: row.id,
    projectId: row.project_id,
    startedAt: row.started_at,
    endedAt: row.ended_at,
    exitStatus: row.exit_status,
  }));
}

/**
 * Get sessions with statistics
 */
export async function getProjectSessionsWithStats(
  groupId: string, 
  projectId: string
): Promise<SessionWithStats[]> {
  const db = await getGroupDb(groupId);

  const rows = await db.select<{
    id: string;
    project_id: string;
    started_at: number;
    ended_at: number | null;
    exit_status: string | null;
    log_count: number;
    log_size: number;
    metric_count: number;
  }[]>(
    `SELECT s.id, s.project_id, s.started_at, s.ended_at, s.exit_status,
            COALESCE(l.log_count, 0) as log_count, 
            COALESCE(l.log_size, 0) as log_size,
            COALESCE(m.metric_count, 0) as metric_count
     FROM sessions s
     LEFT JOIN (
       SELECT session_id, COUNT(*) as log_count, COALESCE(SUM(LENGTH(data)), 0) as log_size 
       FROM logs GROUP BY session_id
     ) l ON l.session_id = s.id
     LEFT JOIN (
       SELECT session_id, COUNT(*) as metric_count 
       FROM metrics GROUP BY session_id
     ) m ON m.session_id = s.id
     WHERE s.project_id = $1
     ORDER BY s.started_at DESC`,
    [projectId]
  );

  return rows.map(row => ({
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
 * Get the last completed session for a project
 */
export async function getLastCompletedSession(
  groupId: string, 
  projectId: string
): Promise<Session | null> {
  const db = await getGroupDb(groupId);

  const rows = await db.select<{
    id: string;
    project_id: string;
    started_at: number;
    ended_at: number | null;
    exit_status: string | null;
  }[]>(
    `SELECT id, project_id, started_at, ended_at, exit_status 
     FROM sessions 
     WHERE project_id = $1 AND ended_at IS NOT NULL 
     ORDER BY ended_at DESC LIMIT 1`,
    [projectId]
  );

  if (rows.length === 0) return null;

  const row = rows[0];
  return {
    id: row.id,
    projectId: row.project_id,
    startedAt: row.started_at,
    endedAt: row.ended_at,
    exitStatus: row.exit_status,
  };
}

// ============================================================================
// Log Operations (Group DB)
// ============================================================================

/**
 * Insert a log entry
 */
export async function insertLog(
  groupId: string,
  sessionId: string,
  stream: 'stdout' | 'stderr',
  data: string
): Promise<void> {
  const db = await getGroupDb(groupId);
  const now = Date.now();

  await db.execute(
    `INSERT INTO logs (session_id, stream, data, timestamp) VALUES ($1, $2, $3, $4)`,
    [sessionId, stream, data, now]
  );
}

/**
 * Get logs for a session
 */
export async function getSessionLogs(groupId: string, sessionId: string): Promise<string> {
  const db = await getGroupDb(groupId);

  const rows = await db.select<{ data: string }[]>(
    `SELECT data FROM logs WHERE session_id = $1 ORDER BY timestamp ASC, id ASC`,
    [sessionId]
  );

  return rows.map(r => r.data).join('');
}

/**
 * Get logs for the latest session of a project
 */
export async function getProjectLogs(groupId: string, projectId: string): Promise<string> {
  const db = await getGroupDb(groupId);

  const sessions = await db.select<{ id: string }[]>(
    `SELECT id FROM sessions WHERE project_id = $1 ORDER BY started_at DESC LIMIT 1`,
    [projectId]
  );

  if (sessions.length === 0) return '';

  return getSessionLogs(groupId, sessions[0].id);
}

/**
 * Get recent logs (limited count)
 */
export async function getRecentLogs(
  groupId: string, 
  projectId: string, 
  limit: number
): Promise<string> {
  const db = await getGroupDb(groupId);

  const sessions = await db.select<{ id: string }[]>(
    `SELECT id FROM sessions WHERE project_id = $1 ORDER BY started_at DESC LIMIT 1`,
    [projectId]
  );

  if (sessions.length === 0) return '';

  const rows = await db.select<{ data: string }[]>(
    `SELECT data FROM (
      SELECT data FROM logs 
      WHERE session_id = $1 
      ORDER BY timestamp DESC, id DESC LIMIT $2
    ) sub ORDER BY timestamp ASC, id ASC`,
    [sessions[0].id, limit]
  );

  return rows.map(r => r.data).join('');
}

/**
 * Clear all logs for a project
 */
export async function clearProjectLogs(groupId: string, projectId: string): Promise<void> {
  const db = await getGroupDb(groupId);

  await db.execute(
    `DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE project_id = $1)`,
    [projectId]
  );
}

// ============================================================================
// Metric Operations (Group DB)
// ============================================================================

/**
 * Insert a metric
 */
export async function insertMetric(
  groupId: string,
  sessionId: string,
  cpuUsage: number,
  memoryUsage: number
): Promise<void> {
  const db = await getGroupDb(groupId);
  const now = Date.now();

  await db.execute(
    `INSERT INTO metrics (session_id, cpu_usage, memory_usage, timestamp) VALUES ($1, $2, $3, $4)`,
    [sessionId, cpuUsage, memoryUsage, now]
  );
}

/**
 * Get all metrics for a session
 */
export async function getSessionMetrics(
  groupId: string, 
  sessionId: string
): Promise<MetricPoint[]> {
  const db = await getGroupDb(groupId);

  const rows = await db.select<{
    timestamp: number;
    cpu_usage: number;
    memory_usage: number;
  }[]>(
    `SELECT timestamp, cpu_usage, memory_usage 
     FROM metrics WHERE session_id = $1 ORDER BY timestamp ASC`,
    [sessionId]
  );

  return rows.map(row => ({
    timestamp: row.timestamp,
    cpuUsage: row.cpu_usage,
    memoryUsage: row.memory_usage,
  }));
}

/**
 * Get the last metric for a session
 */
export async function getLastMetric(
  groupId: string, 
  sessionId: string
): Promise<MetricPoint | null> {
  const db = await getGroupDb(groupId);

  const rows = await db.select<{
    timestamp: number;
    cpu_usage: number;
    memory_usage: number;
  }[]>(
    `SELECT timestamp, cpu_usage, memory_usage 
     FROM metrics WHERE session_id = $1 ORDER BY timestamp DESC LIMIT 1`,
    [sessionId]
  );

  if (rows.length === 0) return null;

  return {
    timestamp: rows[0].timestamp,
    cpuUsage: rows[0].cpu_usage,
    memoryUsage: rows[0].memory_usage,
  };
}

// ============================================================================
// Storage Statistics
// ============================================================================

/**
 * Get storage statistics for a group
 */
export async function getGroupStorageStats(
  groupId: string
): Promise<{ sessionCount: number; logCount: number; logSize: number; metricCount: number }> {
  const db = await getGroupDb(groupId);

  const [sessionResult, logResult, logSizeResult, metricResult] = await Promise.all([
    db.select<{ count: number }[]>(`SELECT COUNT(*) as count FROM sessions`),
    db.select<{ count: number }[]>(`SELECT COUNT(*) as count FROM logs`),
    db.select<{ size: number }[]>(`SELECT COALESCE(SUM(LENGTH(data)), 0) as size FROM logs`),
    db.select<{ count: number }[]>(`SELECT COUNT(*) as count FROM metrics`),
  ]);

  return {
    sessionCount: sessionResult[0]?.count || 0,
    logCount: logResult[0]?.count || 0,
    logSize: logSizeResult[0]?.size || 0,
    metricCount: metricResult[0]?.count || 0,
  };
}

/**
 * Clean up old data from a group
 */
export async function cleanupGroupData(groupId: string, daysToKeep: number): Promise<void> {
  const db = await getGroupDb(groupId);
  const cutoff = Date.now() - (daysToKeep * 24 * 60 * 60 * 1000);

  await db.execute(
    `DELETE FROM sessions WHERE ended_at IS NOT NULL AND ended_at < $1`,
    [cutoff]
  );
}

/**
 * Clean up all data from a group (destructive!)
 */
export async function cleanupAllGroupData(groupId: string): Promise<void> {
  const db = await getGroupDb(groupId);

  await db.execute(`DELETE FROM metrics`);
  await db.execute(`DELETE FROM logs`);
  await db.execute(`DELETE FROM sessions`);
}
