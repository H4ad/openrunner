use crate::database::group_schema::init_group_database;
use crate::error::Error;
use crate::models::{LogStream, MetricPoint, Session, SessionWithStats};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Entry in the LRU cache
struct CacheEntry {
    conn: Connection,
    last_accessed: std::time::Instant,
}

/// Manages per-group SQLite databases with LRU caching
pub struct GroupDbManager {
    groups_dir: PathBuf,
    cache: Mutex<HashMap<String, CacheEntry>>,
    max_cache_size: usize,
}

impl GroupDbManager {
    /// Create a new GroupDbManager
    pub fn new(groups_dir: PathBuf) -> Self {
        Self {
            groups_dir,
            cache: Mutex::new(HashMap::new()),
            max_cache_size: 10,
        }
    }

    /// Get the path for a group's database
    fn group_db_path(&self, group_id: &str) -> PathBuf {
        self.groups_dir.join(format!("{}.db", group_id))
    }

    /// Get a connection to a group's database (cached)
    pub fn get_connection(&self, group_id: &str) -> Result<Connection, Error> {
        let mut cache = self.cache.lock().unwrap();

        // Check if already in cache
        if let Some(entry) = cache.get_mut(group_id) {
            entry.last_accessed = std::time::Instant::now();
            // We can't return the connection directly since it's behind the mutex
            // Instead, we'll open a new connection or use the cached one
            // For simplicity, we'll open a new connection each time
            // and let SQLite handle the caching via WAL mode
            drop(cache);
            return self.open_connection(group_id);
        }

        // Not in cache, open new connection
        drop(cache);
        let conn = self.open_connection(group_id)?;

        // Add to cache if there's room
        self.add_to_cache(group_id, &conn)?;

        Ok(conn)
    }

    /// Open a connection to a group's database
    fn open_connection(&self, group_id: &str) -> Result<Connection, Error> {
        let db_path = self.group_db_path(group_id);

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = init_group_database(&db_path)?;
        Ok(conn)
    }

    /// Add a connection to the cache, evicting oldest if needed
    fn add_to_cache(&self, group_id: &str, _conn: &Connection) -> Result<(), Error> {
        let mut cache = self.cache.lock().unwrap();

        // Evict oldest entry if at capacity
        if cache.len() >= self.max_cache_size {
            let oldest_key = cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(key, _)| key.clone());

            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }

        // We store a placeholder since we'll reopen connections as needed
        // This keeps the LRU logic working
        let db_path = self.group_db_path(group_id);
        let conn = init_group_database(&db_path)?;
        cache.insert(
            group_id.to_string(),
            CacheEntry {
                conn,
                last_accessed: std::time::Instant::now(),
            },
        );

        Ok(())
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Remove a specific group from cache
    pub fn invalidate_cache(&self, group_id: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(group_id);
    }

    /// Delete a group's database file
    pub fn delete_group_db(&self, group_id: &str) -> Result<(), Error> {
        // Remove from cache first
        self.invalidate_cache(group_id);

        let db_path = self.group_db_path(group_id);
        if db_path.exists() {
            std::fs::remove_file(&db_path)?;
        }

        Ok(())
    }

    // ============================================================================
    // Session Operations
    // ============================================================================

    /// Create a new session
    pub fn create_session(&self, group_id: &str, project_id: &str) -> Result<String, Error> {
        let conn = self.get_connection(group_id)?;
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        conn.execute(
            "INSERT INTO sessions (id, project_id, started_at) VALUES (?1, ?2, ?3)",
            params![session_id, project_id, now],
        )?;

        Ok(session_id)
    }

    /// End a session
    pub fn end_session(
        &self,
        group_id: &str,
        session_id: &str,
        exit_status: &str,
    ) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        conn.execute(
            "UPDATE sessions SET ended_at = ?1, exit_status = ?2 WHERE id = ?3",
            params![now, exit_status, session_id],
        )?;

