pub mod config_db;
pub mod config_schema;
pub mod group_db;
pub mod group_schema;
pub mod migration;

// Legacy modules (for migration support)
pub mod logs;
pub mod maintenance;
pub mod metrics;
pub mod schema;
pub mod sessions;

// Re-export new database types
pub use config_db::ConfigDatabase;
pub use group_db::GroupDbManager;
pub use migration::{get_migration_status, run_migration_if_needed, MigrationStatus};

// Re-export schema initialization functions
pub use config_schema::init_config_database;
pub use group_schema::init_group_database;

// Legacy re-exports (for migration support)
pub use logs::{
    clear_project_logs, get_project_logs, get_recent_logs, get_session_logs_text, insert_log,
};
pub use maintenance::{
    cleanup_all_data, cleanup_old_data, get_current_session_for_project, get_storage_stats,
};
pub use metrics::{get_last_metric, get_session_metrics, insert_metric};
pub use schema::init_database;
pub use sessions::{
    create_session, delete_session, end_session, get_last_completed_session, get_project_sessions,
    get_project_sessions_with_stats, get_session,
};
