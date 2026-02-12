use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn stop_process(state: State<'_, Arc<AppState>>, project_id: String) -> Result<(), Error> {
    let app_state: &AppState = &state;
    crate::process::lifecycle::stop_process(app_state, &project_id)
}
