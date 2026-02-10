use crate::commands::types::{AppSettings, Error};
use crate::storage;
use tauri::AppHandle;

#[tauri::command]
pub fn get_settings(app_handle: AppHandle) -> Result<AppSettings, Error> {
    storage::load_settings(&app_handle)
}
