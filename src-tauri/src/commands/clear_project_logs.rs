use crate::database;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn clear_project_logs(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<(), Error> {
    // Clear from SQLite
    if let Ok(db) = state.db.lock() {
        let _ = database::clear_project_logs(&db, &project_id);
    }

    // Also clear log file
    let log_path = state.log_file_path(&project_id);
    if log_path.exists() {
        std::fs::write(&log_path, b"")?;
    }
    Ok(())
}
