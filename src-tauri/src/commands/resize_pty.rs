use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn resize_pty(
    state: State<'_, Arc<AppState>>,
    project_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), Error> {
    crate::process::io::resize_pty(&state, &project_id, cols, rows)
}
