use crate::commands::types::{Error, MetricPoint};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_session_metrics(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<MetricPoint>, Error> {
    let db = state.database.lock().unwrap();
    db.get_session_metrics(&session_id)
}
