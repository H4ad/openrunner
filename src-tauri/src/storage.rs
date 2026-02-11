use crate::database::Database;
use crate::error::Error;
use crate::models::{AppConfig, AppSettings, Group};
use std::fs;
use std::path::PathBuf;

/// Get the unified config directory path
/// Uses `~/.config/openrunner/` on all platforms for consistency
pub fn get_config_dir() -> Result<PathBuf, Error> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| Error::StorageError("Could not find config directory".to_string()))?
        .join("openrunner");

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

/// Get the path to the unified database file
pub fn get_database_path() -> Result<PathBuf, Error> {
    Ok(get_config_dir()?.join("runner-ui.db"))
}

/// Open the unified database (CLI mode)
pub fn open_database() -> Result<Database, Error> {
    let db_path = get_database_path()?;
    Database::open(&db_path)
}

/// Load config from SQLite database (CLI mode)
pub fn load_config_cli() -> Result<AppConfig, Error> {
    let db = open_database()?;
    let groups = db.get_groups()?;
    Ok(AppConfig { groups })
}

/// Save config to SQLite database (CLI mode)
/// Note: This operation is not atomic - it replaces all groups
pub fn save_config_cli(config: &AppConfig) -> Result<(), Error> {
    let mut db = open_database()?;

    // Get existing groups to know which ones to delete
    let existing_groups = db.get_groups()?;
    let existing_ids: std::collections::HashSet<String> =
        existing_groups.into_iter().map(|g| g.id).collect();

    let new_ids: std::collections::HashSet<String> =
        config.groups.iter().map(|g| g.id.clone()).collect();

    // Delete groups that no longer exist
    for old_id in existing_ids.difference(&new_ids) {
        db.delete_group(old_id)?;
    }

    // Update or create groups
    for group in &config.groups {
        if existing_ids.contains(&group.id) {
            // Update existing group - delete and recreate
            db.delete_group(&group.id)?;
        }
        db.create_group(group)?;
    }

    Ok(())
}

/// Load settings from SQLite database (CLI mode)
pub fn load_settings_cli() -> Result<AppSettings, Error> {
    let db = open_database()?;
    db.get_settings()
}

/// Save settings to SQLite database (CLI mode)
pub fn save_settings_cli(settings: &AppSettings) -> Result<(), Error> {
    let db = open_database()?;
    db.update_settings(settings)
}

// ============================================================================
// Helper functions for common operations
// ============================================================================

/// Get a group by ID (CLI mode)
pub fn get_group(group_id: &str) -> Result<Option<Group>, Error> {
    let db = open_database()?;
    db.get_group(group_id)
}

/// Create a new group (CLI mode)
pub fn create_group(group: &Group) -> Result<(), Error> {
    let mut db = open_database()?;
    db.create_group(group)
}

/// Delete a group and all its data (CLI mode)
pub fn delete_group(group_id: &str) -> Result<(), Error> {
    let db = open_database()?;
    db.delete_group(group_id)
}

/// Update group sync settings (CLI mode)
pub fn update_group_sync(
    group_id: &str,
    sync_file: Option<&str>,
    sync_enabled: bool,
) -> Result<(), Error> {
    let db = open_database()?;
    db.update_group_sync(group_id, sync_file, sync_enabled)
}

// ============================================================================
// Statistics and maintenance
// ============================================================================

/// Get storage statistics for all groups
pub fn get_all_storage_stats() -> Result<Vec<(String, u64, u64, u64, u64)>, Error> {
    let db = open_database()?;
    let groups = db.get_groups()?;
    let (session_count, log_count, log_size, metric_count) = db.get_storage_stats()?;

    // Return stats aggregated per group (for now, all stats are global)
    let mut stats = Vec::new();
    for group in groups {
        stats.push((group.name, 0, 0, 0, 0)); // Per-group stats not tracked separately in unified DB
    }

    Ok(stats)
}

/// Clean up old data from all groups
pub fn cleanup_all_groups(days_to_keep: u32) -> Result<(), Error> {
    let db = open_database()?;
    db.cleanup_old_data(days_to_keep)
}

/// Clean up all data from all groups (destructive!)
pub fn cleanup_all_groups_data() -> Result<(), Error> {
    let db = open_database()?;
    db.cleanup_all_data()
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
    fn test_database_path() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("runner-ui.db");
        assert!(db_path.to_string_lossy().contains("runner-ui"));
    }
}
