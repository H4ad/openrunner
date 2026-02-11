use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

/// Get the database path for the unified database
#[tauri::command]
pub fn get_database_path(state: State<'_, Arc<AppState>>) -> String {
    state.database_path.to_string_lossy().to_string()
}
