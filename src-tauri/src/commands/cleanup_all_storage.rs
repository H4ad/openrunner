use crate::commands::types::{Error, StorageStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn cleanup_all_storage(state: State<'_, Arc<AppState>>) -> Result<StorageStats, Error> {
    let config_db = state.config_db.lock().unwrap();
    let groups = config_db.get_groups()?;

    for group in groups {
        if let Ok(conn) = state.group_db_manager.get_connection(&group.id) {
            // Delete all data
            let _ = conn.execute_batch(
                "DELETE FROM metrics; DELETE FROM logs; DELETE FROM sessions; VACUUM;",
            );
        }
    }

    // Return updated stats
    drop(config_db);
    super::get_storage_stats::get_storage_stats(state)
}
