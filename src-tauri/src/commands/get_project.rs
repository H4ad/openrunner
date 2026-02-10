use crate::commands::types::{Error, Project};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_project(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<Project, Error> {
    let config = state.config.lock().unwrap();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        if let Some(project) = group.projects.iter().find(|p| p.id == project_id) {
            return Ok(project.clone());
        }
    }

    Err(Error::ProjectNotFound(project_id))
}
