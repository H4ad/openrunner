use crate::commands::types::{Error, Group, ProjectType};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn convert_multiple_projects(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_ids: Vec<String>,
    new_type: ProjectType,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();
    let group_index = config
        .groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;

    for project in config.groups[group_index].projects.iter_mut() {
        if project_ids.contains(&project.id) {
            project.project_type = new_type.clone();

            // Update auto_restart based on type
            project.auto_restart = new_type == ProjectType::Service;
        }
    }

    storage::save_config(&app_handle, &config)?;

    // Sync to YAML after saving (only if sync is enabled)
    if config.groups[group_index].sync_enabled {
        if let Some(ref yaml_path) = config.groups[group_index].sync_file {
            let _ = yaml_config::write_yaml(
                &config.groups[group_index],
                std::path::Path::new(yaml_path),
            );
        }
    }

    Ok(config.groups[group_index].clone())
}
