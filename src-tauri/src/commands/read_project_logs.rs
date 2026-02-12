use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn read_project_logs(
    state: State<'_, Arc<AppState>>,
    _group_id: String,
    project_id: String,
) -> Result<String, Error> {
    // Note: group_id is kept for API compatibility but not needed with unified database

    // Try SQLite first
    let db = state.database.lock().unwrap();
    let logs = db.get_project_logs(&project_id)?;
    if !logs.is_empty() {
        return Ok(logs);
    }
    drop(db);

    // Fallback to log file
    let log_path = state.log_file_path(&project_id);
    if log_path.exists() {
        Ok(std::fs::read_to_string(&log_path)?)
    } else {
        Ok(String::new())
    }
}
