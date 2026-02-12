use crate::commands::types::{Error, Session};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_last_completed_session(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Option<Session>, Error> {
    let db = state.database.lock().unwrap();
    db.get_last_completed_session(&project_id)
}
