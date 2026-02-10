pub mod schema;
pub mod sessions;
pub mod logs;
pub mod metrics;
pub mod maintenance;

// Re-export all database functions for convenience
pub use schema::init_database;
pub use sessions::{
    create_session,
    end_session,
    get_session,
    get_project_sessions,
    get_project_sessions_with_stats,
    get_last_completed_session,
    delete_session,
};
pub use logs::{
    insert_log,
    get_project_logs,
    get_session_logs_text,
    clear_project_logs,
    get_recent_logs,
};
pub use metrics::{
    insert_metric,
    get_session_metrics,
    get_last_metric,
};
pub use maintenance::{
    get_storage_stats,
    cleanup_old_data,
    cleanup_all_data,
    get_current_session_for_project,
};
