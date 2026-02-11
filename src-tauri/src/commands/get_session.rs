use crate::error::Error;
use crate::models::Session;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_session(
    state: State<'_, Arc<AppState>>,
    _group_id: String,
    session_id: String,
) -> Result<Option<Session>, Error> {
    // Note: group_id is kept for API compatibility but not needed with unified database
    let db = state.database.lock().unwrap();
    db.get_session(&session_id)
}
