use crate::error::Error;
use crate::models::{LogMessage, LogStream};
use crate::state::AppState;
use portable_pty::PtySize;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncReadExt;

/// Read from a stream and emit log events
pub async fn read_stream<R: AsyncReadExt + Unpin>(
    mut reader: R,
    app_handle: &AppHandle,
    project_id: &str,
    stream: LogStream,
    log_path: &Path,
    _session_id: &str,
) {
    let _stream_str = match stream {
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

                // Write to SQLite - TODO: fix to use group_db_manager
                // let state = app_handle.state::<Arc<AppState>>();
                // if let Ok(conn) = state.group_db_manager.get_connection(group_id) {
                //     let _ = database::insert_log(&conn, session_id, stream_str, &data, timestamp);
                // }

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

/// Write data to a process's stdin (PTY only)
pub fn write_to_process_stdin(
    state: &AppState,
    project_id: &str,
    data: &str,
) -> Result<(), Error> {
    use std::io::Write;
    let mut processes = state.processes.lock().unwrap();

    let managed = processes
        .get_mut(project_id)
        .ok_or_else(|| Error::ProcessNotRunning(project_id.to_string()))?;

    if let Some(ref mut pty_writer) = managed.pty_writer {
        pty_writer
            .write_all(data.as_bytes())
            .map_err(Error::IoError)?;
        pty_writer.flush().map_err(Error::IoError)?;
    }

    Ok(())
}

/// Resize the PTY for an interactive process
pub fn resize_pty(
    state: &AppState,
    project_id: &str,
    cols: u16,
    rows: u16,
) -> Result<(), Error> {
    let processes = state.processes.lock().unwrap();

    let managed = processes
        .get(project_id)
        .ok_or_else(|| Error::ProcessNotRunning(project_id.to_string()))?;

    if let Some(ref pty_master) = managed.pty_master {
        pty_master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::PtyError(e.to_string()))?;
    }

    Ok(())
}
