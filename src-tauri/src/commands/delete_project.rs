use crate::commands::types::Error;
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn delete_project(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
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
    if crate::process::lifecycle::stop_process(&state, &project_id).is_ok() {
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
