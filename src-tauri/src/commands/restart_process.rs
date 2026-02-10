use crate::commands::utils::resolve_working_dir;
use crate::error::Error;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn restart_process(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<(), Error> {
    // Stop first (ignore error if not running)
    let app_state: &AppState = &state;
    let _ = crate::process::lifecycle::stop_process(app_state, &project_id);

    // Wait briefly for process to actually stop, then start
    let app = app_handle.clone();
    let state_arc: Arc<AppState> = state.inner().clone();
    let gid = group_id.clone();
    let pid = project_id.clone();

    tauri::async_runtime::spawn(async move {
        // Wait for the process to be fully removed
        for _ in 0..50 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let processes = state_arc.processes.lock().unwrap();
            if !processes.contains_key(&pid) {
                break;
            }
        }

        let (command, working_dir, env_vars, auto_restart, project_type, interactive) = {
            let config = state_arc.config.lock().unwrap();
            let group = match config.groups.iter().find(|g| g.id == gid) {
                Some(g) => g,
                None => return,
            };
            let project = match group.projects.iter().find(|p| p.id == pid) {
                Some(p) => p,
                None => return,
            };
            let mut merged_env = group.env_vars.clone();
            merged_env.extend(project.env_vars.clone());
            (
                project.command.clone(),
                resolve_working_dir(&group.directory, &project.cwd),
                merged_env,
                project.auto_restart,
                project.project_type.clone(),
                project.interactive,
            )
        };

        let _ = crate::process::spawn_process(
            &app,
            &state_arc,
            &pid,
            &command,
            &working_dir,
            &env_vars,
            auto_restart,
            project_type,
            interactive,
        );
    });

    Ok(())
}
