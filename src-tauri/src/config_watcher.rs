use crate::error::Error;
use crate::storage;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Start watching the config file for changes
pub fn start_config_watcher(app_handle: AppHandle) -> Result<RecommendedWatcher, Error> {
    let config_dir = storage::get_config_dir()?;

    let watcher_config = Config::default().with_poll_interval(Duration::from_secs(1));

    let watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                // Check if the event is related to our config file
                let config_file_name = std::path::Path::new("config.json");
                let is_config_file = event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == config_file_name.file_name());

                if is_config_file {
                    match event.kind {
                        notify::EventKind::Modify(_) | notify::EventKind::Create(_) => {
                            // Debounce: wait a bit to ensure file write is complete
                            std::thread::sleep(Duration::from_millis(100));

                            // Reload config
                            match storage::load_config_cli() {
                                Ok(new_config) => {
                                    // Update the app state with new config
                                    let app_handle = app_handle.clone();
                                    if let Some(state) =
                                        app_handle.try_state::<Arc<crate::state::AppState>>()
                                    {
                                        if let Ok(mut config) = state.config.lock() {
                                            *config = new_config.clone();
                                        }
                                    }

                                    // Emit event to frontend
                                    let _ = app_handle.emit("config-reloaded", new_config);
                                }
                                Err(e) => {
                                    eprintln!("Failed to reload config: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        },
        watcher_config,
    )
    .map_err(|e| Error::StorageError(format!("Failed to create file watcher: {}", e)))?;

    let mut watcher = watcher;
    watcher
        .watch(&config_dir, RecursiveMode::NonRecursive)
        .map_err(|e| Error::StorageError(format!("Failed to watch config directory: {}", e)))?;

    Ok(watcher)
}
