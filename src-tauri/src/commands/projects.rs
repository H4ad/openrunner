use crate::error::Error;
use crate::models::Project;
use crate::state::AppState;
use crate::storage;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn create_project(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    name: String,
    command: String,
    cwd: Option<String>,
) -> Result<Project, Error> {
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        command,
        auto_restart: false,
        env_vars: HashMap::new(),
        cwd,
    };

    let mut config = state.config.lock().unwrap();

    let group = config
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id))?;

    group.projects.push(project.clone());
    storage::save_config(&app_handle, &config)?;

    Ok(project)
}

#[tauri::command]
pub fn update_project(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
    name: Option<String>,
    command: Option<String>,
    auto_restart: Option<bool>,
    env_vars: Option<HashMap<String, String>>,
    cwd: Option<Option<String>>,
) -> Result<Project, Error> {
    let mut config = state.config.lock().unwrap();

    let group = config
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;

    let project = group
        .projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id))?;

    if let Some(name) = name {
        project.name = name;
    }
    if let Some(command) = command {
        project.command = command;
    }
    if let Some(auto_restart) = auto_restart {
        project.auto_restart = auto_restart;
    }
    if let Some(env_vars) = env_vars {
        project.env_vars = env_vars;
    }
    if let Some(cwd) = cwd {
        project.cwd = cwd;
    }

    let updated = project.clone();
    storage::save_config(&app_handle, &config)?;

    Ok(updated)
}

#[tauri::command]
pub fn delete_project(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<(), Error> {
    let mut config = state.config.lock().unwrap();

    let group = config
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id))?;

    let initial_len = group.projects.len();
    group.projects.retain(|p| p.id != project_id);

    if group.projects.len() == initial_len {
        return Err(Error::ProjectNotFound(project_id));
    }

    storage::save_config(&app_handle, &config)?;
    Ok(())
}
