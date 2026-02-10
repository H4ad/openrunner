use crate::database;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn read_project_logs(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<String, Error> {
    // Try SQLite first
    if let Ok(db) = state.db.lock() {
        let logs = database::get_project_logs(&db, &project_id)?;
        if !logs.is_empty() {
            return Ok(logs);
        }
    }

    // Fallback to log file
    let log_path = state.log_file_path(&project_id);
    if log_path.exists() {
        Ok(std::fs::read_to_string(&log_path)?)
    } else {
        Ok(String::new())
    }
}
