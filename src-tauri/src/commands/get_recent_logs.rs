use crate::commands::types::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_recent_logs(
    state: State<'_, Arc<AppState>>,
    project_id: String,
    limit: u32,
) -> Result<String, Error> {
    let db = state.database.lock().unwrap();
    db.get_recent_logs(&project_id, limit)
}
