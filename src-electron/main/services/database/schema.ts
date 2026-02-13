/**
 * Database schema initialization.
 * This is the equivalent of src-tauri/src/database/schema.rs
 */

import type { Database } from 'better-sqlite3';

/**
 * Initialize the database schema
 */
export function initSchema(db: Database): void {
  db.exec(`
    -- Groups table
    CREATE TABLE IF NOT EXISTS groups (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      directory TEXT NOT NULL,
      sync_file TEXT,
      sync_enabled INTEGER NOT NULL DEFAULT 0
    );

    -- Projects table with foreign key to groups
    CREATE TABLE IF NOT EXISTS projects (
      id TEXT PRIMARY KEY,
      group_id TEXT NOT NULL,
      name TEXT NOT NULL,
      command TEXT NOT NULL,
      auto_restart INTEGER NOT NULL DEFAULT 0,
      cwd TEXT,
      project_type TEXT NOT NULL DEFAULT 'service',
      interactive INTEGER NOT NULL DEFAULT 0,
      watch_patterns TEXT,
      FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
    );

    -- Group environment variables
    CREATE TABLE IF NOT EXISTS group_env_vars (
      group_id TEXT NOT NULL,
      key TEXT NOT NULL,
      value TEXT NOT NULL,
      PRIMARY KEY (group_id, key),
      FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
    );

    -- Project environment variables
    CREATE TABLE IF NOT EXISTS project_env_vars (
      project_id TEXT NOT NULL,
      key TEXT NOT NULL,
      value TEXT NOT NULL,
      PRIMARY KEY (project_id, key),
      FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Application settings
    CREATE TABLE IF NOT EXISTS settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );

    -- Sessions table with foreign key to projects
    CREATE TABLE IF NOT EXISTS sessions (
      id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      started_at INTEGER NOT NULL,
      ended_at INTEGER,
      exit_status TEXT,
      FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Logs table with foreign key to sessions
    CREATE TABLE IF NOT EXISTS logs (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id TEXT NOT NULL,
      stream TEXT NOT NULL,
      data TEXT NOT NULL,
      timestamp INTEGER NOT NULL,
      FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    );

    -- Metrics table with foreign key to sessions
    CREATE TABLE IF NOT EXISTS metrics (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id TEXT NOT NULL,
      cpu_usage REAL NOT NULL,
      memory_usage INTEGER NOT NULL,
      timestamp INTEGER NOT NULL,
      FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    );

    -- Insert default settings
    INSERT OR IGNORE INTO settings (key, value) VALUES ('max_log_lines', '10000');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('editor', '');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('fullscreen', 'false');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('shell', '');
    INSERT OR IGNORE INTO settings (key, value) VALUES ('auto_launch', 'false');

    -- Indexes for better query performance
    CREATE INDEX IF NOT EXISTS idx_projects_group ON projects(group_id);
    CREATE INDEX IF NOT EXISTS idx_env_vars_group ON group_env_vars(group_id);
    CREATE INDEX IF NOT EXISTS idx_env_vars_project ON project_env_vars(project_id);
    CREATE INDEX IF NOT EXISTS idx_logs_session ON logs(session_id);
    CREATE INDEX IF NOT EXISTS idx_metrics_session ON metrics(session_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
    CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);
    CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics(timestamp);
  `);
}
