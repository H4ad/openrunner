use crate::commands::types::{Error, Group};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_groups(state: State<'_, Arc<AppState>>) -> Result<Vec<Group>, Error> {
    let db = state.database.lock().unwrap();
    db.get_groups()
}
