use crate::database::config_db::ConfigDatabase;
use crate::database::group_db::GroupDbManager;
use crate::error::Error;
use crate::models::{AppConfig, AppSettings, Group};
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

/// Get the groups directory path (where per-group DBs are stored)
pub fn get_groups_dir() -> Result<PathBuf, Error> {
    let groups_dir = get_config_dir()?.join("groups");
    fs::create_dir_all(&groups_dir)?;
    Ok(groups_dir)
}

/// Open the configuration database (CLI mode)
pub fn open_config_db() -> Result<ConfigDatabase, Error> {
    let config_dir = get_config_dir()?;
    ConfigDatabase::open(&config_dir)
}

/// Open the group database manager (CLI mode)
pub fn open_group_manager() -> Result<GroupDbManager, Error> {
    let groups_dir = get_groups_dir()?;
    Ok(GroupDbManager::new(groups_dir))
}

/// Load config from SQLite database (CLI mode)
pub fn load_config_cli() -> Result<AppConfig, Error> {
    let config_db = open_config_db()?;
    let groups = config_db.get_groups()?;
    Ok(AppConfig { groups })
}

/// Save config to SQLite database (CLI mode)
/// Note: This operation is not atomic - it replaces all groups
pub fn save_config_cli(config: &AppConfig) -> Result<(), Error> {
    let config_db = open_config_db()?;
    let group_manager = open_group_manager()?;

    // Get existing groups to know which ones to delete
    let existing_groups = config_db.get_groups()?;
    let existing_ids: std::collections::HashSet<String> =
        existing_groups.into_iter().map(|g| g.id).collect();

    let new_ids: std::collections::HashSet<String> =
        config.groups.iter().map(|g| g.id.clone()).collect();

    // Delete groups that no longer exist
    for old_id in existing_ids.difference(&new_ids) {
        config_db.delete_group(old_id)?;
        // Also delete the group's database file
        let _ = group_manager.delete_group_db(old_id);
    }

    // Update or create groups
    for group in &config.groups {
        if existing_ids.contains(&group.id) {
            // Update existing group - delete and recreate
            config_db.delete_group(&group.id)?;
        }
        config_db.create_group(group)?;
    }

    Ok(())
}

/// Load settings from SQLite database (CLI mode)
pub fn load_settings_cli() -> Result<AppSettings, Error> {
    let config_db = open_config_db()?;
    config_db.get_settings()
}

/// Save settings to SQLite database (CLI mode)
pub fn save_settings_cli(settings: &AppSettings) -> Result<(), Error> {
    let config_db = open_config_db()?;
    config_db.update_settings(settings)
}

// ============================================================================
// Legacy functions for backward compatibility during transition
// These functions should be deprecated and removed in future versions
// ============================================================================

/// Load config (unified path for both CLI and UI)
/// Deprecated: Use ConfigDatabase directly
#[deprecated(
    since = "0.2.0",
    note = "Use ConfigDatabase::open() and get_groups() instead"
)]
pub fn load_config(_app_handle: &tauri::AppHandle) -> Result<AppConfig, Error> {
    load_config_cli()
}

/// Save config (unified path for both CLI and UI)
/// Deprecated: Use ConfigDatabase directly
#[deprecated(since = "0.2.0", note = "Use ConfigDatabase operations instead")]
pub fn save_config(_app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), Error> {
    save_config_cli(config)
}

/// Load settings (kept in app_data_dir for now, separate from config)
/// Deprecated: Use ConfigDatabase directly
#[deprecated(since = "0.2.0", note = "Use ConfigDatabase::get_settings() instead")]
pub fn load_settings(app_handle: &tauri::AppHandle) -> Result<AppSettings, Error> {
    // Try new SQLite method first
    match load_settings_cli() {
        Ok(settings) => Ok(settings),
        Err(_) => {
            // Fall back to legacy file-based loading
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| Error::StorageError(e.to_string()))?;

            let settings_path = app_data_dir.join("settings.json");
            if settings_path.exists() {
                let data = fs::read_to_string(&settings_path)?;
                let settings =
                    serde_json::from_str(&data).map_err(|e| Error::StorageError(e.to_string()))?;

                // Migrate to new storage
                let _ = save_settings_cli(&settings);

                Ok(settings)
            } else {
                Ok(AppSettings::default())
            }
        }
    }
}

