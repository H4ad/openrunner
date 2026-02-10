use crate::commands::types::{Error, Project};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn update_project(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
    updates: HashMap<String, serde_json::Value>,
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
                    if let Ok(vars) =
                        serde_json::from_value::<HashMap<String, String>>(env_vars.clone())
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
