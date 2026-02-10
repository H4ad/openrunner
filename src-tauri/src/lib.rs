pub mod cli;
mod commands;
mod config_watcher;
mod database;
mod error;
mod file_watcher;
mod models;
mod platform;
mod process;
mod state;
mod stats_collector;
mod storage;
mod yaml_config;

// use file_watcher::start_yaml_watcher; // Function doesn't exist
use state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let config = storage::load_config(&handle).unwrap_or_default();
            let log_dir = app
                .path()
                .temp_dir()
                .unwrap_or_else(|_| std::env::temp_dir())
                .join("openrunner-logs");

            // Initialize SQLite database
            let db_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("openrunner"));
            let _ = std::fs::create_dir_all(&db_dir);
            let db_path = db_dir.join("openrunner.db");
            let db = database::init_database(&db_path)
                .expect("Failed to initialize database");

            let state = Arc::new(AppState::new(config, log_dir, db, db_dir));

            // Kill any orphaned processes from previous runs
            process::lifecycle::kill_orphaned_processes(&state);

            // Start stats collector
            stats_collector::start_collector(handle.clone(), state.clone());

            // Start config file watcher
            match config_watcher::start_config_watcher(handle.clone()) {
                Ok(watcher) => {
                    // Store the watcher in the app state to keep it alive
                    // We need to add this to AppState
                    app.manage(watcher);
                }
                Err(e) => {
                    eprintln!("Failed to start config watcher: {}", e);
                }
            }

            app.manage(state.clone());

            // Start YAML file watchers for groups with sync enabled
            {
                let config = state.config.lock().unwrap();
                let watcher = state.yaml_watcher.lock().unwrap();
                if let Err(e) = watcher.sync_watchers(handle.clone(), &config.groups) {
                    eprintln!("Failed to sync YAML watchers: {}", e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Group commands (11)
            commands::get_groups::get_groups,
            commands::create_group::create_group,
            commands::rename_group::rename_group,
            commands::update_group_directory::update_group_directory,
            commands::update_group_env_vars::update_group_env_vars,
            commands::delete_group::delete_group,
            commands::export_group::export_group,
            commands::import_group::import_group,
            commands::reload_group_from_yaml::reload_group_from_yaml,
            commands::toggle_group_sync::toggle_group_sync,

            // Project commands (10)
            commands::create_project::create_project,
            commands::update_project::update_project,
            commands::delete_project::delete_project,
            commands::delete_multiple_projects::delete_multiple_projects,
            commands::convert_multiple_projects::convert_multiple_projects,
            commands::get_project::get_project,
            commands::get_projects::get_projects,
            commands::update_project_env_vars::update_project_env_vars,

            // Process commands (6)
            commands::start_process::start_process,
            commands::stop_process::stop_process,
            commands::restart_process::restart_process,
            commands::get_all_statuses::get_all_statuses,
            commands::write_to_process_stdin::write_to_process_stdin,
            commands::resize_pty::resize_pty,

            // Settings commands (6)
            commands::get_settings::get_settings,
            commands::update_settings::update_settings,
            commands::detect_system_editor::detect_system_editor,
            commands::get_storage_stats::get_storage_stats,
            commands::cleanup_storage::cleanup_storage,
            commands::cleanup_all_storage::cleanup_all_storage,

            // Log commands (2)
            commands::read_project_logs::read_project_logs,
            commands::clear_project_logs::clear_project_logs,

            // Session commands (9)
            commands::get_project_sessions::get_project_sessions,
            commands::get_project_sessions_with_stats::get_project_sessions_with_stats,
            commands::get_session::get_session,
            commands::get_session_logs::get_session_logs,
            commands::get_session_metrics::get_session_metrics,
            commands::delete_session::delete_session,
            commands::get_last_completed_session::get_last_completed_session,
            commands::get_recent_logs::get_recent_logs,
            commands::get_last_metric::get_last_metric,

            // File commands (5)
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
                    // Check if we have any running processes
                    let state = app_handle.state::<Arc<AppState>>();
                    let has_running = {
                        let processes = state.processes.lock().unwrap();
                        !processes.is_empty()
                    };

                    if has_running {
                        // Prevent immediate exit
                        api.prevent_exit();

                        // Emit event to frontend so it can show shutdown UI
                        let _ = app_handle.emit("app-closing", ());

                        // Gracefully stop all processes
                        let state_clone = state.inner().clone();
                        let handle_clone = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            process::lifecycle::graceful_shutdown_all(&handle_clone, &state_clone).await;
                            // After graceful shutdown, exit the app
                            handle_clone.exit(0);
                        });
                    }
                }
                tauri::RunEvent::Exit => {
                    // Final cleanup - kill anything still running
                    let state = app_handle.state::<Arc<AppState>>();
                    process::lifecycle::kill_all_processes(&**state);
                }
                _ => {}
            }
        });
}
