use crate::commands::types::{AppSettings, Error};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn update_settings(
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> Result<(), Error> {
    let db = state.database.lock().unwrap();
    db.update_settings(&settings)
}
