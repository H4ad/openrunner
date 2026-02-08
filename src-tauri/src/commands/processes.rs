use crate::error::Error;
use crate::models::ProcessInfo;
use crate::process_manager;
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

fn resolve_working_dir(group_dir: &str, project_cwd: &Option<String>) -> String {
    match project_cwd {
        Some(cwd) if !cwd.is_empty() => {
            let path = PathBuf::from(cwd);
            if path.is_absolute() {
                cwd.clone()
            } else {
                PathBuf::from(group_dir).join(cwd).to_string_lossy().to_string()
            }
        }
        _ => group_dir.to_string(),
    }
}

#[tauri::command]
pub async fn start_process(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<(), Error> {
    let (command, working_dir, env_vars, auto_restart) = {
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
        )
    };

    let app_state: &AppState = &state;
    process_manager::spawn_process(
        &app_handle,
        app_state,
        &project_id,
        &command,
        &working_dir,
        &env_vars,
        auto_restart,
    )
}

#[tauri::command]
pub fn stop_process(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<(), Error> {
    let app_state: &AppState = &state;
    process_manager::stop_process(app_state, &project_id)
}

#[tauri::command]
pub fn restart_process(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<(), Error> {
    // Stop first (ignore error if not running)
    let app_state: &AppState = &state;
    let _ = process_manager::stop_process(app_state, &project_id);

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

        let (command, working_dir, env_vars, auto_restart) = {
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
            )
        };

        let _ = process_manager::spawn_process(
            &app,
            &state_arc,
            &pid,
            &command,
            &working_dir,
            &env_vars,
            auto_restart,
        );
    });

    Ok(())
}

#[tauri::command]
pub fn get_all_statuses(state: State<'_, Arc<AppState>>) -> Result<Vec<ProcessInfo>, Error> {
    let infos = state.process_infos.lock().unwrap();
    Ok(infos.values().cloned().collect())
}
