use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn delete_multiple_projects(
    group_id: String,
    project_ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Delete projects from database
    {
        let mut db = state.database.lock().unwrap();
        db.delete_projects(&project_ids)?;
    }

    // Update local group
    group.projects.retain(|p| !project_ids.contains(&p.id));

    // Sync to YAML if enabled
    yaml_config::sync_yaml(&group, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
