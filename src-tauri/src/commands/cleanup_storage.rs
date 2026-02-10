use crate::commands::types::{Error, StorageStats};
use crate::database;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn cleanup_storage(state: State<'_, Arc<AppState>>, days: u32) -> Result<StorageStats, Error> {
    let db = state.db.lock().unwrap();
    database::cleanup_old_data(&db, days)?;
    database::get_storage_stats(&db)
}
