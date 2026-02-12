use crate::error::Error;
use crate::models::StorageStats;
use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn cleanup_all_data(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch("DELETE FROM metrics; DELETE FROM logs; DELETE FROM sessions; VACUUM;")
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}
