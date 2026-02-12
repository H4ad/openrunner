use crate::commands::types::{Error, StorageStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_storage_stats(state: State<'_, Arc<AppState>>) -> Result<StorageStats, Error> {
    let db = state.database.lock().unwrap();

    let (session_count, log_count, _log_size, metric_count) = db.get_storage_stats()?;

    // Get database file size
    let total_size = std::fs::metadata(&state.database_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(StorageStats {
        total_size,
        log_count,
        metric_count,
        session_count,
    })
}
