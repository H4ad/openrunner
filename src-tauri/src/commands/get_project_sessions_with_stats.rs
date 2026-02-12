use crate::commands::types::{Error, SessionWithStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_project_sessions_with_stats(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<SessionWithStats>, Error> {
    let db = state.database.lock().unwrap();
    db.get_project_sessions_with_stats(&project_id)
}
