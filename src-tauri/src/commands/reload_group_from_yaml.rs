use crate::commands::types::{Error, Group, Project, ProjectType};
use crate::state::AppState;
use crate::storage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

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
                let _ = crate::process::lifecycle::stop_process(&state, &project_id);
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
