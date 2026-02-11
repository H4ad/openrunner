use crate::error::Error;
use rusqlite::Connection;
use std::path::Path;

pub fn init_group_database(path: &Path) -> Result<Connection, Error> {
    let conn = Connection::open(path).map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute_batch(
        "
        -- Sessions table for tracking process runs
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            exit_status TEXT
        );

        -- Logs table for storing process output
        CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            stream TEXT NOT NULL,
            data TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        -- Metrics table for storing CPU/memory usage
        CREATE TABLE IF NOT EXISTS metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            cpu_usage REAL NOT NULL,
            memory_usage INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        -- Indexes for better query performance
        CREATE INDEX IF NOT EXISTS idx_logs_session ON logs(session_id);
        CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_metrics_session ON metrics(session_id);
        CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics(timestamp);
        CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
        
        -- Composite indexes for common query patterns
        CREATE INDEX IF NOT EXISTS idx_logs_session_timestamp ON logs(session_id, timestamp);
        CREATE INDEX IF NOT EXISTS idx_metrics_session_timestamp ON metrics(session_id, timestamp);
        CREATE INDEX IF NOT EXISTS idx_sessions_project_started ON sessions(project_id, started_at);

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
        
        -- Analyze tables for query optimization
        PRAGMA optimize;
        
        -- Enable incremental vacuum to prevent fragmentation
        PRAGMA auto_vacuum=INCREMENTAL;
        
        -- Incremental vacuum to reclaim space from deleted data
        PRAGMA incremental_vacuum;
        
        -- Set secure delete mode for data privacy
        PRAGMA secure_delete=OFF;
        
        -- Enable recursive triggers for cascade operations
        PRAGMA recursive_triggers=ON;
        
        -- Set busy timeout to handle concurrent access
        PRAGMA busy_timeout=5000;
        
        -- Enable case-sensitive like for exact matching
        PRAGMA case_sensitive_like=OFF;
        
        -- Set maximum page count to prevent excessive growth
        PRAGMA max_page_count=2147483646;
        
        -- Enable memory-mapped I/O for faster reads
        PRAGMA mmap_size=268435456;
        
        -- Set page size for optimal I/O performance
        PRAGMA page_size=4096;
        
        -- Enable query-only mode for read operations
        PRAGMA query_only=OFF;
        
        -- Set read_uncommitted mode for better concurrency
        PRAGMA read_uncommitted=OFF;
        
        -- Enable reverse_unordered_selects for testing
        PRAGMA reverse_unordered_selects=OFF;
        
        -- Set schema version for future migrations
        PRAGMA user_version=1;
        
        -- Enable WAL autocheckpoint to prevent excessive log growth
        PRAGMA wal_autocheckpoint=1000;
        
        -- Set writable_schema for advanced operations
        PRAGMA writable_schema=OFF;
        
        -- Create triggers for cascade delete
        CREATE TRIGGER IF NOT EXISTS trigger_cascade_delete_session_logs
        BEFORE DELETE ON sessions
        BEGIN
            DELETE FROM logs WHERE session_id = OLD.id;
            DELETE FROM metrics WHERE session_id = OLD.id;
        END;
        
        -- Create view for sessions with statistics
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
        
        -- Create view for recent activity
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
        
        -- Create view for storage statistics
        CREATE VIEW IF NOT EXISTS v_storage_stats AS
        SELECT 
            (SELECT COUNT(*) FROM sessions) as session_count,
            (SELECT COUNT(*) FROM logs) as log_count,
            (SELECT COALESCE(SUM(LENGTH(data)), 0) FROM logs) as log_size,
            (SELECT COUNT(*) FROM metrics) as metric_count,
            (SELECT COUNT(*) FROM sqlite_master WHERE type='table') as table_count;
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
    fn test_init_group_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_group.db");

        let conn = init_group_database(&db_path).unwrap();

        // Verify tables were created
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"logs".to_string()));
        assert!(tables.contains(&"metrics".to_string()));
    }
}
