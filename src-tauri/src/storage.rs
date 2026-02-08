use crate::error::Error;
use crate::models::{AppConfig, AppSettings};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

fn config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, Error> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| Error::StorageError(e.to_string()))?;

    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join("config.json"))
}

pub fn load_config(app_handle: &tauri::AppHandle) -> Result<AppConfig, Error> {
    let path = config_path(app_handle)?;

    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).map_err(|e| Error::StorageError(e.to_string()))
}

pub fn save_config(app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), Error> {
    let path = config_path(app_handle)?;
    let data =
        serde_json::to_string_pretty(config).map_err(|e| Error::StorageError(e.to_string()))?;
    fs::write(&path, data)?;
    Ok(())
}

fn settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, Error> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| Error::StorageError(e.to_string()))?;

    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join("settings.json"))
}

pub fn load_settings(app_handle: &tauri::AppHandle) -> Result<AppSettings, Error> {
    let path = settings_path(app_handle)?;

    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).map_err(|e| Error::StorageError(e.to_string()))
}

pub fn save_settings(app_handle: &tauri::AppHandle, settings: &AppSettings) -> Result<(), Error> {
    let path = settings_path(app_handle)?;
    let data =
        serde_json::to_string_pretty(settings).map_err(|e| Error::StorageError(e.to_string()))?;
    fs::write(&path, data)?;
    Ok(())
}
