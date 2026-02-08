pub mod cli;
mod commands;
mod database;
mod error;
mod models;
mod platform;
mod process_manager;
mod state;
mod stats_collector;
mod storage;

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
            process_manager::kill_orphaned_processes(&state);

            // Start stats collector
            stats_collector::start_collector(handle, state.clone());

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::groups::get_groups,
            commands::groups::create_group,
            commands::groups::rename_group,
            commands::groups::update_group_directory,
            commands::groups::update_group_env_vars,
            commands::groups::delete_group,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::processes::start_process,
            commands::processes::stop_process,
            commands::processes::restart_process,
            commands::processes::get_all_statuses,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_storage_stats,
            commands::settings::cleanup_storage,
            commands::settings::cleanup_all_storage,
            commands::logs::read_project_logs,
            commands::logs::clear_project_logs,
            commands::sessions::get_project_sessions,
            commands::sessions::get_project_sessions_with_stats,
            commands::sessions::get_session,
            commands::sessions::get_session_logs,
            commands::sessions::get_session_metrics,
            commands::sessions::delete_session,
            commands::sessions::get_last_completed_session,
            commands::sessions::get_recent_logs,
            commands::sessions::get_last_metric,
            commands::files::open_file_in_editor,
            commands::files::resolve_project_working_dir,
            commands::files::resolve_working_dir_by_project,
            commands::files::open_path,
            commands::files::open_in_terminal,
            commands::settings::detect_system_editor,
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
                            process_manager::graceful_shutdown_all(&handle_clone, &state_clone).await;
                            // After graceful shutdown, exit the app
                            handle_clone.exit(0);
                        });
                    }
                }
                tauri::RunEvent::Exit => {
                    // Final cleanup - kill anything still running
                    let state = app_handle.state::<Arc<AppState>>();
                    process_manager::kill_all_processes(&**state);
                }
                _ => {}
            }
        });
}
