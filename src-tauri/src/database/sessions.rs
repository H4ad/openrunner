use crate::error::Error;
use crate::models::{Session, SessionWithStats};
use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};

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
