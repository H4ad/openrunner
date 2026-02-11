use crate::commands::types::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn delete_group(
    group_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), Error> {
    // Get project IDs in the group to stop them
    let config_db = state.config_db.lock().unwrap();
    let project_ids: Vec<String> = if let Some(group) = config_db.get_group(&group_id)? {
        group.projects.iter().map(|p| p.id.clone()).collect()
    } else {
        vec![]
    };
    drop(config_db);

    // Stop each project
    for project_id in project_ids {
        let _ = crate::process::lifecycle::stop_process(&state, &project_id);
    }

    // Remove the group and its database
    let config_db = state.config_db.lock().unwrap();
    config_db.delete_group(&group_id)?;
    
    // Clear the group from cache
    state.group_db_manager.invalidate_cache(&group_id);

    Ok(())
}
