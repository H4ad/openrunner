use crate::commands::types::{Error, StorageStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn cleanup_storage(state: State<'_, Arc<AppState>>, days: u32) -> Result<StorageStats, Error> {
    let db = state.database.lock().unwrap();
    db.cleanup_old_data(days)?;
    db.vacuum()?;
    drop(db);

    // Return updated stats
    super::get_storage_stats::get_storage_stats(state)
}
