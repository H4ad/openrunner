use crate::commands::types::{Error, Group, Project, ProjectType};
use crate::state::AppState;
use crate::storage;
use crate::yaml_config::{find_yaml_file, YamlConfig};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn import_group(
    app_handle: AppHandle,
    state: State<'_, Arc<AppState>>,
    file_path: String,
) -> Result<Group, Error> {
    // Read and parse YAML file
    let yaml_content = std::fs::read_to_string(&file_path)
        .map_err(|e| Error::StorageError(format!("Failed to read file: {}", e)))?;

    let yaml_config: YamlConfig = serde_yaml::from_str(&yaml_content)
        .map_err(|e| Error::StorageError(format!("Failed to parse YAML: {}", e)))?;

    // Convert YAML config to Group
    let mut group = Group {
        id: uuid::Uuid::new_v4().to_string(),
        name: yaml_config.name,
        directory: Path::new(&file_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone()),
        projects: yaml_config
            .projects
            .into_iter()
            .map(|p| Project {
                id: uuid::Uuid::new_v4().to_string(),
                name: p.name,
                command: p.command,
                project_type: ProjectType::from(p.project_type),
                auto_restart: p.auto_restart.unwrap_or(true),
                env_vars: p.env_vars.unwrap_or_default(),
                cwd: Some(p.cwd.unwrap_or_default()),
                interactive: p.interactive.unwrap_or(false),
            })
            .collect(),
        env_vars: yaml_config.env_vars.unwrap_or_default(),
        sync_file: Some(file_path),
        sync_enabled: true,
    };

    // Check if directory has YAML and set up sync
    let dir_path = Path::new(&group.directory);
    if let Some(yaml_path) = find_yaml_file(dir_path) {
        group.sync_file = Some(yaml_path.to_string_lossy().to_string());
    }

    {
        let mut config = state.config.lock().unwrap();
        config.groups.push(group.clone());
        storage::save_config(&app_handle, &config)?;
    }

    // Start watching YAML if applicable
    if group.sync_file.is_some() {
        let watcher = state.yaml_watcher.lock().unwrap();
        let _ = watcher.watch_group(app_handle, &group);
    }

    Ok(group)
}
