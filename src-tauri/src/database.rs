use crate::error::Error;
use crate::models::{MetricPoint, Session, SessionWithStats, StorageStats};
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn init_database(path: &Path) -> Result<Connection, Error> {
    let conn =
        Connection::open(path).map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            exit_status TEXT
        );

        CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            stream TEXT NOT NULL,
            data TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE TABLE IF NOT EXISTS metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            cpu_usage REAL NOT NULL,
            memory_usage INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE INDEX IF NOT EXISTS idx_logs_session ON logs(session_id);
        CREATE INDEX IF NOT EXISTS idx_metrics_session ON metrics(session_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);
        ",
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    // Enable WAL mode for better concurrent read/write performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(conn)
}

pub fn create_session(conn: &Connection, project_id: &str) -> Result<String, Error> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    conn.execute(
        "INSERT INTO sessions (id, project_id, started_at) VALUES (?1, ?2, ?3)",
        params![session_id, project_id, now],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(session_id)
}

pub fn end_session(conn: &Connection, session_id: &str, exit_status: &str) -> Result<(), Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    conn.execute(
        "UPDATE sessions SET ended_at = ?1, exit_status = ?2 WHERE id = ?3",
        params![now, exit_status, session_id],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn insert_log(
    conn: &Connection,
    session_id: &str,
    stream: &str,
    data: &str,
    timestamp: u64,
) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO logs (session_id, stream, data, timestamp) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, stream, data, timestamp],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn insert_metric(
    conn: &Connection,
    session_id: &str,
    cpu_usage: f32,
    memory_usage: u64,
    timestamp: u64,
) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO metrics (session_id, cpu_usage, memory_usage, timestamp) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, cpu_usage, memory_usage, timestamp],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn get_current_session_for_project(
    conn: &Connection,
    project_id: &str,
) -> Result<Option<String>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id FROM sessions WHERE project_id = ?1 AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let result = stmt
        .query_row(params![project_id], |row| row.get::<_, String>(0))
        .ok();

    Ok(result)
}

pub fn get_project_logs(conn: &Connection, project_id: &str) -> Result<String, Error> {
    // Get the latest session for this project
    let mut stmt = conn
        .prepare(
            "SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let session_id: Option<String> = stmt
        .query_row(params![project_id], |row| row.get(0))
        .ok();

    match session_id {
        Some(sid) => get_session_logs_text(conn, &sid),
        None => Ok(String::new()),
    }
}

pub fn get_session_logs_text(conn: &Connection, session_id: &str) -> Result<String, Error> {
    let mut stmt = conn
        .prepare("SELECT data FROM logs WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![session_id], |row| row.get::<_, String>(0))
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let mut result = String::new();
    for row in rows {
        if let Ok(data) = row {
            result.push_str(&data);
        }
    }

    Ok(result)
}

pub fn clear_project_logs(conn: &Connection, project_id: &str) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE project_id = ?1)",
        params![project_id],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn get_project_sessions(conn: &Connection, project_id: &str) -> Result<Vec<Session>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let sessions = stmt
        .query_map(params![project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        })
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

pub fn get_project_sessions_with_stats(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<SessionWithStats>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.project_id, s.started_at, s.ended_at, s.exit_status,
                    COALESCE(l.log_count, 0), COALESCE(l.log_size, 0),
                    COALESCE(m.metric_count, 0)
             FROM sessions s
             LEFT JOIN (SELECT session_id, COUNT(*) as log_count, COALESCE(SUM(LENGTH(data)), 0) as log_size FROM logs GROUP BY session_id) l ON l.session_id = s.id
             LEFT JOIN (SELECT session_id, COUNT(*) as metric_count FROM metrics GROUP BY session_id) m ON m.session_id = s.id
             WHERE s.project_id = ?1
             ORDER BY s.started_at DESC",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let sessions = stmt
        .query_map(params![project_id], |row| {
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
        })
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

pub fn get_session(conn: &Connection, session_id: &str) -> Result<Option<Session>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE id = ?1",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let result = stmt
        .query_row(params![session_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        })
        .ok();

    Ok(result)
}

pub fn get_session_metrics(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<MetricPoint>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1 ORDER BY timestamp ASC",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let metrics = stmt
        .query_map(params![session_id], |row| {
            Ok(MetricPoint {
                cpu_usage: row.get(0)?,
                memory_usage: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(metrics)
}

pub fn delete_session(conn: &Connection, session_id: &str) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM metrics WHERE session_id = ?1",
        params![session_id],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute(
        "DELETE FROM logs WHERE session_id = ?1",
        params![session_id],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn get_storage_stats(conn: &Connection) -> Result<StorageStats, Error> {
    let log_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM logs", [], |row| row.get(0))
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let metric_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM metrics", [], |row| row.get(0))
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let session_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    // Estimate size from page_count * page_size
    let page_count: u64 = conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let page_size: u64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(StorageStats {
        total_size: page_count * page_size,
        log_count,
        metric_count,
        session_count,
    })
}

pub fn cleanup_old_data(conn: &Connection, days: u32) -> Result<(), Error> {
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
        - (days as u64 * 24 * 60 * 60 * 1000);

    conn.execute(
        "DELETE FROM metrics WHERE session_id IN (SELECT id FROM sessions WHERE started_at < ?1)",
        params![cutoff],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute(
        "DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE started_at < ?1)",
        params![cutoff],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    conn.execute(
        "DELETE FROM sessions WHERE started_at < ?1",
        params![cutoff],
    )
    .map_err(|e| Error::DatabaseError(e.to_string()))?;

    // Reclaim space
    conn.execute_batch("VACUUM;")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn get_last_completed_session(
    conn: &Connection,
    project_id: &str,
) -> Result<Option<Session>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, started_at, ended_at, exit_status FROM sessions WHERE project_id = ?1 AND ended_at IS NOT NULL ORDER BY ended_at DESC LIMIT 1",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let result = stmt
        .query_row(params![project_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                exit_status: row.get(4)?,
            })
        })
        .ok();

    Ok(result)
}

pub fn get_recent_logs(
    conn: &Connection,
    project_id: &str,
    limit: u32,
) -> Result<String, Error> {
    // Get the latest session for this project
    let mut stmt = conn
        .prepare(
            "SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let session_id: Option<String> = stmt
        .query_row(params![project_id], |row| row.get(0))
        .ok();

    match session_id {
        Some(sid) => {
            let mut stmt = conn
                .prepare(
                    "SELECT data FROM (SELECT data, timestamp, id FROM logs WHERE session_id = ?1 ORDER BY timestamp DESC, id DESC LIMIT ?2) sub ORDER BY timestamp ASC, id ASC",
                )
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            let rows = stmt
                .query_map(params![sid, limit], |row| row.get::<_, String>(0))
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            let mut result = String::new();
            for row in rows {
                if let Ok(data) = row {
                    result.push_str(&data);
                }
            }
            Ok(result)
        }
        None => Ok(String::new()),
    }
}

pub fn get_last_metric(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<MetricPoint>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT 1",
        )
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let result = stmt
        .query_row(params![session_id], |row| {
            Ok(MetricPoint {
                cpu_usage: row.get(0)?,
                memory_usage: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })
        .ok();

    Ok(result)
}

pub fn cleanup_all_data(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch("DELETE FROM metrics; DELETE FROM logs; DELETE FROM sessions; VACUUM;")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}
