use crate::commands::types::{Error, MetricPoint};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_last_metric(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Option<MetricPoint>, Error> {
    let db = state.database.lock().unwrap();
    db.get_last_metric(&session_id)
}
