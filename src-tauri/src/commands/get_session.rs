use crate::database;
use crate::error::Error;
use crate::models::Session;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_session(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    session_id: String,
) -> Result<Option<Session>, Error> {
    let conn = state.group_db_manager.get_connection(&group_id)?;
    database::get_session(&conn, &session_id)
}
