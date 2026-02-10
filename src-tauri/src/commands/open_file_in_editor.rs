use crate::commands::utils::resolve_working_dir;
use crate::error::Error;
use crate::state::AppState;
use crate::storage;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn open_file_in_editor(
    app_handle: AppHandle,
    file_path: String,
    line: Option<u32>,
    column: Option<u32>,
    working_dir: String,
) -> Result<(), Error> {
    // Resolve relative paths against working_dir
    let resolved = if PathBuf::from(&file_path).is_absolute() {
        PathBuf::from(&file_path)
    } else {
        PathBuf::from(&working_dir).join(&file_path)
    };

    // Canonicalize to resolve ../ and similar
    let resolved = resolved
        .canonicalize()
        .map_err(|_| Error::FileNotFound(file_path.clone()))?;

    let is_directory = resolved.is_dir();

    if !resolved.exists() {
        return Err(Error::FileNotFound(file_path));
    }

    // For directories, just open in file manager
    if is_directory {
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open")
                .arg(resolved.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open directory: {}", e)))?;
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(resolved.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open directory: {}", e)))?;
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(resolved.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open directory: {}", e)))?;
        }
        return Ok(());
    }

    let resolved_str = resolved.to_string_lossy().to_string();

    // Get configured editor or fall back to environment
    let settings = storage::load_settings(&app_handle).unwrap_or_default();
    let editor = settings
        .editor
        .filter(|e| !e.is_empty())
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_default();

    let editor_name = PathBuf::from(&editor)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let line_num = line.unwrap_or(1);
    let col_num = column.unwrap_or(1);

    let result = match editor_name.as_str() {
        "code" | "code-insiders" | "cursor" => Command::new(&editor)
            .arg("--goto")
            .arg(format!("{}:{}:{}", resolved_str, line_num, col_num))
            .spawn(),
        "zed" => Command::new(&editor)
            .arg(format!("{}:{}:{}", resolved_str, line_num, col_num))
            .spawn(),
        "subl" | "sublime_text" => Command::new(&editor)
            .arg(format!("{}:{}:{}", resolved_str, line_num, col_num))
            .spawn(),
        "vim" | "nvim" | "vi" => Command::new(&editor)
            .arg(format!("+{}", line_num))
            .arg(&resolved_str)
            .spawn(),
        "idea" | "goland" | "webstorm" | "phpstorm" | "pycharm" | "rustrover" | "clion" => {
            Command::new(&editor)
                .arg("--line")
                .arg(line_num.to_string())
                .arg("--column")
                .arg(col_num.to_string())
                .arg(&resolved_str)
                .spawn()
        }
        "emacs" | "emacsclient" => Command::new(&editor)
            .arg(format!("+{}:{}", line_num, col_num))
            .arg(&resolved_str)
            .spawn(),
        _ if !editor.is_empty() => Command::new(&editor).arg(&resolved_str).spawn(),
        _ => {
            // Fallback to platform default
            #[cfg(target_os = "linux")]
            {
                Command::new("xdg-open").arg(&resolved_str).spawn()
            }
            #[cfg(target_os = "macos")]
            {
                Command::new("open").arg(&resolved_str).spawn()
            }
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(["/C", "start", "", &resolved_str])
                    .spawn()
            }
        }
    };

    result.map_err(|e| Error::SpawnError(format!("Failed to open editor: {}", e)))?;

    Ok(())
}
