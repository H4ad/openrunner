use crate::error::Error;
use rusqlite::Connection;
use std::path::Path;

pub fn init_database(path: &Path) -> Result<Connection, Error> {
    let conn = Connection::open(path).map_err(|e| Error::DatabaseError(e.to_string()))?;

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