        Ok(())
    }

    /// Get all sessions for a project
    pub fn get_project_sessions(
        &self,
        group_id: &str,
        project_id: &str,
    ) -> Result<Vec<Session>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status 
             FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC",
        )?;

        let sessions = stmt.query_map(params![project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        })?;

        let result: Result<Vec<_>, _> = sessions.collect();
        result.map_err(|e| Error::DatabaseError(e.to_string()))
    }

    /// Get sessions with stats for a project
    pub fn get_project_sessions_with_stats(
        &self,
        group_id: &str,
        project_id: &str,
    ) -> Result<Vec<SessionWithStats>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT s.id, s.project_id, s.started_at, s.ended_at, s.exit_status,
                    COALESCE(l.log_count, 0), COALESCE(l.log_size, 0),
                    COALESCE(m.metric_count, 0)
             FROM sessions s
             LEFT JOIN (SELECT session_id, COUNT(*) as log_count, COALESCE(SUM(LENGTH(data)), 0) as log_size FROM logs GROUP BY session_id) l ON l.session_id = s.id
             LEFT JOIN (SELECT session_id, COUNT(*) as metric_count FROM metrics GROUP BY session_id) m ON m.session_id = s.id
             WHERE s.project_id = ?1
             ORDER BY s.started_at DESC"
        )?;

        let sessions = stmt.query_map(params![project_id], |row| {
            Ok(SessionWithStats {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
                log_count: row.get(5)?,
                log_size: row.get(6)?,
                metric_count: row.get(7)?,
            })
        })?;

        let result: Result<Vec<_>, _> = sessions.collect();
        result.map_err(|e| Error::DatabaseError(e.to_string()))
    }

    /// Get a single session
    pub fn get_session(&self, group_id: &str, session_id: &str) -> Result<Option<Session>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Get the last completed session for a project
    pub fn get_last_completed_session(
        &self,
        group_id: &str,
        project_id: &str,
    ) -> Result<Option<Session>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status 
             FROM sessions 
             WHERE project_id = ?1 AND ended_at IS NOT NULL 
             ORDER BY ended_at DESC LIMIT 1",
        )?;

        let result = stmt.query_row(params![project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    /// Delete a session and all its logs/metrics
    pub fn delete_session(&self, group_id: &str, session_id: &str) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;

        conn.execute(
            "DELETE FROM metrics WHERE session_id = ?1",
            params![session_id],
        )?;

        conn.execute(
            "DELETE FROM logs WHERE session_id = ?1",
            params![session_id],
        )?;

        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;

        Ok(())
    }

    /// Get the current (active) session for a project
    pub fn get_current_session_for_project(
        &self,
        group_id: &str,
        project_id: &str,
    ) -> Result<Option<String>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT id FROM sessions WHERE project_id = ?1 AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
        )?;

        let result: Result<String, _> = stmt.query_row(params![project_id], |row| row.get(0));

        match result {
            Ok(session_id) => Ok(Some(session_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    // ============================================================================
    // Log Operations
    // ============================================================================

    /// Insert a log entry
    pub fn insert_log(
        &self,
        group_id: &str,
        session_id: &str,
        stream: LogStream,
        data: &str,
    ) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let stream_str = match stream {
            LogStream::Stdout => "stdout",
            LogStream::Stderr => "stderr",
        };

        conn.execute(
            "INSERT INTO logs (session_id, stream, data, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, stream_str, data, now],
        )?;

        Ok(())
    }

    /// Get logs for a session as text
    pub fn get_session_logs(&self, group_id: &str, session_id: &str) -> Result<String, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT data FROM logs WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;

        let mut result = String::new();
        for data in rows.flatten() {
            result.push_str(&data);
        }

        Ok(result)
    }

    /// Get logs for the latest session of a project
    pub fn get_project_logs(&self, group_id: &str, project_id: &str) -> Result<String, Error> {
        let conn = self.get_connection(group_id)?;

        // Get latest session
        let session_id: Option<String> = conn
            .query_row(
                "SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1",
                params![project_id],
                |row| row.get(0),
            )
            .ok();

        match session_id {
            Some(sid) => self.get_session_logs(group_id, &sid),
            None => Ok(String::new()),
        }
    }

    /// Get recent logs (limited count) for the latest session
    pub fn get_recent_logs(
        &self,
        group_id: &str,
        project_id: &str,
        limit: u32,
    ) -> Result<String, Error> {
        let conn = self.get_connection(group_id)?;

        // Get latest session
        let session_id: Option<String> = conn
            .query_row(
                "SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1",
                params![project_id],
                |row| row.get(0),
            )
            .ok();

        match session_id {
            Some(sid) => {
                let mut stmt = conn.prepare(
                    "SELECT data FROM (
                        SELECT data, timestamp, id FROM logs 
                        WHERE session_id = ?1 
                        ORDER BY timestamp DESC, id DESC LIMIT ?2
                    ) sub 
                    ORDER BY timestamp ASC, id ASC",
                )?;

                let rows = stmt.query_map(params![sid, limit], |row| row.get::<_, String>(0))?;

                let mut result = String::new();
                for data in rows.flatten() {
                    result.push_str(&data);
                }
                Ok(result)
            }
            None => Ok(String::new()),
        }
    }

    /// Clear all logs for a project
    pub fn clear_project_logs(&self, group_id: &str, project_id: &str) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;

        conn.execute(
            "DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE project_id = ?1)",
            params![project_id],
        )?;

        Ok(())
    }

    // ============================================================================
    // Metric Operations
    // ============================================================================

    /// Insert a metric point
    pub fn insert_metric(
        &self,
        group_id: &str,
        session_id: &str,
        cpu_usage: f32,
        memory_usage: u64,
    ) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        conn.execute(
            "INSERT INTO metrics (session_id, cpu_usage, memory_usage, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, cpu_usage, memory_usage, now],
        )?;

        Ok(())
    }

    /// Get all metrics for a session
    pub fn get_session_metrics(
        &self,
        group_id: &str,
        session_id: &str,
    ) -> Result<Vec<MetricPoint>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1 ORDER BY timestamp ASC"
        )?;

        let metrics = stmt.query_map(params![session_id], |row| {
            Ok(MetricPoint {
                cpu_usage: row.get(0)?,
                memory_usage: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })?;

        let result: Result<Vec<_>, _> = metrics.collect();
        result.map_err(|e| Error::DatabaseError(e.to_string()))
    }

    /// Get the last metric for a session
    pub fn get_last_metric(
        &self,
        group_id: &str,
        session_id: &str,
    ) -> Result<Option<MetricPoint>, Error> {
        let conn = self.get_connection(group_id)?;
        let mut stmt = conn.prepare(
            "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT 1"
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            Ok(MetricPoint {
                cpu_usage: row.get(0)?,
                memory_usage: row.get(1)?,
                timestamp: row.get(2)?,
            })
        });

        match result {
            Ok(metric) => Ok(Some(metric)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    // ============================================================================
    // Maintenance Operations
    // ============================================================================

    /// Get storage statistics for a group
    pub fn get_storage_stats(&self, group_id: &str) -> Result<(u64, u64, u64, u64), Error> {
        let conn = self.get_connection(group_id)?;

        let session_count: u64 =
            conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

        let log_count: u64 = conn.query_row("SELECT COUNT(*) FROM logs", [], |row| row.get(0))?;

        let log_size: u64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM logs",
            [],
            |row| row.get(0),
        )?;

        let metric_count: u64 =
            conn.query_row("SELECT COUNT(*) FROM metrics", [], |row| row.get(0))?;

        Ok((session_count, log_count, log_size, metric_count))
    }

    /// Clean up old data (sessions older than specified days)
    pub fn cleanup_old_data(&self, group_id: &str, days_to_keep: u32) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - (days_to_keep as u64 * 24 * 60 * 60 * 1000);

        // Delete old sessions (cascade will delete logs and metrics)
        conn.execute(
            "DELETE FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?1",
            params![cutoff],
        )?;

        Ok(())
    }

    /// Clean up all data for a group
    pub fn cleanup_all_data(&self, group_id: &str) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;

        conn.execute("DELETE FROM metrics", [])?;
        conn.execute("DELETE FROM logs", [])?;
        conn.execute("DELETE FROM sessions", [])?;

        Ok(())
    }

    /// Vacuum the database to reclaim space
    pub fn vacuum(&self, group_id: &str) -> Result<(), Error> {
        let conn = self.get_connection(group_id)?;
        conn.execute("VACUUM", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let manager = GroupDbManager::new(temp_dir.path().to_path_buf());

        let project_id = "test-project";

        // Create session
        let session_id = manager.create_session("group1", project_id).unwrap();

        // Get session
        let session = manager.get_session("group1", &session_id).unwrap();
        assert!(session.is_some());

        // End session
        manager.end_session("group1", &session_id, "0").unwrap();

        // Verify ended
        let session = manager.get_session("group1", &session_id).unwrap().unwrap();
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn test_logs() {
        let temp_dir = TempDir::new().unwrap();
        let manager = GroupDbManager::new(temp_dir.path().to_path_buf());

        let project_id = "test-project";
        let session_id = manager.create_session("group1", project_id).unwrap();

        // Insert logs
        manager
            .insert_log("group1", &session_id, LogStream::Stdout, "Hello ")
            .unwrap();
        manager
            .insert_log("group1", &session_id, LogStream::Stdout, "World")
            .unwrap();

        // Get logs
        let logs = manager.get_session_logs("group1", &session_id).unwrap();
        assert_eq!(logs, "Hello World");
    }

    #[test]
    fn test_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let manager = GroupDbManager::new(temp_dir.path().to_path_buf());

        let project_id = "test-project";
        let session_id = manager.create_session("group1", project_id).unwrap();

        // Insert metrics
        manager
            .insert_metric("group1", &session_id, 10.5, 1024)
            .unwrap();
        manager
            .insert_metric("group1", &session_id, 20.5, 2048)
            .unwrap();

        // Get metrics
        let metrics = manager.get_session_metrics("group1", &session_id).unwrap();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].cpu_usage, 10.5);
        assert_eq!(metrics[1].memory_usage, 2048);
    }
}
