use crate::database;
use crate::error::Error;
use crate::models::{LogMessage, LogStream, ProcessInfo, ProcessStatus};
use crate::platform::{create_platform_manager, PlatformProcessManager};
use crate::state::{AppState, ManagedProcess};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Platform-specific process manager instance
static PLATFORM_MANAGER: std::sync::OnceLock<Box<dyn PlatformProcessManager>> =
    std::sync::OnceLock::new();

fn get_platform_manager() -> &'static dyn PlatformProcessManager {
    PLATFORM_MANAGER
        .get_or_init(|| Box::new(create_platform_manager()))
        .as_ref()
}

/// Kill any orphaned processes from previous app runs
pub fn kill_orphaned_processes(state: &AppState) {
    let pids = state.read_stored_pids();
    get_platform_manager().kill_orphaned_processes(&pids);
    state.clear_pid_file();
}

pub fn spawn_process(
    app_handle: &AppHandle,
    state: &AppState,
    project_id: &str,
    command: &str,
    working_dir: &str,
    env_vars: &std::collections::HashMap<String, String>,
    auto_restart: bool,
) -> Result<(), Error> {
    // Check if already running
    {
        let processes = state.processes.lock().unwrap();
        if processes.contains_key(project_id) {
            return Err(Error::ProcessAlreadyRunning(project_id.to_string()));
        }
    }

    // Create a new session in SQLite
    let session_id = {
        let db = state.db.lock().unwrap();
        database::create_session(&db, project_id)?
    };

    // Track active session
    {
        let mut sessions = state.active_sessions.lock().unwrap();
        sessions.insert(project_id.to_string(), session_id.clone());
    }

    // Get the platform shell command
    let (shell, shell_args) = get_platform_manager().get_shell_command();
    let shell_flag = shell_args[0];

    let mut cmd = Command::new(shell);
    cmd.arg(shell_flag).arg(command);
    cmd.current_dir(working_dir);
    cmd.envs(env_vars);
    // Force color output since many tools disable colors when piped
    cmd.env("FORCE_COLOR", "1");
    cmd.env("CLICOLOR_FORCE", "1");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    // Apply platform-specific configuration
    get_platform_manager().configure_command(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::SpawnError(e.to_string()))?;

    let pid = child.id();

    // Save the PID to disk for orphan cleanup on restart
    if let Some(p) = pid {
        state.save_pid(p);
    }

    // Clear log file for this project on fresh start (keep for backward compat)
    let log_path = state.log_file_path(project_id);
    let _ = std::fs::write(&log_path, b"");

    // Set up stdout reader
    if let Some(stdout) = child.stdout.take() {
        let app = app_handle.clone();
        let pid_str = project_id.to_string();
        let log_path_clone = log_path.clone();
        let sid = session_id.clone();
        tauri::async_runtime::spawn(async move {
            read_stream(stdout, &app, &pid_str, LogStream::Stdout, &log_path_clone, &sid).await;
        });
    }

    // Set up stderr reader
    if let Some(stderr) = child.stderr.take() {
        let app = app_handle.clone();
        let pid_str = project_id.to_string();
        let log_path_clone = log_path.clone();
        let sid = session_id.clone();
        tauri::async_runtime::spawn(async move {
            read_stream(stderr, &app, &pid_str, LogStream::Stderr, &log_path_clone, &sid).await;
        });
    }

    // Store process
    {
        let mut processes = state.processes.lock().unwrap();
        processes.insert(
            project_id.to_string(),
            ManagedProcess {
                child,
                manually_stopped: false,
                session_id: Some(session_id),
            },
        );
    }

    // Update process info
    {
        let mut infos = state.process_infos.lock().unwrap();
        infos.insert(
            project_id.to_string(),
            ProcessInfo {
                project_id: project_id.to_string(),
                status: ProcessStatus::Running,
                pid,
                cpu_usage: 0.0,
                memory_usage: 0,
            },
        );
    }

    // Emit status update
    emit_status_update(app_handle, project_id, ProcessStatus::Running, pid);

    // Spawn exit watcher
    let app = app_handle.clone();
    let project_id_owned = project_id.to_string();
    let command_owned = command.to_string();
    let working_dir_owned = working_dir.to_string();
    let env_vars_owned = env_vars.clone();

    tauri::async_runtime::spawn(async move {
        watch_exit(
            &app,
            &project_id_owned,
            &command_owned,
            &working_dir_owned,
            &env_vars_owned,
            auto_restart,
        )
        .await;
    });

    Ok(())
}

