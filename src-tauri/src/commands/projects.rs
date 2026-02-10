use crate::error::Error;
use crate::models::{Project, ProjectType};
use crate::process_manager;
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn create_project(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    name: String,
    command: String,
    cwd: Option<String>,
    project_type: ProjectType,
    interactive: bool,
) -> Result<Project, Error> {
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        command,
        auto_restart: project_type == ProjectType::Service,
        env_vars: std::collections::HashMap::new(),
        cwd,
        project_type,
        interactive,
    };

    let mut group_updated = false;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            // Add to group first
            group.projects.push(project.clone());
            group_updated = true;

            // Then sync to YAML (now includes the new project)
            if let Err(e) = yaml_config::sync_yaml(group, &state) {
                eprintln!("Failed to sync project to YAML: {}", e);
            }
        }

        if group_updated {
            storage::save_config(&app_handle, &config)?;
        }
    }

    if !group_updated {
        return Err(Error::GroupNotFound(group_id));
    }

    Ok(project)
}

#[tauri::command]
pub fn update_project(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
    updates: std::collections::HashMap<String, serde_json::Value>,
) -> Result<Project, Error> {
    let mut updated_project = None;
    let mut sync_file_path = None;
    let mut sync_enabled = false;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            // Check if sync is enabled and store the path
            sync_file_path = group.sync_file.clone();
            sync_enabled = group.sync_enabled;

            if let Some(project) = group.projects.iter_mut().find(|p| p.id == project_id) {
                // Apply updates
                if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
                    project.name = name.to_string();
                }
                if let Some(command) = updates.get("command").and_then(|v| v.as_str()) {
                    project.command = command.to_string();
                }
                if let Some(auto_restart) = updates.get("autoRestart").and_then(|v| v.as_bool()) {
                    project.auto_restart = auto_restart;
                }
                if let Some(cwd) = updates.get("cwd") {
                    project.cwd = cwd.as_str().map(|s| s.to_string());
                }
                if let Some(env_vars) = updates.get("envVars") {
                    if let Ok(vars) = serde_json::from_value::<
                        std::collections::HashMap<String, String>,
                    >(env_vars.clone())
                    {
                        project.env_vars = vars;
                    }
                }
                if let Some(interactive) = updates.get("interactive").and_then(|v| v.as_bool()) {
                    project.interactive = interactive;
                }

                updated_project = Some(project.clone());
            }
        }

        if updated_project.is_some() {
            storage::save_config(&app_handle, &config)?;
        }
    }

    // Sync to YAML after releasing locks - sync entire group (only if sync is enabled)
    if sync_enabled {
        if let Some(yaml_path) = sync_file_path {
            let config = state.config.lock().unwrap();
            if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                let _ = yaml_config::write_yaml(group, std::path::Path::new(&yaml_path));
            }
        }
    }

    updated_project.ok_or(Error::ProjectNotFound(project_id))
}

#[tauri::command]
pub fn delete_project(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    project_id: String,
) -> Result<(), Error> {
    let mut project_name = None;

    // First, find the project name and check if it's running
    {
        let config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
            if let Some(project) = group.projects.iter().find(|p| p.id == project_id) {
                project_name = Some(project.name.clone());
            }
        }
    }

    // Stop the process if running
    if let Err(_) = process_manager::stop_process(&state, &project_id) {
        // Process not running, that's fine
    } else {
        // Wait a moment for process to stop
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let mut group_updated = false;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            // Remove from YAML if syncing
            if let Some(ref name) = project_name {
                if let Err(e) = yaml_config::remove_project_from_yaml(group, name, &state) {
                    eprintln!("Failed to remove project from YAML: {}", e);
                }
            }

            // Remove from group
            let initial_len = group.projects.len();
            group.projects.retain(|p| p.id != project_id);
            group_updated = group.projects.len() != initial_len;
        }

        if group_updated {
            storage::save_config(&app_handle, &config)?;
        }
    }

    if !group_updated {
        return Err(Error::ProjectNotFound(project_id));
    }

    Ok(())
}

#[tauri::command]
pub fn delete_multiple_projects(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    project_ids: Vec<String>,
) -> Result<(), Error> {
    // Collect project names and stop processes
    let mut project_names = Vec::new();
    let mut sync_file_path = None;
    let mut sync_enabled = false;
    {
        let config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
            sync_file_path = group.sync_file.clone();
            sync_enabled = group.sync_enabled;
            for project_id in &project_ids {
                if let Some(project) = group.projects.iter().find(|p| p.id == *project_id) {
                    project_names.push(project.name.clone());
                    // Stop process if running
                    let _ = process_manager::stop_process(&state, project_id);
                }
            }
        }
    }

    // Wait for processes to stop
    std::thread::sleep(std::time::Duration::from_millis(500));

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            // Remove from group
            group.projects.retain(|p| !project_ids.contains(&p.id));
        }

        storage::save_config(&app_handle, &config)?;
    }

    // Sync to YAML after releasing locks (only if sync is enabled)
    if sync_enabled {
        if let Some(yaml_path) = sync_file_path {
            let config = state.config.lock().unwrap();
            if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                let _ = yaml_config::write_yaml(group, std::path::Path::new(&yaml_path));
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn convert_multiple_projects(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    project_ids: Vec<String>,
    new_type: ProjectType,
) -> Result<crate::models::Group, Error> {
    let mut config = state.config.lock().unwrap();
    let group_index = config
        .groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;

    for project in config.groups[group_index].projects.iter_mut() {
        if project_ids.contains(&project.id) {
            project.project_type = new_type.clone();

            // Update auto_restart based on type
            project.auto_restart = new_type == ProjectType::Service;
        }
    }

    storage::save_config(&app_handle, &config)?;

    // Sync to YAML after saving (only if sync is enabled)
    if config.groups[group_index].sync_enabled {
        if let Some(ref yaml_path) = config.groups[group_index].sync_file {
            let _ = yaml_config::write_yaml(
                &config.groups[group_index],
                std::path::Path::new(yaml_path),
            );
        }
    }

    Ok(config.groups[group_index].clone())
}

#[tauri::command]
pub fn get_project(
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    project_id: String,
) -> Result<Project, Error> {
    let config = state.config.lock().unwrap();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        if let Some(project) = group.projects.iter().find(|p| p.id == project_id) {
            return Ok(project.clone());
        }
    }

    Err(Error::ProjectNotFound(project_id))
}

#[tauri::command]
pub fn get_projects(
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
) -> Result<Vec<Project>, Error> {
    let config = state.config.lock().unwrap();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        return Ok(group.projects.clone());
    }

    Err(Error::GroupNotFound(group_id))
}

#[tauri::command]
pub fn update_project_env_vars(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    project_id: String,
    env_vars: std::collections::HashMap<String, String>,
) -> Result<(), Error> {
    let mut sync_file_path = None;
    let mut sync_enabled = false;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            sync_file_path = group.sync_file.clone();
            sync_enabled = group.sync_enabled;
            if let Some(project) = group.projects.iter_mut().find(|p| p.id == project_id) {
                project.env_vars = env_vars;
            }
        }

        storage::save_config(&app_handle, &config)?;
    }

    // Sync to YAML after releasing locks (only if sync is enabled)
    if sync_enabled {
        if let Some(yaml_path) = sync_file_path {
            let config = state.config.lock().unwrap();
            if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                let _ = yaml_config::write_yaml(group, std::path::Path::new(&yaml_path));
            }
        }
    }

    Ok(())
}
