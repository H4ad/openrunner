use crate::error::Error;
use rusqlite::{params, Connection};

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

pub fn get_project_logs(conn: &Connection, project_id: &str) -> Result<String, Error> {
    // Get the latest session for this project
    let mut stmt = conn
        .prepare("SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let session_id: Option<String> = stmt.query_row(params![project_id], |row| row.get(0)).ok();

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

pub fn get_recent_logs(conn: &Connection, project_id: &str, limit: u32) -> Result<String, Error> {
    // Get the latest session for this project
    let mut stmt = conn
        .prepare("SELECT id FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT 1")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let session_id: Option<String> = stmt.query_row(params![project_id], |row| row.get(0)).ok();

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
