use crate::error::Error;
use crate::models::ProcessInfo;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_all_statuses(state: State<'_, Arc<AppState>>) -> Result<Vec<ProcessInfo>, Error> {
    let infos = state.process_infos.lock().unwrap();
    Ok(infos.values().cloned().collect())
}
