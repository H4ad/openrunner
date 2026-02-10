use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn update_group_directory(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    directory: String,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();
    let group_index = config
        .groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;

    let old_sync_file = config.groups[group_index].sync_file.clone();
    config.groups[group_index].directory = directory;

    // Check if new directory has YAML file
    let dir_path = Path::new(&config.groups[group_index].directory);
    if let Some(yaml_path) = yaml_config::find_yaml_file(dir_path) {
        config.groups[group_index].sync_file = Some(yaml_path.to_string_lossy().to_string());

        // Update watcher
        {
            let watcher = state.yaml_watcher.lock().unwrap();
            if old_sync_file.is_some() {
                let _ = watcher.unwatch_group(&group_id);
            }
            let _ = watcher.watch_group(app_handle.clone(), &config.groups[group_index]);
        }
    } else {
        config.groups[group_index].sync_file = None;

        // Remove watcher if was watching
        if old_sync_file.is_some() {
            let watcher = state.yaml_watcher.lock().unwrap();
            let _ = watcher.unwatch_group(&group_id);
        }
    }

    storage::save_config(&app_handle, &config)?;
    Ok(config.groups[group_index].clone())
}