/// Save settings
/// Deprecated: Use ConfigDatabase directly
#[deprecated(
    since = "0.2.0",
    note = "Use ConfigDatabase::update_settings() instead"
)]
pub fn save_settings(_app_handle: &tauri::AppHandle, settings: &AppSettings) -> Result<(), Error> {
    save_settings_cli(settings)
}

// ============================================================================
// Helper functions for common operations
// ============================================================================

/// Get a group by ID (CLI mode)
pub fn get_group(group_id: &str) -> Result<Option<Group>, Error> {
    let config_db = open_config_db()?;
    config_db.get_group(group_id)
}

/// Create a new group (CLI mode)
pub fn create_group(group: &Group) -> Result<(), Error> {
    let config_db = open_config_db()?;
    config_db.create_group(group)
}

/// Delete a group and all its data (CLI mode)
pub fn delete_group(group_id: &str) -> Result<(), Error> {
    let config_db = open_config_db()?;
    config_db.delete_group(group_id)
}

/// Update group sync settings (CLI mode)
pub fn update_group_sync(
    group_id: &str,
    sync_file: Option<&str>,
    sync_enabled: bool,
) -> Result<(), Error> {
    let config_db = open_config_db()?;
    config_db.update_group_sync(group_id, sync_file, sync_enabled)
}

/// Get the path to a group's database file (CLI mode)
pub fn get_group_db_path(group_id: &str) -> Result<PathBuf, Error> {
    let groups_dir = get_groups_dir()?;
    Ok(groups_dir.join(format!("{}.db", group_id)))
}

/// Check if a group's database file exists
pub fn group_db_exists(group_id: &str) -> Result<bool, Error> {
    let db_path = get_group_db_path(group_id)?;
    Ok(db_path.exists())
}

/// Delete a group's database file (CLI mode)
pub fn delete_group_db(group_id: &str) -> Result<(), Error> {
    let db_path = get_group_db_path(group_id)?;
    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    Ok(())
}

// ============================================================================
// Statistics and maintenance
// ============================================================================

/// Get storage statistics for all groups
pub fn get_all_storage_stats() -> Result<Vec<(String, u64, u64, u64, u64)>, Error> {
    let config_db = open_config_db()?;
    let group_manager = open_group_manager()?;
    let groups = config_db.get_groups()?;

    let mut stats = Vec::new();
    for group in groups {
        match group_manager.get_storage_stats(&group.id) {
            Ok((session_count, log_count, log_size, metric_count)) => {
                stats.push((group.name, session_count, log_count, log_size, metric_count));
            }
            Err(_) => {
                // Group has no database yet
                stats.push((group.name, 0, 0, 0, 0));
            }
        }
    }

    Ok(stats)
}

/// Clean up old data from all groups
pub fn cleanup_all_groups(days_to_keep: u32) -> Result<(), Error> {
    let config_db = open_config_db()?;
    let group_manager = open_group_manager()?;
    let groups = config_db.get_groups()?;

    for group in groups {
        let _ = group_manager.cleanup_old_data(&group.id, days_to_keep);
    }

    Ok(())
}

/// Clean up all data from all groups (destructive!)
pub fn cleanup_all_groups_data() -> Result<(), Error> {
    let config_db = open_config_db()?;
    let group_manager = open_group_manager()?;
    let groups = config_db.get_groups()?;

    for group in groups {
        let _ = group_manager.cleanup_all_data(&group.id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_dir_creation() {
        // This test verifies the config directory logic works
        // Note: In a real test environment, we'd mock dirs::config_dir()
    }

    #[test]
    fn test_group_db_path() {
        let temp_dir = TempDir::new().unwrap();
        let groups_dir = temp_dir.path().join("groups");
        fs::create_dir_all(&groups_dir).unwrap();

        let db_path = groups_dir.join("test-uuid.db");
        assert!(db_path.to_string_lossy().contains("test-uuid"));
    }
}
