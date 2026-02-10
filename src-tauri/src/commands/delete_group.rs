use crate::commands::types::{Error, Group};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config;
use std::sync::Arc;
use tauri::{AppHandle, State};

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
        let _ = crate::process::lifecycle::stop_process(&state, &project_id);
    }

    // Remove the group
    let mut config = state.config.lock().unwrap();
    config.groups.retain(|g| g.id != group_id);
    storage::save_config(&app_handle, &config)?;

    Ok(())
}
