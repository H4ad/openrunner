use crate::commands::types::{AppSettings, Error};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<AppSettings, Error> {
    let db = state.database.lock().unwrap();
    db.get_settings()
}
