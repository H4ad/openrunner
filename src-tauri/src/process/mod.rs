pub mod platform;
pub mod spawn;
pub mod lifecycle;
pub mod watcher;
pub mod io;

// Re-export commonly used functions for convenience
pub use lifecycle::{
    stop_process,
    kill_all_processes,
    graceful_shutdown_all,
    kill_orphaned_processes,
};
pub use io::{
    write_to_process_stdin,
    resize_pty,
};

use crate::error::Error;
use crate::models::{ProcessInfo, ProcessStatus, ProjectType};
use crate::state::AppState;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};

/// Spawn a process for a project
pub fn spawn_process(
    app_handle: &AppHandle,
    state: &AppState,
    project_id: &str,
    group_id: &str,
    command: &str,
    working_dir: &str,
    env_vars: &HashMap<String, String>,
    auto_restart: bool,
    project_type: ProjectType,
    interactive: bool,
) -> Result<(), Error> {
    // Check if already running
    {
        let processes = state.processes.lock().unwrap();
        if processes.contains_key(project_id) {
            return Err(Error::ProcessAlreadyRunning(project_id.to_string()));
        }
    }

    // Create a new session in the database
    let session_id = state.db().create_session(project_id)?;

    // Track active session
    {
        let mut sessions = state.active_sessions.lock().unwrap();
        sessions.insert(project_id.to_string(), session_id.clone());
    }

    // Clear log file for this project on fresh start (keep for backward compat)
    let log_path = state.log_file_path(project_id);
    let _ = std::fs::write(&log_path, b"");

    if interactive {
        // Spawn using PTY for interactive mode
        spawn::spawn_interactive_process(
            app_handle,
            state,
            project_id,
            group_id,
            command,
            working_dir,
            env_vars,
            session_id,
            &log_path,
        )?;
    } else {
        // Spawn using regular pipes for non-interactive mode
        spawn::spawn_regular_process(
            app_handle,
            state,
            project_id,
            group_id,
            command,
            working_dir,
            env_vars,
            session_id,
            &log_path,
        )?;
    }

    // Spawn exit watcher
    let app = app_handle.clone();
    let project_id_owned = project_id.to_string();
    let command_owned = command.to_string();
    let working_dir_owned = working_dir.to_string();
    let env_vars_owned = env_vars.clone();

    tauri::async_runtime::spawn(async move {
        watcher::watch_exit(
            &app,
            &project_id_owned,
            &command_owned,
            &working_dir_owned,
            &env_vars_owned,
            auto_restart,
            project_type,
            interactive,
        )
        .await;
    });

    Ok(())
}

/// Emit a status update event
pub fn emit_status_update(
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
