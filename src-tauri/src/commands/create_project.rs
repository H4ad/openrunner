use crate::commands::types::{Error, Project, ProjectType};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn create_project(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
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
        env_vars: HashMap::new(),
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
