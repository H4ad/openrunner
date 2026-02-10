use crate::error::Error;
use crate::models::Group;
use crate::yaml_config;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Debounce duration to ignore events after our own writes (in milliseconds)
const DEBOUNCE_MS: u64 = 500;

/// Manages file watchers for YAML configuration files
pub struct YamlWatcher {
    watchers: Arc<Mutex<HashMap<String, RecommendedWatcher>>>,
    /// Tracks the last time we wrote to each YAML file to prevent self-triggering
    last_write_times: Arc<Mutex<HashMap<String, Instant>>>,
}

impl YamlWatcher {
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
            last_write_times: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Update the timestamp for a YAML file to prevent self-triggering
    pub fn update_yaml_timestamp(&self, yaml_path: &str) -> Result<(), Error> {
        let mut times = self.last_write_times.lock().unwrap();
        times.insert(yaml_path.to_string(), Instant::now());
        Ok(())
    }

    /// Check if we should ignore an event (because we recently wrote to the file)
    fn should_ignore_event(
        last_write_times: &Arc<Mutex<HashMap<String, Instant>>>,
        yaml_path: &str,
    ) -> bool {
        let times = last_write_times.lock().unwrap();
        if let Some(last_write) = times.get(yaml_path) {
            last_write.elapsed() < Duration::from_millis(DEBOUNCE_MS)
        } else {
            false
        }
    }

    /// Start watching a group's YAML file
    pub fn watch_group(&self, app_handle: AppHandle, group: &Group) -> Result<(), Error> {
        // Only watch if sync is enabled and sync_file exists
        if !group.sync_enabled {
            return Ok(());
        }

        if let Some(ref sync_file) = group.sync_file {
            let path = Path::new(sync_file);
            let group_id = group.id.clone();
            let sync_file_clone = sync_file.clone();
            let watched_file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Create a watcher
            let watchers = self.watchers.clone();
            let last_write_times = self.last_write_times.clone();
            let app_handle_clone = app_handle.clone();

            let watcher_result =
                notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                    match res {
                        Ok(event) => {
                            // Only react to modify events for our specific file
                            if event.kind.is_modify() {
                                // Check if the event is for our specific YAML file
                                let is_our_file = event.paths.iter().any(|p| {
                                    p.file_name()
                                        .map(|n| n.to_string_lossy() == watched_file_name)
                                        .unwrap_or(false)
                                });

                                if is_our_file {
                                    // Check if we should ignore this event (self-triggered)
                                    if Self::should_ignore_event(
                                        &last_write_times,
                                        &sync_file_clone,
                                    ) {
                                        return;
                                    }

                                    // Emit event to frontend
                                    let _ = app_handle_clone.emit(
                                        "yaml-file-changed",
                                        serde_json::json!({
                                            "groupId": &group_id,
                                            "filePath": &sync_file_clone,
                                        }),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("File watcher error: {:?}", e);
                        }
                    }
                });

            match watcher_result {
                Ok(mut watcher) => {
                    if let Err(e) = watcher.watch(
                        path.parent().unwrap_or(Path::new("/")),
                        RecursiveMode::NonRecursive,
                    ) {
                        return Err(Error::StorageError(format!("Failed to watch file: {}", e)));
                    }

                    // Store the watcher
                    let mut watchers_guard = watchers.lock().unwrap();
                    watchers_guard.insert(group.id.clone(), watcher);

                    Ok(())
                }
                Err(e) => Err(Error::StorageError(format!(
                    "Failed to create file watcher: {}",
                    e
                ))),
            }
        } else {
            Ok(())
        }
    }

    /// Stop watching a group's YAML file
    pub fn unwatch_group(&self, group_id: &str) -> Result<(), Error> {
        let mut watchers = self.watchers.lock().unwrap();
        if watchers.remove(group_id).is_some() {
            Ok(())
        } else {
            // Not an error if it wasn't being watched
            Ok(())
        }
    }

    /// Update watchers for all groups in config
    pub fn sync_watchers(&self, app_handle: AppHandle, groups: &[Group]) -> Result<(), Error> {
        // Get current watched group IDs
        let current_ids: Vec<String> = {
            let watchers = self.watchers.lock().unwrap();
            watchers.keys().cloned().collect()
        };

        // Get group IDs that should be watched (must have sync_file AND sync_enabled)
        let group_ids_with_sync: Vec<String> = groups
            .iter()
            .filter(|g| g.sync_file.is_some() && g.sync_enabled)
            .map(|g| g.id.clone())
            .collect();

        // Unwatch groups that no longer have sync enabled
        for group_id in &current_ids {
            if !group_ids_with_sync.contains(group_id) {
                self.unwatch_group(group_id)?;
            }
        }

        // Watch new groups with sync enabled
        for group in groups {
            if group.sync_file.is_some() && group.sync_enabled && !current_ids.contains(&group.id) {
                self.watch_group(app_handle.clone(), group)?;
            }
        }

        Ok(())
    }

    /// Reload a group's configuration from its YAML file
    pub fn reload_group_from_yaml(&self, group: &mut Group) -> Result<(), Error> {
        if let Some(ref sync_file) = group.sync_file {
            let path = Path::new(sync_file);
            if path.exists() {
                let yaml_config =
                    yaml_config::parse_yaml(path).map_err(|e| Error::YamlConfig(e))?;
                let base_dir = Path::new(&group.directory);

                // Update group properties from YAML
                group.name = yaml_config.name;
                group.env_vars = yaml_config.env_vars.unwrap_or_default();

                // Recreate projects from YAML (preserving IDs if names match)
                let existing_projects: HashMap<String, String> = group
                    .projects
                    .iter()
                    .map(|p| (p.name.clone(), p.id.clone()))
                    .collect();

                group.projects = yaml_config
                    .projects
                    .iter()
                    .map(|yp| {
                        let mut project = yaml_config::yaml_to_project(yp, base_dir);
                        // Preserve ID if project with same name exists
                        if let Some(id) = existing_projects.get(&project.name) {
                            project.id = id.clone();
                        }
                        project
                    })
                    .collect();

                Ok(())
            } else {
                Err(Error::StorageError(format!(
                    "YAML file no longer exists: {}",
                    sync_file
                )))
            }
        } else {
            Err(Error::StorageError("Group has no sync file".to_string()))
        }
    }
}

impl Default for YamlWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle type for YAML watchers stored in AppState
pub type YamlWatcherHandle = RecommendedWatcher;
