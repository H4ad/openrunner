use crate::commands::types::{Error, Group, Project};
use crate::state::AppState;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn create_project(
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

    // Create project in database
    {
        let mut db = state.database.lock().unwrap();
        db.create_project(&group_id, &project)?;
    }

    // Add to local group
    group.projects.push(project.clone());

    // Sync to YAML if enabled
    yaml_config::add_project_to_yaml(&group, &project, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
