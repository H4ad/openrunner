use crate::commands::types::{Error, Project};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_projects(
    state: State<'_, Arc<AppState>>,
    group_id: String,
) -> Result<Vec<Project>, Error> {
    let config = state.config.lock().unwrap();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        return Ok(group.projects.clone());
    }

    Err(Error::GroupNotFound(group_id))
}
