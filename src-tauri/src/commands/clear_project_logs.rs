use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn clear_project_logs(
    state: State<'_, Arc<AppState>>,
    _group_id: String,
    project_id: String,
) -> Result<(), Error> {
    // Note: group_id is kept for API compatibility but not needed with unified database

    // Clear from SQLite
    let db = state.database.lock().unwrap();
    let _ = db.clear_project_logs(&project_id);
    drop(db);

    // Also clear log file
    let log_path = state.log_file_path(&project_id);
    if log_path.exists() {
        std::fs::write(&log_path, b"")?;
    }
    Ok(())
}
