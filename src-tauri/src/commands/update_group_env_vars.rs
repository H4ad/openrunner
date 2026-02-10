use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn update_group_env_vars(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    env_vars: HashMap<String, String>,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();
    let group_index = config
        .groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;

    config.groups[group_index].env_vars = env_vars;

    // Sync to YAML if enabled
    yaml_config::update_group_in_yaml(&config.groups[group_index], &state)
        .map_err(Error::YamlConfig)?;

    storage::save_config(&app_handle, &config)?;
    Ok(config.groups[group_index].clone())
}
