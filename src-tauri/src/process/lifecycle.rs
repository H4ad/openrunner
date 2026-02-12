use crate::error::Error;
use crate::state::AppState;
use crate::process::platform::get_platform_manager;
use crate::process::emit_status_update;
use crate::models::ProcessStatus;
use std::time::Duration;
use tauri::AppHandle;

/// Stop a running process
pub fn stop_process(state: &AppState, project_id: &str) -> Result<(), Error> {
    let mut processes = state.processes.lock().unwrap();

    let managed = processes
        .get_mut(project_id)
        .ok_or_else(|| Error::ProcessNotRunning(project_id.to_string()))?;

    managed.manually_stopped = true;

    let platform = get_platform_manager();

    // Use real_pid for PTY processes, child.id() for regular processes
    let pid_to_kill = if managed.is_interactive {
        managed.real_pid
    } else {
        managed.child.id()
    };

    // Graceful shutdown
    if let Some(pid) = pid_to_kill {
        platform.graceful_shutdown(pid);

        // Spawn a task to force kill after timeout
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            platform.force_kill(pid);
        });
    }

    Ok(())
}

/// Kill all running processes immediately
pub fn kill_all_processes(state: &AppState) {
    let mut processes = state.processes.lock().unwrap();
    let platform = get_platform_manager();

    for (project_id, managed) in processes.iter_mut() {
        managed.manually_stopped = true;

        // End session in SQLite
        if let Some(sid) = &managed.session_id {
            let db = state.database.lock().unwrap();
            let _ = db.end_session(sid, "stopped");
        }

        // Remove from active sessions
        if let Ok(mut sessions) = state.active_sessions.lock() {
            sessions.remove(project_id);
        }

        // Use real_pid for PTY processes, child.id() for regular processes
        let pid_to_kill = if managed.is_interactive {
            managed.real_pid
        } else {
            managed.child.id()
        };

        if let Some(pid) = pid_to_kill {
            platform.force_kill(pid);
        }
    }
    processes.clear();

    // Clear the PID file since all processes are now dead
    state.clear_pid_file();
}

/// Gracefully shutdown all processes with UI feedback
/// Sends graceful shutdown signal, waits for processes to exit, then force kills if needed
pub async fn graceful_shutdown_all(app_handle: &AppHandle, state: &AppState) {
    let platform = get_platform_manager();

    // Collect all running process info
    let process_info: Vec<(String, String, Option<u32>, Option<String>, bool)> = {
        let mut processes = state.processes.lock().unwrap();
        processes
            .iter_mut()
            .map(|(project_id, managed)| {
                managed.manually_stopped = true;
                // Use real_pid for PTY processes, child.id() for regular processes
                let pid = if managed.is_interactive {
                    managed.real_pid
                } else {
                    managed.child.id()
                };
                (
                    project_id.clone(),
                    managed.group_id.clone(),
                    pid,
                    managed.session_id.clone(),
                    managed.is_interactive,
                )
            })
            .collect()
    };

    if process_info.is_empty() {
        return;
    }

    // Update UI to show "stopping" status for all processes
    for (project_id, _, pid, _, _) in &process_info {
        emit_status_update(app_handle, project_id, ProcessStatus::Stopping, *pid);
    }

    // Send graceful shutdown to all processes
    for (_, _, pid, _, _) in &process_info {
        if let Some(p) = pid {
            platform.graceful_shutdown(*p);
        }
    }

    // Wait for processes to exit (up to 5 seconds)
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let all_exited = {
            let mut processes = state.processes.lock().unwrap();
            let mut all_done = true;
            let mut to_remove = Vec::new();

            for (project_id, managed) in processes.iter_mut() {
                if managed.is_interactive {
                    // For PTY processes, check if the real process is still running
                    if let Some(real_pid) = managed.real_pid {
                        if !platform.is_process_running(real_pid) {
                            to_remove.push(project_id.clone());
                        } else {
                            all_done = false;
                        }
                    } else {
                        to_remove.push(project_id.clone());
                    }
                } else {
                    // For regular processes, use try_wait
                    match managed.child.try_wait() {
                        Ok(Some(_)) => {
                            to_remove.push(project_id.clone());
                        }
                        Ok(None) => {
                            all_done = false;
                        }
                        Err(_) => {
                            to_remove.push(project_id.clone());
                        }
                    }
                }
            }

            for project_id in to_remove {
                processes.remove(&project_id);
            }

            all_done
        };

        if all_exited || start.elapsed() >= timeout {
            break;
        }
    }

    // Force kill any remaining processes
    for (_, _, pid, _, _) in &process_info {
        if let Some(p) = pid {
            if platform.is_process_running(*p) {
                platform.force_kill(*p);
            }
        }
    }

    // End all sessions in SQLite
    for (project_id, _group_id, _, session_id, _) in &process_info {
        if let Some(sid) = session_id {
            let db = state.database.lock().unwrap();
            let _ = db.end_session(sid, "stopped");
        }

        // Remove from active sessions
        if let Ok(mut sessions) = state.active_sessions.lock() {
            sessions.remove(project_id);
        }

        // Update UI to show stopped
        emit_status_update(app_handle, project_id, ProcessStatus::Stopped, None);
    }

    // Clear process map and PID file
    {
        let mut processes = state.processes.lock().unwrap();
        processes.clear();
    }
    state.clear_pid_file();
}

/// Kill any orphaned processes from previous app runs
pub fn kill_orphaned_processes(state: &AppState) {
    let pids = state.read_stored_pids();
    get_platform_manager().kill_orphaned_processes(&pids);
    state.clear_pid_file();
}
