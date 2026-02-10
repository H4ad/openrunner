use crate::commands::types::Error;
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn delete_multiple_projects(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_ids: Vec<String>,
) -> Result<(), Error> {
    // Collect project names and stop processes
    let mut sync_file_path = None;
    let mut sync_enabled = false;
    {
        let config = state.config.lock().unwrap();
        if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
            sync_file_path = group.sync_file.clone();
            sync_enabled = group.sync_enabled;
            for project_id in &project_ids {
                // Stop process if running
                let _ = crate::process::lifecycle::stop_process(&state, project_id);
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
