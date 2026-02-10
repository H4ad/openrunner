use crate::commands::processes;
use crate::error::Error;
use crate::models::{Group, Project, ProjectType};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_groups(state: State<'_, Arc<crate::state::AppState>>) -> Result<Vec<Group>, Error> {
    let config = state.config.lock().unwrap();
    Ok(config.groups.clone())
}

#[tauri::command]
pub fn create_group(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    name: String,
    directory: String,
    sync_enabled: Option<bool>,
) -> Result<Group, Error> {
    let dir_path = Path::new(&directory);

    // Check if openrunner.yaml or openrunner.yml exists
    if let Some(yaml_path) = yaml_config::find_yaml_file(dir_path) {
        // Parse YAML and create group from it
        let yaml_config = yaml_config::parse_yaml(&yaml_path)
            .map_err(|e| Error::YamlConfig(e))?;
        let mut group = yaml_config::yaml_to_group(&yaml_config, dir_path, &yaml_path);

        // Use provided name if different from YAML
        if name != group.name {
            group.name = name;
        }

        // Set sync_enabled - default to true if YAML exists
        group.sync_enabled = sync_enabled.unwrap_or(true);

        // Save to config
        {
            let mut config = state.config.lock().unwrap();
            config.groups.push(group.clone());
            storage::save_config(&app_handle, &config)?;
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
            env_vars: std::collections::HashMap::new(),
            sync_file: None,
            sync_enabled: true,
        };

        // Create YAML file
        let yaml_path = dir_path.join("openrunner.yaml");
        yaml_config::write_yaml(&group, &yaml_path)
            .map_err(|e| Error::YamlConfig(e))?;
        group.sync_file = Some(yaml_path.to_string_lossy().to_string());

        // Save to config
        {
            let mut config = state.config.lock().unwrap();
            config.groups.push(group.clone());
            storage::save_config(&app_handle, &config)?;
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
            env_vars: std::collections::HashMap::new(),
            sync_file: None,
            sync_enabled: false,
        };

        {
            let mut config = state.config.lock().unwrap();
            config.groups.push(group.clone());
            storage::save_config(&app_handle, &config)?;
        }

        Ok(group)
    }
}

#[tauri::command]
pub fn rename_group(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    name: String,
) -> Result<Group, Error> {
    let mut config = state.config.lock().unwrap();
    let group_index = config
        .groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;

    config.groups[group_index].name = name;

    // Sync to YAML if enabled
    yaml_config::update_group_in_yaml(&config.groups[group_index], &state)
        .map_err(|e| Error::YamlConfig(e))?;

    storage::save_config(&app_handle, &config)?;
    Ok(config.groups[group_index].clone())
}

#[tauri::command]
pub fn update_group_directory(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
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

#[tauri::command]
pub fn update_group_env_vars(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    env_vars: std::collections::HashMap<String, String>,
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
        .map_err(|e| Error::YamlConfig(e))?;

    storage::save_config(&app_handle, &config)?;
    Ok(config.groups[group_index].clone())
}

#[tauri::command]
pub fn export_group(
    state: State<'_, Arc<crate::state::AppState>>,
    group_id: String,
    file_path: String,
) -> Result<(), Error> {
    use crate::yaml_config::{YamlConfig, YamlProject, YamlProjectType};

    let config = state.config.lock().unwrap();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        // Convert group to YAML format
        let yaml_config = YamlConfig {
            version: "1.0".to_string(),
            name: group.name.clone(),
            env_vars: Some(group.env_vars.clone()),
            projects: group
                .projects
                .iter()
                .map(|p| YamlProject {
                    name: p.name.clone(),
                    command: p.command.clone(),
                    project_type: YamlProjectType::from(p.project_type),
                    auto_restart: Some(p.auto_restart),
                    env_vars: Some(p.env_vars.clone()),
                    cwd: p.cwd.clone(),
                    interactive: Some(p.interactive),
                })
                .collect(),
        };

        let yaml = serde_yaml::to_string(&yaml_config)
            .map_err(|e| Error::StorageError(format!("Failed to serialize group: {}", e)))?;
        std::fs::write(&file_path, yaml)
            .map_err(|e| Error::StorageError(format!("Failed to write file: {}", e)))?;
        Ok(())
    } else {
        Err(Error::GroupNotFound(group_id))
    }
}

