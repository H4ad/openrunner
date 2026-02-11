use crate::commands::types::{Error, Group, ProjectType};
use crate::state::AppState;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn convert_multiple_projects(
    group_id: String,
    project_ids: Vec<String>,
    new_type: ProjectType,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Build conversions list
    let conversions: Vec<(String, ProjectType)> = project_ids
        .iter()
        .map(|id| (id.clone(), new_type))
        .collect();

    // Update projects in database
    {
        let mut db = state.database.lock().unwrap();
        db.convert_projects(&conversions)?;
    }

    // Update local group
    for project_id in &project_ids {
        if let Some(idx) = group.projects.iter().position(|p| p.id == *project_id) {
            group.projects[idx].project_type = new_type;
            // Only update auto_restart when converting to Task (must be false)
            // When converting to Service, preserve existing setting (user may have disabled it)
            if new_type == ProjectType::Task {
                group.projects[idx].auto_restart = false;
            }
            // When converting Task -> Service, keep existing auto_restart (defaults to true on creation)
        }
    }

    // Sync to YAML if enabled
    yaml_config::sync_yaml(&group, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
