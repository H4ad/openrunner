use crate::commands::utils::resolve_working_dir;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn resolve_project_working_dir(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<String, Error> {
    let db = state.database.lock().unwrap();
    let group = db
        .get_group(&group_id)?
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;
    let project = group
        .projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id.clone()))?;
    Ok(resolve_working_dir(&group.directory, &project.cwd))
}