#[tauri::command]
pub fn import_group(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    file_path: String,
) -> Result<Group, Error> {
    use crate::yaml_config::YamlConfig;

    // Read and parse YAML file
    let yaml_content = std::fs::read_to_string(&file_path)
        .map_err(|e| Error::StorageError(format!("Failed to read file: {}", e)))?;

    let yaml_config: YamlConfig = serde_yaml::from_str(&yaml_content)
        .map_err(|e| Error::StorageError(format!("Failed to parse YAML: {}", e)))?;

            // Convert YAML config to Group
            let mut group = Group {
                id: uuid::Uuid::new_v4().to_string(),
                name: yaml_config.name,
                directory: std::path::Path::new(&file_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| file_path.clone()),
                projects: yaml_config
                    .projects
                    .into_iter()
                    .map(|p| Project {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: p.name,
                        command: p.command,
                        project_type: crate::models::ProjectType::from(p.project_type),
                        auto_restart: p.auto_restart.unwrap_or(true),
                        env_vars: p.env_vars.unwrap_or_default(),
                        cwd: Some(p.cwd.unwrap_or_default()),
                        interactive: p.interactive.unwrap_or(false),
                    })
                    .collect(),
                env_vars: yaml_config.env_vars.unwrap_or_default(),
                sync_file: Some(file_path),
                sync_enabled: true,
            };

    // Check if directory has YAML and set up sync
    let dir_path = Path::new(&group.directory);
    if let Some(yaml_path) = yaml_config::find_yaml_file(dir_path) {
        group.sync_file = Some(yaml_path.to_string_lossy().to_string());
    }

    {
        let mut config = state.config.lock().unwrap();
        config.groups.push(group.clone());
        storage::save_config(&app_handle, &config)?;
    }

    // Start watching YAML if applicable
    if group.sync_file.is_some() {
        let watcher = state.yaml_watcher.lock().unwrap();
        let _ = watcher.watch_group(app_handle, &group);
    }

    Ok(group)
}

#[tauri::command]
pub async fn toggle_group_sync(
    group_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
) -> Result<Group, Error> {
    let sync_file: Option<String>;
    let sync_enabled: bool;
    let group_name: String;
    let group_dir: String;
    let group_projects: Vec<Project>;
    let group_env_vars: HashMap<String, String>;

    {
        let mut config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            group.sync_enabled = !group.sync_enabled;
            sync_enabled = group.sync_enabled;
            sync_file = group.sync_file.clone();
            group_name = group.name.clone();
            group_dir = group.directory.clone();
            group_projects = group.projects.clone();
            group_env_vars = group.env_vars.clone();
        } else {
            return Err(Error::GroupNotFound(group_id));
        }
    }

    // If sync is being enabled but no sync file exists, create it
    if sync_enabled {
        if sync_file.is_none() {
            let yaml_path = PathBuf::from(&group_dir).join("openrunner.yaml");
            let group_clone = Group {
                id: group_id.clone(),
                name: group_name,
                directory: group_dir,
                projects: group_projects,
                env_vars: group_env_vars,
                sync_file: Some(yaml_path.to_string_lossy().to_string()),
                sync_enabled: true,
            };
            
            // Create the YAML file
            if let Err(e) = yaml_config::write_yaml(&group_clone, &yaml_path) {
                // Revert sync_enabled if file creation failed
                let mut config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                    group.sync_enabled = false;
                }
                return Err(Error::YamlConfig(e));
            }
            
            // Update the group with the new sync file path
            let mut config = state.config.lock().unwrap();
            if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                group.sync_file = Some(yaml_path.to_string_lossy().to_string());
            }
            
            // Start watching the new YAML file
            if let Ok(watcher) = state.yaml_watcher.lock() {
                // Get the updated group to pass to watch_group
                let config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                    let _ = watcher.watch_group(app_handle.clone(), group);
                }
            }
        } else {
            // Sync file exists, just update it and start watching
            {
                let mut config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                    let _ = yaml_config::sync_yaml(group, &state);
                }
            }
            
            // Start watching the YAML file
            if let Ok(watcher) = state.yaml_watcher.lock() {
                let config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                    let _ = watcher.watch_group(app_handle.clone(), group);
                }
            }
        }
    } else {
        // Sync is being disabled - stop watching (but keep sync_file path for re-enabling later)
        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.unwatch_group(&group_id);
        }
    }

    // Save config
    let config = state.config.lock().unwrap();
    storage::save_config(&app_handle, &config)?;
    config
        .groups
        .iter()
        .find(|g| g.id == group_id)
        .cloned()
        .ok_or(Error::GroupNotFound(group_id))
}

