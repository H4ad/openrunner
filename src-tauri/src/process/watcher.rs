use crate::models::{ProcessStatus, ProjectType};
use crate::state::AppState;
use crate::process::platform::get_platform_manager;
use crate::process::emit_status_update;
use crate::process::spawn_process;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Watch for process exit and handle auto-restart
pub async fn watch_exit(
    app_handle: &AppHandle,
    project_id: &str,
    command: &str,
    working_dir: &str,
    env_vars: &HashMap<String, String>,
    auto_restart: bool,
    project_type: ProjectType,
    interactive: bool,
) {
    let state = app_handle.state::<Arc<AppState>>();
    let platform = get_platform_manager();

    // Wait for process to exit
    let (manually_stopped, exit_success, session_id, group_id, pid, _is_interactive) = {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let mut processes = state.processes.lock().unwrap();
            if let Some(managed) = processes.get_mut(project_id) {
                let is_interactive = managed.is_interactive;

                // For PTY processes, check if the real process is still running
                if is_interactive {
                    if managed.manually_stopped {
                        let session_id = managed.session_id.clone();
                        let group_id = managed.group_id.clone();
                        let pid = managed.real_pid;
                        processes.remove(project_id);
                        break (true, true, session_id, group_id, pid, true);
                    }
                    // Check if the real PTY process is still running
                    if let Some(real_pid) = managed.real_pid {
                        if !platform.is_process_running(real_pid) {
                            // Process has exited - mark as error since we can't determine exit code
                            let session_id = managed.session_id.clone();
                            let group_id = managed.group_id.clone();
                            processes.remove(project_id);
                            break (false, false, session_id, group_id, Some(real_pid), true);
                        }
                    }
                    // Continue waiting for PTY processes
                    drop(processes);
                    continue;
                }

                match managed.child.try_wait() {
                    Ok(Some(status)) => {
                        let manually_stopped = managed.manually_stopped;
                        let exit_success = status.success();
                        let session_id = managed.session_id.clone();
                        let group_id = managed.group_id.clone();
                        let pid = managed.child.id();
                        processes.remove(project_id);
                        break (manually_stopped, exit_success, session_id, group_id, pid, false);
                    }
                    Ok(None) => continue,
                    Err(_) => {
                        let manually_stopped = managed.manually_stopped;
                        let session_id = managed.session_id.clone();
                        let group_id = managed.group_id.clone();
                        let pid = managed.child.id();
                        processes.remove(project_id);
                        break (manually_stopped, false, session_id, group_id, pid, false);
                    }
                }
            } else {
                return; // Process was already removed
            }
        }
    };

    // Remove the PID from the PID file
    if let Some(p) = pid {
        state.remove_pid(p);
    }

    let status = if manually_stopped || exit_success {
        ProcessStatus::Stopped
    } else {
        ProcessStatus::Errored
    };

    // End session in SQLite
    if let Some(sid) = &session_id {
        let exit_status_str = if manually_stopped {
            "stopped"
        } else if exit_success {
            "stopped"
        } else {
            "errored"
        };
        let db = state.database.lock().unwrap();
        let _ = db.end_session(sid, exit_status_str);
        // Remove from active sessions
        let mut sessions = state.active_sessions.lock().unwrap();
        sessions.remove(project_id);
    }

    // Update process info
    {
        let mut infos = state.process_infos.lock().unwrap();
        if let Some(info) = infos.get_mut(project_id) {
            info.status = status;
            info.pid = None;
            info.cpu_usage = 0.0;
            info.memory_usage = 0;
        }
    }

    emit_status_update(app_handle, project_id, status, None);

    // Auto-restart if enabled, not manually stopped, exited successfully, and project type is Service
    // Tasks should never auto-restart, and crashed processes should not auto-restart
    if auto_restart && !manually_stopped && exit_success && project_type == ProjectType::Service {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Re-check auto_restart and project_type from database (they may have changed)
            let (should_restart, current_project_type) = {
                let db = state.database.lock().unwrap();
                let groups = db.get_groups().unwrap_or_default();
                groups
                    .iter()
                    .flat_map(|g| &g.projects)
                    .find(|p| p.id == project_id)
                    .map(|p| (p.auto_restart, p.project_type))
                    .unwrap_or((false, ProjectType::Service))
            };

        // Only restart if it's still a Service type
        if should_restart && current_project_type == ProjectType::Service {
            // Check process isn't already running (e.g., user restarted manually)
            let already_running = {
                let processes = state.processes.lock().unwrap();
                processes.contains_key(project_id)
            };

            if !already_running {
                let _ = spawn_process(
                    app_handle,
                    &state,
                    project_id,
                    &group_id,
                    command,
                    working_dir,
                    env_vars,
                    should_restart,
                    ProjectType::Service,
                    interactive,
                );
            }
        }
    }
}
