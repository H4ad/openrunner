use crate::error::Error;
use crate::models::{AppConfig, AppSettings};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// Get the unified config directory path
/// Uses `~/.config/openrunner/` on all platforms for consistency
pub fn get_config_dir() -> Result<PathBuf, Error> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| Error::StorageError("Could not find config directory".to_string()))?
        .join("openrunner");

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

/// Get the config file path (unified for CLI and UI)
pub fn get_config_path() -> Result<PathBuf, Error> {
    Ok(get_config_dir()?.join("config.json"))
}

/// Get settings path (kept in app_data_dir for now, separate from config)
fn settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, Error> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| Error::StorageError(e.to_string()))?;

    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join("settings.json"))
}

/// Load config (unified path for both CLI and UI)
pub fn load_config(_app_handle: &tauri::AppHandle) -> Result<AppConfig, Error> {
    load_config_cli()
}

/// Save config (unified path for both CLI and UI)
pub fn save_config(_app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), Error> {
    save_config_cli(config)
}

/// Load config in CLI mode (unified path)
pub fn load_config_cli() -> Result<AppConfig, Error> {
    let path = get_config_path()?;

    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).map_err(|e| Error::StorageError(e.to_string()))
}

/// Save config in CLI mode (unified path)
pub fn save_config_cli(config: &AppConfig) -> Result<(), Error> {
    let path = get_config_path()?;
    let data =
        serde_json::to_string_pretty(config).map_err(|e| Error::StorageError(e.to_string()))?;
    fs::write(&path, data)?;
    Ok(())
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
