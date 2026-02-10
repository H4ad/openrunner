use crate::database;
use crate::error::Error;
use crate::models::Session;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_project_sessions(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<Session>, Error> {
    let db = state.db.lock().unwrap();
    database::get_project_sessions(&db, &project_id)
}
