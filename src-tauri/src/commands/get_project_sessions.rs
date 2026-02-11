use crate::commands::types::{Error, Session};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_project_sessions(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<Session>, Error> {
    let db = state.database.lock().unwrap();
    db.get_project_sessions(&project_id)
}
