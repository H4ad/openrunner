use crate::commands::utils::resolve_working_dir;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn start_process(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<(), Error> {
    // Get group and project from SQLite database
    let (group, project) = {
        let db = state.database.lock().unwrap();
        let group = db
            .get_group(&group_id)?
            .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;
        let project = group
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| Error::ProjectNotFound(project_id.clone()))?
            .clone();
        (group, project)
    };

    let (command, working_dir, env_vars, auto_restart, project_type) = {
        // Merge: group env vars as base, project env vars override
        let mut merged_env = group.env_vars.clone();
        merged_env.extend(project.env_vars.clone());
        (
            project.command.clone(),
            resolve_working_dir(&group.directory, &project.cwd),
            merged_env,
            project.auto_restart,
            project.project_type,
        )
    };

    let app_state: &AppState = &state;
    let interactive = project.interactive;

    crate::process::spawn_process(
        &app_handle,
        app_state,
        &project_id,
        &group_id,
        &command,
        &working_dir,
        &env_vars,
        auto_restart,
        project_type,
        interactive,
    )
}
