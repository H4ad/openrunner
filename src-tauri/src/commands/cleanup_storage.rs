use crate::commands::types::{Error, StorageStats};
use crate::state::AppState;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[tauri::command]
pub fn cleanup_storage(state: State<'_, Arc<AppState>>, days: u32) -> Result<StorageStats, Error> {
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
        - (days as u64 * 24 * 60 * 60 * 1000);

    let config_db = state.config_db.lock().unwrap();
    let groups = config_db.get_groups()?;

    for group in groups {
        if let Ok(conn) = state.group_db_manager.get_connection(&group.id) {
            // Delete old metrics
            let _ = conn.execute(
                "DELETE FROM metrics WHERE session_id IN (SELECT id FROM sessions WHERE started_at < ?1)",
                [cutoff],
            );

            // Delete old logs
            let _ = conn.execute(
                "DELETE FROM logs WHERE session_id IN (SELECT id FROM sessions WHERE started_at < ?1)",
                [cutoff],
            );

            // Delete old sessions
            let _ = conn.execute("DELETE FROM sessions WHERE started_at < ?1", [cutoff]);

            // Vacuum to reclaim space
            let _ = conn.execute_batch("VACUUM;");
        }
    }

    // Return updated stats
    drop(config_db);
    super::get_storage_stats::get_storage_stats(state)
}
