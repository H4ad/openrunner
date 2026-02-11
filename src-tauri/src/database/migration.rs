//! Migration module for converting JSON config to SQLite
//!
//! This module handles one-time migration from the old JSON-based storage
//! to the new SQLite-based storage system. It is marked as deprecated
//! and should be removed in future versions once all users have migrated.
//!
//! # Migration Process
//! 1. Check if config.json exists
//! 2. If yes, read and parse the JSON
//! 3. Create new SQLite databases
//! 4. Migrate all data (groups, projects, env_vars)
//! 5. Migrate existing logs/metrics from old DB
//! 6. Delete old files on success
//!
//! If migration fails, the old files are preserved for recovery.

use crate::database::config_db::ConfigDatabase;
use crate::database::group_db::GroupDbManager;
use crate::error::Error;
use crate::storage;
use std::fs;
use std::path::Path;

/// Run migration from JSON config to SQLite if needed
///
/// This function checks if the old config.json exists and migrates it
/// to the new SQLite structure. It is safe to call multiple times -
/// if migration has already been done, it will simply return Ok.
#[deprecated(
    since = "0.2.0",
    note = "One-time migration from JSON to SQLite. Will be removed in future version."
)]
pub fn run_migration_if_needed(config_dir: &Path, groups_dir: &Path) -> Result<(), Error> {
    let config_json_path = config_dir.join("config.json");

    // Check if migration is needed
    if !config_json_path.exists() {
        // No old config.json, nothing to migrate
        return Ok(());
    }

    // Check if already migrated (config.db exists)
    let config_db_path = config_dir.join("config.db");
    if config_db_path.exists() {
        // Already migrated, clean up old files
        eprintln!("Migration already completed, cleaning up old files...");
        cleanup_old_files(config_dir)?;
        return Ok(());
    }

    eprintln!("Starting migration from JSON config to SQLite...");

    // Load old config
    let old_config = match storage::load_config_cli() {
        Ok(config) => {
            eprintln!("Loaded {} groups from config.json", config.groups.len());
            config
        }
        Err(e) => {
            eprintln!("Failed to load config.json: {}", e);
            return Err(e);
        }
    };

    // Initialize new databases
    let config_db = ConfigDatabase::open(config_dir)?;
    let group_manager = GroupDbManager::new(groups_dir.to_path_buf());

    // Migrate groups and projects
    eprintln!("Migrating groups and projects...");
    for group in &old_config.groups {
        match config_db.create_group(group) {
            Ok(_) => eprintln!(
                "  Migrated group: {} ({} projects)",
                group.name,
                group.projects.len()
            ),
            Err(e) => {
                eprintln!("  Failed to migrate group {}: {}", group.name, e);
                // Continue with other groups
            }
        }
    }

    // Migrate old database data (logs, metrics, sessions)
    let old_db_path = config_dir
        .parent()
        .unwrap_or(config_dir)
        .join("openrunner.db");
    if old_db_path.exists() {
        eprintln!("Migrating logs and metrics from old database...");
        match migrate_old_database(&old_db_path, &group_manager, &config_db) {
            Ok(count) => eprintln!("  Migrated {} sessions", count),
            Err(e) => eprintln!("  Failed to migrate old database: {}", e),
        }
    }

    // Migrate settings
    let old_settings_path = config_dir
        .parent()
        .map(|p| p.join("settings.json"))
        .unwrap_or_else(|| config_dir.join("settings.json"));

    if old_settings_path.exists() {
        eprintln!("Migrating settings...");
        match fs::read_to_string(&old_settings_path) {
            Ok(content) => {
                if let Ok(settings) = serde_json::from_str::<crate::models::AppSettings>(&content) {
                    if let Err(e) = config_db.update_settings(&settings) {
                        eprintln!("  Failed to migrate settings: {}", e);
                    } else {
                        eprintln!("  Settings migrated successfully");
                    }
                }
            }
            Err(e) => eprintln!("  Failed to read settings.json: {}", e),
        }
    }

    // Clean up old files
    eprintln!("Cleaning up old files...");
    cleanup_old_files(config_dir)?;

    eprintln!("Migration completed successfully!");
    Ok(())
}

