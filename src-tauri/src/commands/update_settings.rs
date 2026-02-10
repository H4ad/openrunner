use crate::commands::types::{AppSettings, Error};
use crate::storage;
use tauri::AppHandle;

#[tauri::command]
pub fn update_settings(
    app_handle: AppHandle,
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
