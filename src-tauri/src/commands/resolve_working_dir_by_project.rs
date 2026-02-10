use crate::commands::utils::resolve_working_dir;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn resolve_working_dir_by_project(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<String, Error> {
    let config = state.config.lock().unwrap();
    for group in &config.groups {
        if let Some(project) = group.projects.iter().find(|p| p.id == project_id) {
            return Ok(resolve_working_dir(&group.directory, &project.cwd));
        }
    }
    Err(Error::ProjectNotFound(project_id))
}