async fn read_stream<R: AsyncReadExt + Unpin>(
    mut reader: R,
    app_handle: &AppHandle,
    project_id: &str,
    stream: LogStream,
    log_path: &std::path::Path,
    session_id: &str,
) {
    let stream_str = match stream {
        LogStream::Stdout => "stdout",
        LogStream::Stderr => "stderr",
    };
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let data = String::from_utf8_lossy(&buf[..n]).to_string();
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                // Append to log file (backward compat)
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)
                {
                    let _ = file.write_all(data.as_bytes());
                }

                // Write to SQLite
                let state = app_handle.state::<Arc<AppState>>();
                if let Ok(db) = state.db.lock() {
                    let _ = database::insert_log(&db, session_id, stream_str, &data, timestamp);
                }

                let msg = LogMessage {
                    project_id: project_id.to_string(),
                    stream,
                    data,
                    timestamp,
                };

                let _ = app_handle.emit("process-log", &msg);
            }
            Err(_) => break,
        }
    }
}

async fn watch_exit(
    app_handle: &AppHandle,
    project_id: &str,
    command: &str,
    working_dir: &str,
    env_vars: &std::collections::HashMap<String, String>,
    auto_restart: bool,
) {
    let state = app_handle.state::<Arc<AppState>>();

    // Wait for process to exit
    let (manually_stopped, exit_success, session_id, pid) = {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let mut processes = state.processes.lock().unwrap();
            if let Some(managed) = processes.get_mut(project_id) {
                match managed.child.try_wait() {
                    Ok(Some(status)) => {
                        let manually_stopped = managed.manually_stopped;
                        let exit_success = status.success();
                        let session_id = managed.session_id.clone();
                        let pid = managed.child.id();
                        processes.remove(project_id);
                        break (manually_stopped, exit_success, session_id, pid);
                    }
                    Ok(None) => continue,
                    Err(_) => {
                        let manually_stopped = managed.manually_stopped;
                        let session_id = managed.session_id.clone();
                        let pid = managed.child.id();
                        processes.remove(project_id);
                        break (manually_stopped, false, session_id, pid);
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
        if let Ok(db) = state.db.lock() {
            let _ = database::end_session(&db, sid, exit_status_str);
        }
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

    // Auto-restart if enabled and not manually stopped
    if auto_restart && !manually_stopped {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Re-check auto_restart from config (it may have changed)
        let should_restart = {
            let config = state.config.lock().unwrap();
            config
                .groups
                .iter()
                .flat_map(|g| &g.projects)
                .find(|p| p.id == project_id)
                .map(|p| p.auto_restart)
                .unwrap_or(false)
        };

        if should_restart {
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
                    command,
                    working_dir,
                    env_vars,
                    auto_restart,
                );
            }
        }
    }
}

pub fn stop_process(state: &AppState, project_id: &str) -> Result<(), Error> {
    let mut processes = state.processes.lock().unwrap();

    let managed = processes
        .get_mut(project_id)
        .ok_or_else(|| Error::ProcessNotRunning(project_id.to_string()))?;

    managed.manually_stopped = true;

    let platform = get_platform_manager();

    // Graceful shutdown
    if let Some(pid) = managed.child.id() {
        platform.graceful_shutdown(pid);

        // Spawn a task to force kill after timeout
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            platform.force_kill(pid);
        });
    }

    Ok(())
}

pub fn kill_all_processes(state: &AppState) {
    let mut processes = state.processes.lock().unwrap();
    let platform = get_platform_manager();

    for (project_id, managed) in processes.iter_mut() {
        managed.manually_stopped = true;

        // End session in SQLite
        if let Some(sid) = &managed.session_id {
            if let Ok(db) = state.db.lock() {
                let _ = database::end_session(&db, sid, "stopped");
            }
        }

        // Remove from active sessions
        if let Ok(mut sessions) = state.active_sessions.lock() {
            sessions.remove(project_id);
        }

        if let Some(pid) = managed.child.id() {
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
    let process_info: Vec<(String, Option<u32>, Option<String>)> = {
        let mut processes = state.processes.lock().unwrap();
        processes
            .iter_mut()
            .map(|(project_id, managed)| {
                managed.manually_stopped = true;
                (
                    project_id.clone(),
                    managed.child.id(),
                    managed.session_id.clone(),
                )
            })
            .collect()
    };

    if process_info.is_empty() {
        return;
    }

    // Update UI to show "stopping" status for all processes
    for (project_id, pid, _) in &process_info {
        emit_status_update(app_handle, project_id, ProcessStatus::Stopping, *pid);
    }

    // Send graceful shutdown to all processes
    for (_, pid, _) in &process_info {
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
    for (_, pid, _) in &process_info {
        if let Some(p) = pid {
            if platform.is_process_running(*p) {
                platform.force_kill(*p);
            }
        }
    }

    // End all sessions in SQLite
    for (project_id, _, session_id) in &process_info {
        if let Some(sid) = session_id {
            if let Ok(db) = state.db.lock() {
                let _ = database::end_session(&db, sid, "stopped");
            }
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

fn emit_status_update(
    app_handle: &AppHandle,
    project_id: &str,
    status: ProcessStatus,
    pid: Option<u32>,
) {
    let info = ProcessInfo {
        project_id: project_id.to_string(),
        status,
        pid,
        cpu_usage: 0.0,
        memory_usage: 0,
    };
    let _ = app_handle.emit("process-status-changed", &info);
}
