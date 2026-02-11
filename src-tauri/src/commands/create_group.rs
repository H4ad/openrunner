use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn create_group(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
    directory: String,
    sync_enabled: Option<bool>,
) -> Result<Group, Error> {
    let dir_path = Path::new(&directory);

    // Check if openrunner.yaml or openrunner.yml exists
    if let Some(yaml_path) = yaml_config::find_yaml_file(dir_path) {
        // Parse YAML and create group from it
        let yaml_config = yaml_config::parse_yaml(&yaml_path).map_err(Error::YamlConfig)?;
        let mut group = yaml_config::yaml_to_group(&yaml_config, dir_path, &yaml_path);

        // Use provided name if different from YAML
        if name != group.name {
            group.name = name;
        }

        // Set sync_enabled - default to true if YAML exists
        group.sync_enabled = sync_enabled.unwrap_or(true);

        // Save to database
        {
            let config_db = state.config_db.lock().unwrap();
            config_db.create_group(&group)?;
        }

        // Start watching the YAML file
        {
            let watcher = state.yaml_watcher.lock().unwrap();
            let _ = watcher.watch_group(app_handle.clone(), &group);
        }

        Ok(group)
    } else if sync_enabled.unwrap_or(false) {
        // Create group with YAML sync enabled - create the YAML file
        let mut group = Group {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.clone(),
            directory: directory.clone(),
            projects: Vec::new(),
            env_vars: HashMap::new(),
            sync_file: None,
            sync_enabled: true,
        };

        // Create YAML file
        let yaml_path = dir_path.join("openrunner.yaml");
        yaml_config::write_yaml(&group, &yaml_path).map_err(Error::YamlConfig)?;
        group.sync_file = Some(yaml_path.to_string_lossy().to_string());

        // Save to database
        {
            let config_db = state.config_db.lock().unwrap();
            config_db.create_group(&group)?;
        }

        // Start watching the YAML file
        {
            let watcher = state.yaml_watcher.lock().unwrap();
            let _ = watcher.watch_group(app_handle.clone(), &group);
        }

        Ok(group)
    } else {
        // Create empty group without sync
        let group = Group {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            directory,
            projects: Vec::new(),
            env_vars: HashMap::new(),
            sync_file: None,
            sync_enabled: false,
        };

        // Save to database
        {
            let config_db = state.config_db.lock().unwrap();
            config_db.create_group(&group)?;
        }

        Ok(group)
    }
}
