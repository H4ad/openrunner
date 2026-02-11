use crate::error::Error;
use crate::models::{LogMessage, LogStream, ProcessInfo, ProcessStatus};
use crate::state::{AppState, ManagedProcess};
use crate::process::platform::get_platform_manager;
use crate::process::io::read_stream;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

/// Spawn a regular (non-interactive) process using pipes
pub fn spawn_regular_process(
    app_handle: &AppHandle,
    state: &AppState,
    project_id: &str,
    group_id: &str,
    command: &str,
    working_dir: &str,
    env_vars: &HashMap<String, String>,
    session_id: String,
    log_path: &Path,
) -> Result<(), Error> {
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

    // Set up stdout reader
    if let Some(stdout) = child.stdout.take() {
        let app = app_handle.clone();
        let pid_str = project_id.to_string();
        let log_path_clone = log_path.to_path_buf();
        let sid = session_id.clone();
        tauri::async_runtime::spawn(async move {
            read_stream(stdout, &app, &pid_str, LogStream::Stdout, &log_path_clone, &sid).await;
        });
    }

    // Set up stderr reader
    if let Some(stderr) = child.stderr.take() {
        let app = app_handle.clone();
        let pid_str = project_id.to_string();
        let log_path_clone = log_path.to_path_buf();
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
                group_id: group_id.to_string(),
                pty_master: None,
                pty_writer: None,
                is_interactive: false,
                real_pid: pid,
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
    crate::process::emit_status_update(app_handle, project_id, ProcessStatus::Running, pid);

    Ok(())
}

/// Spawn an interactive process using PTY
pub fn spawn_interactive_process(
    app_handle: &AppHandle,
    state: &AppState,
    project_id: &str,
    group_id: &str,
    command: &str,
    working_dir: &str,
    env_vars: &HashMap<String, String>,
    session_id: String,
    log_path: &Path,
) -> Result<(), Error> {
    // Get the native PTY system
    let pty_system = native_pty_system();

    // Open a PTY with default size
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| Error::SpawnError(format!("Failed to open PTY: {}", e)))?;

    // Build the command
    let mut cmd_builder = CommandBuilder::new("sh");
    cmd_builder.arg("-c");
    cmd_builder.arg(command);
    cmd_builder.cwd(std::path::PathBuf::from(working_dir));

    // Add environment variables
    for (key, value) in env_vars {
        cmd_builder.env(key, value);
    }

    // Force color output
    cmd_builder.env("FORCE_COLOR", "1");
    cmd_builder.env("CLICOLOR_FORCE", "1");
    cmd_builder.env("TERM", "xterm-256color");

    // Spawn the command in the PTY slave
    let child = pty_pair
        .slave
        .spawn_command(cmd_builder)
        .map_err(|e| Error::SpawnError(format!("Failed to spawn PTY command: {}", e)))?;

    let pid = child.process_id().map(|p| p as u32);

    // Save the PID to disk for orphan cleanup on restart
    if let Some(p) = pid {
        state.save_pid(p);
    }

    // Clone the master for reading and get writer
    let reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|e| Error::SpawnError(format!("Failed to clone PTY reader: {}", e)))?;

    let writer = pty_pair
        .master
        .take_writer()
        .map_err(|e| Error::SpawnError(format!("Failed to get PTY writer: {}", e)))?;

    // Set up reader for PTY output
    let app = app_handle.clone();
    let pid_str = project_id.to_string();
    let log_path_clone = log_path.to_path_buf();
    let _sid = session_id.clone();

    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    // Append to log file
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path_clone)
                    {
                        let _ = file.write_all(data.as_bytes());
                    }

                    // Write to SQLite - TODO: fix to use group_db_manager
                    // let state = app.state::<Arc<AppState>>();
                    // if let Ok(conn) = state.group_db_manager.get_connection(group_id) {
                    //     let _ = database::insert_log(&conn, &sid, "stdout", &data, timestamp);
                    // }

                    let msg = LogMessage {
                        project_id: pid_str.clone(),
                        stream: LogStream::Stdout,
                        data,
                        timestamp,
                    };

                    let _ = app.emit("process-log", &msg);
                }
                Err(_) => break,
            }
        }
    });

    // Store process with PTY master
    {
        let mut processes = state.processes.lock().unwrap();
        processes.insert(
            project_id.to_string(),
            ManagedProcess {
                child: {
                    // Create a dummy child for compatibility
                    // The actual process is managed by the PTY
                    let mut dummy_cmd = Command::new("echo");
                    dummy_cmd.arg("pty-process");
                    dummy_cmd.spawn().unwrap()
                },
                manually_stopped: false,
                session_id: Some(session_id),
                group_id: group_id.to_string(),
                pty_master: Some(pty_pair.master),
                pty_writer: Some(writer),
                is_interactive: true,
                real_pid: pid,
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
    crate::process::emit_status_update(app_handle, project_id, ProcessStatus::Running, pid);

    Ok(())
}
