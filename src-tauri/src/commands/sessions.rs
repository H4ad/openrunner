use crate::database;
use crate::error::Error;
use crate::models::{MetricPoint, Session, SessionWithStats};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_project_sessions(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<Session>, Error> {
    let db = state.db.lock().unwrap();
    database::get_project_sessions(&db, &project_id)
}

#[tauri::command]
pub fn get_project_sessions_with_stats(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<SessionWithStats>, Error> {
    let db = state.db.lock().unwrap();
    database::get_project_sessions_with_stats(&db, &project_id)
}

#[tauri::command]
pub fn get_session(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Option<Session>, Error> {
    let db = state.db.lock().unwrap();
    database::get_session(&db, &session_id)
}

#[tauri::command]
pub fn get_session_logs(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<String, Error> {
    let db = state.db.lock().unwrap();
    database::get_session_logs_text(&db, &session_id)
}

#[tauri::command]
pub fn get_session_metrics(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<MetricPoint>, Error> {
    let db = state.db.lock().unwrap();
    database::get_session_metrics(&db, &session_id)
}

#[tauri::command]
pub fn delete_session(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<(), Error> {
    let db = state.db.lock().unwrap();
    database::delete_session(&db, &session_id)
}

#[tauri::command]
pub fn get_last_completed_session(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Option<Session>, Error> {
    let db = state.db.lock().unwrap();
    database::get_last_completed_session(&db, &project_id)
}

#[tauri::command]
pub fn get_recent_logs(
    state: State<'_, Arc<AppState>>,
    project_id: String,
    limit: u32,
) -> Result<String, Error> {
    let db = state.db.lock().unwrap();
    database::get_recent_logs(&db, &project_id, limit)
}

#[tauri::command]
pub fn get_last_metric(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Option<MetricPoint>, Error> {
    let db = state.db.lock().unwrap();
    database::get_last_metric(&db, &session_id)
}
