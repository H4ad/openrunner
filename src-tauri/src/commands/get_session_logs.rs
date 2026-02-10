use crate::database;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_session_logs(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<String, Error> {
    let db = state.db.lock().unwrap();
    database::get_session_logs_text(&db, &session_id)
}
