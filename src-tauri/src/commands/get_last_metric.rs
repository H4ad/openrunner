use crate::database;
use crate::error::Error;
use crate::models::MetricPoint;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_last_metric(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Option<MetricPoint>, Error> {
    let db = state.db.lock().unwrap();
    database::get_last_metric(&db, &session_id)
}
