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
    let (command, working_dir, env_vars, auto_restart, project_type) = {
        let config = state.config.lock().unwrap();
        let group = config
            .groups
            .iter()
            .find(|g| g.id == group_id)
            .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;
        let project = group
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| Error::ProjectNotFound(project_id.clone()))?;
        // Merge: group env vars as base, project env vars override
        let mut merged_env = group.env_vars.clone();
        merged_env.extend(project.env_vars.clone());
        (
            project.command.clone(),
            resolve_working_dir(&group.directory, &project.cwd),
            merged_env,
            project.auto_restart,
            project.project_type.clone(),
        )
    };

    let app_state: &AppState = &state;
    let interactive = {
        let config = state.config.lock().unwrap();
        config
            .groups
            .iter()
            .find(|g| g.id == group_id)
            .and_then(|g| g.projects.iter().find(|p| p.id == project_id))
            .map(|p| p.interactive)
            .unwrap_or(false)
    };

    crate::process::spawn_process(
        &app_handle,
        app_state,
        &project_id,
        &command,
        &working_dir,
        &env_vars,
        auto_restart,
        project_type,
        interactive,
    )
}
