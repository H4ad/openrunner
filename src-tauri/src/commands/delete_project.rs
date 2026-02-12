use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn delete_project(
    group_id: String,
    project_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Delete project from database
    {
        let db = state.database.lock().unwrap();
        db.delete_project(&project_id)?;
    }

    // Update local group
    group.projects.retain(|p| p.id != project_id);

    // Sync to YAML if enabled
    yaml_config::remove_project_from_yaml(&group, &project_id, &state)
        .map_err(Error::YamlConfig)?;

    Ok(group)
}
