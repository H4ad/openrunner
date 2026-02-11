use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn toggle_group_sync(
    group_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
) -> Result<Group, Error> {
    // Get group from SQLite database
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Toggle sync_enabled
    group.sync_enabled = !group.sync_enabled;

    // If sync is being enabled but no sync file exists, create it
    if group.sync_enabled {
        if group.sync_file.is_none() {
            let yaml_path = PathBuf::from(&group.directory).join("openrunner.yaml");
            let group_clone = Group {
                id: group_id.clone(),
                name: group.name.clone(),
                directory: group.directory.clone(),
                projects: group.projects.clone(),
                env_vars: group.env_vars.clone(),
                sync_file: Some(yaml_path.to_string_lossy().to_string()),
                sync_enabled: true,
            };

            // Create the YAML file
            if let Err(e) = yaml_config::write_yaml(&group_clone, &yaml_path) {
                // Revert sync_enabled if file creation failed
                group.sync_enabled = false;
                return Err(Error::YamlConfig(e));
            }

            // Update the group with the new sync file path
            group.sync_file = Some(yaml_path.to_string_lossy().to_string());

            // Start watching the new YAML file
            if let Ok(watcher) = state.yaml_watcher.lock() {
                let _ = watcher.watch_group(app_handle.clone(), &group);
            }
        } else {
            // Sync file exists, just update it and start watching
            {
                let _ = yaml_config::sync_yaml(&group, &state);
            }

            // Start watching the YAML file
            if let Ok(watcher) = state.yaml_watcher.lock() {
                let _ = watcher.watch_group(app_handle.clone(), &group);
            }
        }
    } else {
        // Sync is being disabled - stop watching (but keep sync_file path for re-enabling later)
        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.unwatch_group(&group_id);
        }
    }

    // Update the group in the database
    let db = state.database.lock().unwrap();
    db.update_group_sync(&group_id, group.sync_file.as_deref(), group.sync_enabled)?;

    Ok(group)
}