#[tauri::command]
pub async fn delete_group(
    group_id: String,
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), Error> {
    // Stop all projects in the group first
    let config = state.config.lock().unwrap();
    let project_ids: Vec<String> = if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        group.projects.iter().map(|p| p.id.clone()).collect()
    } else {
        vec![]
    };
    drop(config);
    
    // Stop each project
    for project_id in project_ids {
        let _ = processes::stop_process(state.clone(), project_id);
    }
    
    // Remove the group
    let mut config = state.config.lock().unwrap();
    config.groups.retain(|g| g.id != group_id);
    storage::save_config(&app_handle, &config)?;

    Ok(())
}

#[tauri::command]
pub async fn reload_group_from_yaml(
    group_id: String,
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Group, Error> {
    // Check if there's a sync file
    let sync_file = {
        let config = state.config.lock().unwrap();
        config
            .groups
            .iter()
            .find(|g| g.id == group_id)
            .and_then(|g| g.sync_file.clone())
    };

    if let Some(sync_file) = sync_file {
        let yaml_path = PathBuf::from(&sync_file);
        
        if yaml_path.exists() {
            // Stop all projects in the group before reloading
            let project_ids: Vec<String> = {
                let config = state.config.lock().unwrap();
                if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                    group.projects.iter().map(|p| p.id.clone()).collect()
                } else {
                    return Err(Error::GroupNotFound(group_id));
                }
            };
            
            for project_id in project_ids {
                let _ = crate::commands::processes::stop_process(state.clone(), project_id);
            }
            
            // Now reload the group from YAML
            let mut config = state.config.lock().unwrap();
            let group_clone = if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
                // Use the yaml_config module to parse the YAML
                let yaml_content = std::fs::read_to_string(&yaml_path)
                    .map_err(|e| Error::YamlConfig(format!("Failed to read YAML: {}", e)))?;
                
                let yaml_value: serde_yaml::Value = serde_yaml::from_str(&yaml_content)
                    .map_err(|e| Error::YamlConfig(format!("Failed to parse YAML: {}", e)))?;
                
                // Update group name
                if let Some(name) = yaml_value.get("name").and_then(|v| v.as_str()) {
                    group.name = name.to_string();
                }
                
                // Update env vars
                if let Some(env_vars) = yaml_value.get("envVars").and_then(|v| v.as_mapping()) {
                    group.env_vars.clear();
                    for (k, v) in env_vars {
                        if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                            group.env_vars.insert(key.to_string(), val.to_string());
                        }
                    }
                }
                
                // Update projects
                if let Some(projects) = yaml_value.get("projects").and_then(|v| v.as_sequence()) {
                    group.projects.clear();
                    for project_value in projects {
                        let name = project_value
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unnamed")
                            .to_string();
                        
                        let command = project_value
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let project_type = project_value
                            .get("type")
                            .and_then(|v| v.as_str())
                            .and_then(|t| match t {
                                "service" => Some(ProjectType::Service),
                                "task" => Some(ProjectType::Task),
                                _ => Some(ProjectType::Service),
                            })
                            .unwrap_or(ProjectType::Service);
                        
                        let auto_restart = project_value
                            .get("autoRestart")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        
                        let cwd = project_value
                            .get("cwd")
                            .and_then(|v| v.as_str())
                            .map(|p| {
                                if PathBuf::from(p).is_absolute() {
                                    p.to_string()
                                } else {
                                    PathBuf::from(&group.directory).join(p).to_string_lossy().to_string()
                                }
                            });
                        
                        let project_env_vars: HashMap<String, String> = project_value
                            .get("envVars")
                            .and_then(|v| v.as_mapping())
                            .map(|m| {
                                m.iter()
                                    .filter_map(|(k, v)| {
                                        if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                                            Some((key.to_string(), val.to_string()))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        
                        group.projects.push(Project {
                            id: uuid::Uuid::new_v4().to_string(),
                            name,
                            command,
                            auto_restart,
                            env_vars: project_env_vars,
                            cwd,
                            project_type,
                            interactive: false,
                        });
                    }
                }
                
                Some(group.clone())
            } else {
                None
            };
            
            if let Some(group) = group_clone {
                storage::save_config(&app_handle, &config)?;
                return Ok(group);
            }
        }
    }
    
    Err(Error::YamlConfig("No sync file configured for this group".to_string()))
}
