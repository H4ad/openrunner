use crate::database;
use crate::error::Error;
use crate::models::SessionWithStats;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_project_sessions_with_stats(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<SessionWithStats>, Error> {
    let db = state.db.lock().unwrap();
    database::get_project_sessions_with_stats(&db, &project_id)
}
