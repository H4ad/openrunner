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
mod storage;
mod yaml_config;

use state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Get configuration directories
            let config_dir = storage::get_config_dir()
                .expect("Failed to get config directory");
            let groups_dir = storage::get_groups_dir()
                .expect("Failed to get groups directory");
            
            // Run migration from JSON to SQLite if needed
            #[allow(deprecated)]
            if let Err(e) = database::run_migration_if_needed(&config_dir, &groups_dir) {
                eprintln!("Migration warning: {}", e);
                // Continue even if migration fails - might be first run
            }
            
            // Load configuration from new SQLite database
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
            let state = Arc::new(AppState::new(config, log_dir, config_dir, groups_dir));

            // Kill any orphaned processes from previous runs
            process::lifecycle::kill_orphaned_processes(&state);

            // Start stats collector
            stats_collector::start_collector(handle.clone(), state.clone());

            app.manage(state.clone());

            // Start YAML file watchers for groups with sync enabled
            {
                let config_db = state.config_db.lock().unwrap();
                let groups = config_db.get_groups().unwrap_or_default();
                let watcher = state.yaml_watcher.lock().unwrap();
                if let Err(e) = watcher.sync_watchers(handle.clone(), &groups) {
                    eprintln!("Failed to sync YAML watchers: {}", e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Group commands (CLI needs get_groups, create_group, delete_group via storage)
            commands::get_groups::get_groups,
            commands::create_group::create_group,
            commands::delete_group::delete_group,
            commands::export_group::export_group,
            commands::import_group::import_group,
            commands::reload_group_from_yaml::reload_group_from_yaml,
            commands::toggle_group_sync::toggle_group_sync,

            // Process commands
            commands::start_process::start_process,
            commands::stop_process::stop_process,
            commands::restart_process::restart_process,
            commands::get_all_statuses::get_all_statuses,
            commands::write_to_process_stdin::write_to_process_stdin,
            commands::resize_pty::resize_pty,

            // Settings commands
            commands::detect_system_editor::detect_system_editor,
            commands::get_storage_stats::get_storage_stats,
            commands::cleanup_storage::cleanup_storage,
            commands::cleanup_all_storage::cleanup_all_storage,

            // Log commands
            commands::read_project_logs::read_project_logs,
            commands::clear_project_logs::clear_project_logs,

            // Session commands
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
