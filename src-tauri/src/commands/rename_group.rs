use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn rename_group(
    group_id: String,
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Update name in database
    {
        let db = state.database.lock().unwrap();
        db.rename_group(&group_id, &name)?;
    }

    // Update local group
    group.name = name;

    // Sync to YAML if enabled
    yaml_config::update_group_in_yaml(&group, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
