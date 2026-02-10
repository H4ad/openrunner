use crate::commands::types::Error;
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn update_project_env_vars(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
    env_vars: HashMap<String, String>,
) -> Result<(), Error> {
    let mut sync_file_path = None;
    let mut sync_enabled = false;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            sync_file_path = group.sync_file.clone();
            sync_enabled = group.sync_enabled;
            if let Some(project) = group.projects.iter_mut().find(|p| p.id == project_id) {
                project.env_vars = env_vars;
            }
        }

        storage::save_config(&app_handle, &config)?;
    }

    // Sync to YAML after releasing locks (only if sync is enabled)
    if sync_enabled {
        if let Some(yaml_path) = sync_file_path {
            let config = state.config.lock().unwrap();
            if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                let _ = yaml_config::write_yaml(group, std::path::Path::new(&yaml_path));
            }
        }
    }

    Ok(())
}
