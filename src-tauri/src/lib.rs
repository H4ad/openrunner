pub mod cli;
mod commands;
mod database;
mod error;
mod file_watcher;
mod models;
mod platform;
mod process;
mod state;
mod stats_collector;
pub mod storage;
mod yaml_config;

use state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_devtools::init());
    }

    builder
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Get configuration directory
            let config_dir = storage::get_config_dir()
                .expect("Failed to get config directory");
            
            // Load configuration from database
            let config = storage::load_config_cli()
                .unwrap_or_else(|e| {
                    eprintln!("Failed to load config: {}, using default", e);
                    models::AppConfig::default()
                });
            
            let log_dir = app
                .path()
                .temp_dir()
                .unwrap_or_else(|_| std::env::temp_dir())
                .join("openrunner-logs");
            
            let _ = std::fs::create_dir_all(&log_dir);

            // Initialize state
            let state = Arc::new(AppState::new(config, log_dir, config_dir));

            // Kill any orphaned processes from previous runs
            process::lifecycle::kill_orphaned_processes(&state);

            // Start stats collector
            stats_collector::start_collector(handle.clone(), state.clone());

            app.manage(state.clone());

            // Start YAML file watchers for groups with sync enabled
            {
                let db = state.database.lock().unwrap();
                let groups = db.get_groups().unwrap_or_default();
                let watcher = state.yaml_watcher.lock().unwrap();
                if let Err(e) = watcher.sync_watchers(handle.clone(), &groups) {
                    eprintln!("Failed to sync YAML watchers: {}", e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Database commands
            commands::get_database_path::get_database_path,

            // Group commands (CLI needs get_groups, create_group, delete_group via storage)
            commands::get_groups::get_groups,
            commands::create_group::create_group,
            commands::delete_group::delete_group,
            commands::export_group::export_group,
            commands::import_group::import_group,
            commands::reload_group_from_yaml::reload_group_from_yaml,
            commands::toggle_group_sync::toggle_group_sync,
            commands::rename_group::rename_group,
            commands::update_group_directory::update_group_directory,
            commands::update_group_env_vars::update_group_env_vars,

            // Project commands
            commands::create_project::create_project,
            commands::update_project::update_project,
            commands::delete_project::delete_project,
            commands::delete_multiple_projects::delete_multiple_projects,
            commands::convert_multiple_projects::convert_multiple_projects,

            // Process commands
            commands::start_process::start_process,
            commands::stop_process::stop_process,
            commands::restart_process::restart_process,
            commands::get_all_statuses::get_all_statuses,
            commands::write_to_process_stdin::write_to_process_stdin,
            commands::resize_pty::resize_pty,

            // Settings commands
            commands::detect_system_editor::detect_system_editor,
            commands::get_settings::get_settings,
            commands::update_settings::update_settings,
            commands::get_storage_stats::get_storage_stats,
            commands::cleanup_storage::cleanup_storage,
            commands::cleanup_all_storage::cleanup_all_storage,

            // Log commands
            commands::read_project_logs::read_project_logs,
            commands::clear_project_logs::clear_project_logs,

            // Session commands
            commands::get_project_sessions::get_project_sessions,
            commands::get_project_sessions_with_stats::get_project_sessions_with_stats,
            commands::get_session_logs::get_session_logs,
            commands::get_session_metrics::get_session_metrics,
            commands::get_last_completed_session::get_last_completed_session,
            commands::get_recent_logs::get_recent_logs,
            commands::get_last_metric::get_last_metric,
            commands::get_session::get_session,
            commands::delete_session::delete_session,

            // File commands
            commands::open_file_in_editor::open_file_in_editor,
            commands::resolve_project_working_dir::resolve_project_working_dir,
            commands::resolve_working_dir_by_project::resolve_working_dir_by_project,
            commands::open_path::open_path,
            commands::open_in_terminal::open_in_terminal,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                tauri::RunEvent::ExitRequested { api, .. } => {
                    let state = app_handle.state::<Arc<AppState>>();
                    let has_running = {
                        let processes = state.processes.lock().unwrap();
                        !processes.is_empty()
                    };

                    if has_running {
                        api.prevent_exit();
                        let _ = app_handle.emit("app-closing", ());
                        let state_clone = state.inner().clone();
                        let handle_clone = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            process::lifecycle::graceful_shutdown_all(&handle_clone, &state_clone).await;
                            handle_clone.exit(0);
                        });
                    }
                }
                tauri::RunEvent::Exit => {
                    let state = app_handle.state::<Arc<AppState>>();
                    process::lifecycle::kill_all_processes(&state);
                }
                _ => {}
            }
        });
}
