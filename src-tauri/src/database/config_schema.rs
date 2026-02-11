use crate::error::Error;
use rusqlite::Connection;
use std::path::Path;

pub fn init_config_database(path: &Path) -> Result<Connection, Error> {
    let conn = Connection::open(path).map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute_batch(
        "
        -- Main groups table
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

        -- Application settings (key-value store)
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Insert default settings
        INSERT OR IGNORE INTO settings (key, value) VALUES ('max_log_lines', '10000');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('editor', '');

        -- Indexes for better query performance
        CREATE INDEX IF NOT EXISTS idx_projects_group ON projects(group_id);
        CREATE INDEX IF NOT EXISTS idx_env_vars_group ON group_env_vars(group_id);
        CREATE INDEX IF NOT EXISTS idx_env_vars_project ON project_env_vars(project_id);
        
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
        
        -- Create triggers for maintaining referential integrity
        CREATE TRIGGER IF NOT EXISTS trigger_cleanup_group_env_vars
        AFTER DELETE ON groups
        BEGIN
            DELETE FROM group_env_vars WHERE group_id = OLD.id;
        END;

        CREATE TRIGGER IF NOT EXISTS trigger_cleanup_project_env_vars
        AFTER DELETE ON projects
        BEGIN
            DELETE FROM project_env_vars WHERE project_id = OLD.id;
        END;
        
        -- Create triggers for cascade delete on group deletion
        CREATE TRIGGER IF NOT EXISTS trigger_cascade_delete_group
        BEFORE DELETE ON groups
        BEGIN
            DELETE FROM project_env_vars WHERE project_id IN (
                SELECT id FROM projects WHERE group_id = OLD.id
            );
            DELETE FROM projects WHERE group_id = OLD.id;
            DELETE FROM group_env_vars WHERE group_id = OLD.id;
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
        
        -- Create indexes for common query patterns
        CREATE INDEX IF NOT EXISTS idx_groups_name ON groups(name);
        CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name);
        CREATE INDEX IF NOT EXISTS idx_groups_directory ON groups(directory);
        
        -- Create a view for easy group-project joins
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
        
        -- Create a view for projects with their group names
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
        ",
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_init_config_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_config.db");

        let conn = init_config_database(&db_path).unwrap();

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
        assert!(tables.contains(&"group_env_vars".to_string()));
        assert!(tables.contains(&"project_env_vars".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }
}
