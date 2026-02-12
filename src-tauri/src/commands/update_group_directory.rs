use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn update_group_directory(
    group_id: String,
    directory: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Update directory in database
    {
        let db = state.database.lock().unwrap();
        db.update_group_directory(&group_id, &directory)?;
    }

    // Update local group
    group.directory = directory.clone();

    // If sync is enabled, we need to update the sync_file path to point to new directory
    if group.sync_enabled {
        if let Some(ref old_sync_file) = group.sync_file {
            let old_path = Path::new(old_sync_file);
            let file_name = old_path
                .file_name()
                .ok_or(Error::YamlConfig("Invalid sync file path".to_string()))?;
            let new_sync_file = Path::new(&directory).join(file_name);
            group.sync_file = Some(new_sync_file.to_string_lossy().to_string());
        }
    }

    // Sync to YAML if enabled
    yaml_config::update_group_in_yaml(&group, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
