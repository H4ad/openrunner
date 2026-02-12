use crate::error::Error;
use crate::models::MetricPoint;
use rusqlite::{params, Connection};

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

pub fn get_session_metrics(conn: &Connection, session_id: &str) -> Result<Vec<MetricPoint>, Error> {
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

pub fn get_last_metric(conn: &Connection, session_id: &str) -> Result<Option<MetricPoint>, Error> {
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
