use crate::commands::types::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_session_logs(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<String, Error> {
    let db = state.database.lock().unwrap();
    db.get_session_logs(&session_id)
}
