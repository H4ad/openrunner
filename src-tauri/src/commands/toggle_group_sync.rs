use crate::commands::types::{Error, Group, Project};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn toggle_group_sync(
    group_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
) -> Result<Group, Error> {
    let sync_file: Option<String>;
    let sync_enabled: bool;
    let group_name: String;
    let group_dir: String;
    let group_projects: Vec<Project>;
    let group_env_vars: HashMap<String, String>;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            group.sync_enabled = !group.sync_enabled;
            sync_enabled = group.sync_enabled;
            sync_file = group.sync_file.clone();
            group_name = group.name.clone();
            group_dir = group.directory.clone();
            group_projects = group.projects.clone();
            group_env_vars = group.env_vars.clone();
        } else {
            return Err(Error::GroupNotFound(group_id));
        }
    }

    // If sync is being enabled but no sync file exists, create it
    if sync_enabled {
        if sync_file.is_none() {
            let yaml_path = PathBuf::from(&group_dir).join("openrunner.yaml");
            let group_clone = Group {
                id: group_id.clone(),
                name: group_name,
                directory: group_dir,
                projects: group_projects,
                env_vars: group_env_vars,
                sync_file: Some(yaml_path.to_string_lossy().to_string()),
                sync_enabled: true,
            };

            // Create the YAML file
            if let Err(e) = yaml_config::write_yaml(&group_clone, &yaml_path) {
                // Revert sync_enabled if file creation failed
                let mut config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                    group.sync_enabled = false;
                }
                return Err(Error::YamlConfig(e));
            }

            // Update the group with the new sync file path
            let mut config = state.config.lock().unwrap();
            if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                group.sync_file = Some(yaml_path.to_string_lossy().to_string());
            }

            // Start watching the new YAML file
            if let Ok(watcher) = state.yaml_watcher.lock() {
                // Get the updated group to pass to watch_group
                let config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                    let _ = watcher.watch_group(app_handle.clone(), group);
                }
            }
        } else {
            // Sync file exists, just update it and start watching
            {
                let mut config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                    let _ = yaml_config::sync_yaml(group, &state);
                }
            }

            // Start watching the YAML file
            if let Ok(watcher) = state.yaml_watcher.lock() {
                let config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                    let _ = watcher.watch_group(app_handle.clone(), group);
                }
            }
        }
    } else {
        // Sync is being disabled - stop watching (but keep sync_file path for re-enabling later)
        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.unwatch_group(&group_id);
        }
    }

    // Save config
    let config = state.config.lock().unwrap();
    storage::save_config(&app_handle, &config)?;
    config
        .groups
        .iter()
        .find(|g| g.id == group_id)
        .cloned()
        .ok_or(Error::GroupNotFound(group_id))
}
