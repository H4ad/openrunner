use crate::database;
use crate::error::Error;
use crate::models::Session;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_last_completed_session(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Option<Session>, Error> {
    let db = state.db.lock().unwrap();
    database::get_last_completed_session(&db, &project_id)
}
