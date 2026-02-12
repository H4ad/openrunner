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
    let db = state.database.lock().unwrap();
    let project_ids: Vec<String> = if let Some(group) = db.get_group(&group_id)? {
        group.projects.iter().map(|p| p.id.clone()).collect()
    } else {
        vec![]
    };
    drop(db);

    // Stop each project
    for project_id in project_ids {
        let _ = crate::process::lifecycle::stop_process(&state, &project_id);
    }

    // Remove the group from the database
    let db = state.database.lock().unwrap();
    db.delete_group(&group_id)?;
    drop(db);

    // Stop watching the group's YAML file
    if let Ok(watcher) = state.yaml_watcher.lock() {
        let _ = watcher.unwatch_group(&group_id);
    }

    Ok(())
}
