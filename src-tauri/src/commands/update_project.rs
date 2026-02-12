use crate::commands::types::{Error, Group, Project};
use crate::state::AppState;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn update_project(
    group_id: String,
    project: Project,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Update project in database
    {
        let mut db = state.database.lock().unwrap();
        db.update_project(&project)?;
    }

    // Update local group
    if let Some(idx) = group.projects.iter().position(|p| p.id == project.id) {
        group.projects[idx] = project.clone();
    }

    // Sync to YAML if enabled
    yaml_config::update_project_in_yaml(&group, &project, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
