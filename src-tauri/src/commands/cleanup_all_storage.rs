use crate::commands::types::{Error, StorageStats};
use crate::database;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn cleanup_all_storage(state: State<'_, Arc<AppState>>) -> Result<StorageStats, Error> {
    let db = state.db.lock().unwrap();
    database::cleanup_all_data(&db)?;
    database::get_storage_stats(&db)
}