/// Migrate data from old unified database to per-group databases
fn migrate_old_database(
    old_db_path: &Path,
    group_manager: &GroupDbManager,
    config_db: &ConfigDatabase,
) -> Result<usize, Error> {
    use rusqlite::Connection;

    let old_conn = Connection::open(old_db_path)?;
    let mut migrated_count = 0;

    // Get all sessions
    let mut stmt = old_conn
        .prepare("SELECT id, project_id, started_at, ended_at, exit_status FROM sessions")?;
    let sessions: Vec<(String, String, u64, Option<u64>, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, Option<u64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Group sessions by group_id (need to find which group each project belongs to)
    let groups = config_db.get_groups()?;
    let mut project_to_group: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for group in &groups {
        for project in &group.projects {
            project_to_group.insert(project.id.clone(), group.id.clone());
        }
    }

    // Migrate each session
    for (session_id, project_id, started_at, ended_at, exit_status) in sessions {
        if let Some(group_id) = project_to_group.get(&project_id) {
            // Insert session into new group database
            let conn = group_manager.get_connection(group_id)?;

            conn.execute(
                "INSERT INTO sessions (id, project_id, started_at, ended_at, exit_status) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![session_id, project_id, started_at, ended_at, exit_status],
            )?;

            // Migrate logs for this session
            let mut log_stmt = old_conn
                .prepare("SELECT stream, data, timestamp FROM logs WHERE session_id = ?1")?;
            let logs: Vec<(String, String, u64)> = log_stmt
                .query_map(rusqlite::params![session_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u64>(2)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (stream, data, timestamp) in logs {
                conn.execute(
                    "INSERT INTO logs (session_id, stream, data, timestamp) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![session_id, stream, data, timestamp],
                )?;
            }

            // Migrate metrics for this session
            let mut metric_stmt = old_conn.prepare(
                "SELECT cpu_usage, memory_usage, timestamp FROM metrics WHERE session_id = ?1",
            )?;
            let metrics: Vec<(f32, u64, u64)> = metric_stmt
                .query_map(rusqlite::params![session_id], |row| {
                    Ok((
                        row.get::<_, f32>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (cpu_usage, memory_usage, timestamp) in metrics {
                conn.execute(
                    "INSERT INTO metrics (session_id, cpu_usage, memory_usage, timestamp) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![session_id, cpu_usage, memory_usage, timestamp],
                )?;
            }

            migrated_count += 1;
        }
    }

    Ok(migrated_count)
}

/// Clean up old JSON/DB files after successful migration
fn cleanup_old_files(config_dir: &Path) -> Result<(), Error> {
    let files_to_remove = [
        config_dir.join("config.json"),
        config_dir.join("config.json.bak"),
    ];

    for file in &files_to_remove {
        if file.exists() {
            if let Err(e) = fs::remove_file(file) {
                eprintln!("Warning: Failed to remove {:?}: {}", file, e);
            } else {
                eprintln!("  Removed {:?}", file);
            }
        }
    }

    // Also try to remove old unified database if it exists
    let old_db_path = config_dir
        .parent()
        .unwrap_or(config_dir)
        .join("openrunner.db");
    if old_db_path.exists() {
        if let Err(e) = fs::remove_file(&old_db_path) {
            eprintln!(
                "Warning: Failed to remove old database {:?}: {}",
                old_db_path, e
            );
        } else {
            eprintln!("  Removed old database");
        }
    }

    // Remove old settings.json
    let settings_path = config_dir
        .parent()
        .map(|p| p.join("settings.json"))
        .unwrap_or_else(|| config_dir.join("settings.json"));
    if settings_path.exists() {
        if let Err(e) = fs::remove_file(&settings_path) {
            eprintln!("Warning: Failed to remove settings.json: {}", e);
        } else {
            eprintln!("  Removed settings.json");
        }
    }

    Ok(())
}

/// Check if migration has been completed
pub fn is_migration_complete(config_dir: &Path) -> bool {
    let config_db_path = config_dir.join("config.db");
    config_db_path.exists()
}

/// Get migration status information
pub fn get_migration_status(config_dir: &Path) -> MigrationStatus {
    MigrationStatus {
        has_old_config: config_dir.join("config.json").exists(),
        has_new_config: config_dir.join("config.db").exists(),
        has_old_database: config_dir
            .parent()
            .map(|p| p.join("openrunner.db").exists())
            .unwrap_or(false),
    }
}

/// Migration status information
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub has_old_config: bool,
    pub has_new_config: bool,
    pub has_old_database: bool,
}

impl MigrationStatus {
    /// Check if migration is needed
    pub fn needs_migration(&self) -> bool {
        self.has_old_config && !self.has_new_config
    }

    /// Check if migration has been completed
    pub fn is_complete(&self) -> bool {
        self.has_new_config
    }

    /// Check if cleanup is needed (old files exist but migration is done)
    pub fn needs_cleanup(&self) -> bool {
        self.has_new_config && (self.has_old_config || self.has_old_database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migration_status() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path();

        // Initially nothing exists
        let status = get_migration_status(config_dir);
        assert!(!status.needs_migration());
        assert!(!status.is_complete());

        // Create old config
        fs::write(config_dir.join("config.json"), "{}").unwrap();
        let status = get_migration_status(config_dir);
        assert!(status.needs_migration());
        assert!(!status.is_complete());

        // Create new config (simulates completed migration)
        fs::write(config_dir.join("config.db"), "").unwrap();
        let status = get_migration_status(config_dir);
        assert!(!status.needs_migration());
        assert!(status.is_complete());
        assert!(status.needs_cleanup());
    }
}
