use crate::commands::types::{Error, StorageStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_storage_stats(state: State<'_, Arc<AppState>>) -> Result<StorageStats, Error> {
    let config_db = state.config_db.lock().unwrap();
    let groups = config_db.get_groups()?;

    let mut total_stats = StorageStats {
        total_size: 0,
        log_count: 0,
        metric_count: 0,
        session_count: 0,
    };

    for group in groups {
        if let Ok(conn) = state.group_db_manager.get_connection(&group.id) {
            // Get log count
            if let Ok(count) =
                conn.query_row("SELECT COUNT(*) FROM logs", [], |row| row.get::<_, u64>(0))
            {
                total_stats.log_count += count;
            }

            // Get metric count
            if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM metrics", [], |row| {
                row.get::<_, u64>(0)
            }) {
                total_stats.metric_count += count;
            }

            // Get session count
            if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| {
                row.get::<_, u64>(0)
            }) {
                total_stats.session_count += count;
            }

            // Get database size
            if let Ok(page_count) =
                conn.query_row("PRAGMA page_count", [], |row| row.get::<_, u64>(0))
            {
                if let Ok(page_size) =
                    conn.query_row("PRAGMA page_size", [], |row| row.get::<_, u64>(0))
                {
                    total_stats.total_size += page_count * page_size;
                }
            }
        }
    }

    Ok(total_stats)
}
