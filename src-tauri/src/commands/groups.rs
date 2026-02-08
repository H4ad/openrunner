use crate::error::Error;
use crate::models::Group;
use crate::state::AppState;
use crate::storage;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_groups(state: State<'_, Arc<AppState>>) -> Result<Vec<Group>, Error> {
    let config = state.config.lock().unwrap();
    Ok(config.groups.clone())
}

#[tauri::command]
pub fn create_group(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
    directory: String,
) -> Result<Group, Error> {
    let group = Group {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        directory,
        projects: Vec::new(),
        env_vars: HashMap::new(),
    };

    {
        let mut config = state.config.lock().unwrap();
        config.groups.push(group.clone());
        storage::save_config(&app_handle, &config)?;
    }

    Ok(group)
}

#[tauri::command]
pub fn rename_group(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    name: String,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();

    let group = config
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id))?;

    group.name = name;
    let updated = group.clone();
    storage::save_config(&app_handle, &config)?;

    Ok(updated)
}

#[tauri::command]
pub fn update_group_directory(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    directory: String,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();

    let group = config
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id))?;

    group.directory = directory;
    let updated = group.clone();
    storage::save_config(&app_handle, &config)?;

    Ok(updated)
}

#[tauri::command]
pub fn update_group_env_vars(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    env_vars: HashMap<String, String>,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();

    let group = config
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id))?;

    group.env_vars = env_vars;
    let updated = group.clone();
    storage::save_config(&app_handle, &config)?;

    Ok(updated)
}

#[tauri::command]
pub fn delete_group(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
) -> Result<(), Error> {
    let mut config = state.config.lock().unwrap();

    let initial_len = config.groups.len();
    config.groups.retain(|g| g.id != group_id);

    if config.groups.len() == initial_len {
        return Err(Error::GroupNotFound(group_id));
    }

    storage::save_config(&app_handle, &config)?;
    Ok(())
}
