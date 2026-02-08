use crate::database;
use crate::error::Error;
use crate::models::{AppSettings, StorageStats};
use crate::state::AppState;
use crate::storage;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, Error> {
    storage::load_settings(&app_handle)
}

#[tauri::command]
pub fn update_settings(
    app_handle: tauri::AppHandle,
    max_log_lines: Option<u32>,
    editor: Option<String>,
) -> Result<AppSettings, Error> {
    let mut settings = storage::load_settings(&app_handle)?;

    if let Some(max_log_lines) = max_log_lines {
        settings.max_log_lines = max_log_lines;
    }

    // Allow setting editor to empty string to clear it
    if editor.is_some() {
        let e = editor.unwrap();
        settings.editor = if e.is_empty() { None } else { Some(e) };
    }

    storage::save_settings(&app_handle, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn detect_system_editor() -> String {
    // Try VISUAL first, then EDITOR
    if let Ok(editor) = std::env::var("VISUAL") {
        if !editor.is_empty() {
            return editor;
        }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.is_empty() {
            return editor;
        }
    }

    // Try to find common editors
    let common_editors = [
        "code",
        "cursor",
        "subl",
        "vim",
        "nvim",
        "nano",
        "emacs",
        "idea",
        "goland",
        "webstorm",
        "zed",
    ];

    for editor in common_editors {
        if std::process::Command::new("which")
            .arg(editor)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return editor.to_string();
        }
    }

    String::new()
}

#[tauri::command]
pub fn get_storage_stats(
    state: State<'_, Arc<AppState>>,
) -> Result<StorageStats, Error> {
    let db = state.db.lock().unwrap();
    database::get_storage_stats(&db)
}

#[tauri::command]
pub fn cleanup_storage(
    state: State<'_, Arc<AppState>>,
    days: u32,
) -> Result<StorageStats, Error> {
    let db = state.db.lock().unwrap();
    database::cleanup_old_data(&db, days)?;
    database::get_storage_stats(&db)
}

#[tauri::command]
pub fn cleanup_all_storage(
    state: State<'_, Arc<AppState>>,
) -> Result<StorageStats, Error> {
    let db = state.db.lock().unwrap();
    database::cleanup_all_data(&db)?;
    database::get_storage_stats(&db)
}
