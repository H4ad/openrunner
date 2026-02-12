use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn write_to_process_stdin(
    state: State<'_, Arc<AppState>>,
    project_id: String,
    data: String,
) -> Result<(), Error> {
    crate::process::io::write_to_process_stdin(&state, &project_id, &data)
}
