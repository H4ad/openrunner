use crate::database;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn delete_session(state: State<'_, Arc<AppState>>, session_id: String) -> Result<(), Error> {
    let db = state.db.lock().unwrap();
    database::delete_session(&db, &session_id)
}
