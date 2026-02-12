use crate::commands::types::{Error, StorageStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn cleanup_all_storage(state: State<'_, Arc<AppState>>) -> Result<StorageStats, Error> {
    let db = state.database.lock().unwrap();
    db.cleanup_all_data()?;
    db.vacuum()?;
    drop(db);

    // Return updated stats
    super::get_storage_stats::get_storage_stats(state)
}
