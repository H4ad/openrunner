use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::yaml_config;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn update_group_env_vars(
    group_id: String,
    env_vars: HashMap<String, String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Get current group
    let mut group = {
        let db = state.database.lock().unwrap();
        db.get_group(&group_id)?
            .ok_or(Error::GroupNotFound(group_id.clone()))?
    };

    // Update env vars in database
    {
        let mut db = state.database.lock().unwrap();
        db.update_group_env_vars(&group_id, &env_vars)?;
    }

    // Update local group
    group.env_vars = env_vars;

    // Sync to YAML if enabled
    yaml_config::update_group_in_yaml(&group, &state).map_err(Error::YamlConfig)?;

    Ok(group)
}
