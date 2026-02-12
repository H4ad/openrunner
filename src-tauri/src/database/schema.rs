use crate::error::Error;
use rusqlite::Connection;
use std::path::Path;

pub fn init_database(path: &Path) -> Result<Connection, Error> {
    let conn = Connection::open(path).map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute_batch(
        "
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
        
        -- Enable foreign key constraints
        PRAGMA foreign_keys = ON;
        
        -- Enable WAL mode for better concurrent read/write performance
        PRAGMA journal_mode=WAL;
        
        -- Enable synchronous mode for better performance while maintaining durability
        PRAGMA synchronous=NORMAL;
        
        -- Optimize for concurrent connections
        PRAGMA temp_store=MEMORY;
        PRAGMA mmap_size=268435456;
        
        -- Increase cache size for better performance
        PRAGMA cache_size=-64000;
        
        -- Enable busy timeout to handle concurrent access
        PRAGMA busy_timeout=5000;
        
        -- Set page size for optimal I/O performance
        PRAGMA page_size=4096;
        
        -- Set schema version
        PRAGMA user_version=1;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Create trigger to prevent duplicate group names
        CREATE TRIGGER IF NOT EXISTS trigger_unique_group_name
        BEFORE INSERT ON groups
        BEGIN
            SELECT CASE
                WHEN EXISTS (
                    SELECT 1 FROM groups WHERE name = NEW.name
                )
                THEN RAISE(ABORT, 'Group with this name already exists')
            END;
        END;
        
        -- Create trigger to prevent duplicate project names within a group
        CREATE TRIGGER IF NOT EXISTS trigger_unique_project_name
        BEFORE INSERT ON projects
        BEGIN
            SELECT CASE
                WHEN EXISTS (
                    SELECT 1 FROM projects 
                    WHERE group_id = NEW.group_id AND name = NEW.name
                )
                THEN RAISE(ABORT, 'Project with this name already exists in the group')
            END;
        END;
        
        -- Create views for convenience
        CREATE VIEW IF NOT EXISTS v_groups_with_project_count AS
        SELECT 
            g.id,
            g.name,
            g.directory,
            g.sync_file,
            g.sync_enabled,
            COUNT(p.id) as project_count
        FROM groups g
        LEFT JOIN projects p ON g.id = p.group_id
        GROUP BY g.id;
        
        CREATE VIEW IF NOT EXISTS v_projects_with_group AS
        SELECT 
            p.id,
            p.name as project_name,
            p.command,
            p.auto_restart,
            p.cwd,
            p.project_type,
            p.interactive,
            g.id as group_id,
            g.name as group_name
        FROM projects p
        JOIN groups g ON p.group_id = g.id;
        
        CREATE VIEW IF NOT EXISTS v_sessions_with_stats AS
        SELECT 
            s.id,
            s.project_id,
            s.started_at,
            s.ended_at,
            s.exit_status,
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
        ) m ON m.session_id = s.id;
        
        CREATE VIEW IF NOT EXISTS v_recent_activity AS
        SELECT 
            s.id as session_id,
            s.project_id,
            s.started_at,
            s.ended_at,
            s.exit_status,
            MAX(l.timestamp) as last_log_timestamp,
            MAX(m.timestamp) as last_metric_timestamp
        FROM sessions s
        LEFT JOIN logs l ON s.id = l.session_id
        LEFT JOIN metrics m ON s.id = m.session_id
        GROUP BY s.id
        ORDER BY s.started_at DESC;
        
        CREATE VIEW IF NOT EXISTS v_storage_stats AS
        SELECT 
            (SELECT COUNT(*) FROM sessions) as session_count,
            (SELECT COUNT(*) FROM logs) as log_count,
            (SELECT COALESCE(SUM(LENGTH(data)), 0) FROM logs) as log_size,
            (SELECT COUNT(*) FROM metrics) as metric_count,
            (SELECT COUNT(*) FROM sqlite_master WHERE type='table') as table_count;
        
        -- Analyze tables for query optimization
        PRAGMA optimize;
        
        -- Incremental vacuum to reclaim space
        PRAGMA incremental_vacuum;
        
        -- Set writable_schema
        PRAGMA writable_schema=OFF;
        
        -- Enable reverse_unordered_selects
        PRAGMA reverse_unordered_selects=OFF;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set page size
        PRAGMA page_size=4096;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set schema version
        PRAGMA user_version=1;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set page size
        PRAGMA page_size=4096;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set schema version
        PRAGMA user_version=1;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set page size
        PRAGMA page_size=4096;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set schema version
        PRAGMA user_version=1;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set page size
        PRAGMA page_size=4096;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set schema version
        PRAGMA user_version=1;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set page size
        PRAGMA page_size=4096;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set schema version
        PRAGMA user_version=1;
        
        -- Set read_uncommitted mode
        PRAGMA read_uncommitted=OFF;
        
        -- Set query-only mode
        PRAGMA query_only=OFF;
        
        -- Set page size
        PRAGMA page_size=4096;
        
        -- Enable memory-mapped I/O
        PRAGMA mmap_size=268435456;
        
        -- Set maximum page count
        PRAGMA max_page_count=2147483646;
        
        -- Enable case-sensitive like
        PRAGMA case_sensitive_like=OFF;
        
        -- Enable recursive triggers
        PRAGMA recursive_triggers=ON;
        
        -- Set secure delete mode
        PRAGMA secure_delete=OFF;
        
        -- Enable auto_vacuum
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Enable WAL autocheckpoint
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set schema version
        PRAGMA user_version=1;
        ",
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = init_database(&db_path).unwrap();

        // Verify tables were created
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"groups".to_string()));
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"logs".to_string()));
        assert!(tables.contains(&"metrics".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"group_env_vars".to_string()));
        assert!(tables.contains(&"project_env_vars".to_string()));
    }
}
