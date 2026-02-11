use crate::commands::types::Error;
use crate::state::AppState;
use crate::yaml_config::{YamlConfig, YamlProject, YamlProjectType};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn export_group(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    file_path: String,
) -> Result<(), Error> {
    let config = state.config.lock().unwrap();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        // Convert group to YAML format
        let yaml_config = YamlConfig {
            version: "1.0".to_string(),
            name: group.name.clone(),
            env_vars: Some(group.env_vars.clone()),
            projects: group
                .projects
                .iter()
                .map(|p| YamlProject {
                    name: p.name.clone(),
                    command: p.command.clone(),
                    project_type: YamlProjectType::from(p.project_type),
                    auto_restart: Some(p.auto_restart),
                    env_vars: Some(p.env_vars.clone()),
                    cwd: p.cwd.clone(),
                    interactive: Some(p.interactive),
                })
                .collect(),
        };

        let yaml = serde_yaml::to_string(&yaml_config)
            .map_err(|e| Error::StorageError(format!("Failed to serialize group: {}", e)))?;
        std::fs::write(&file_path, yaml)
            .map_err(|e| Error::StorageError(format!("Failed to write file: {}", e)))?;
        Ok(())
    } else {
        Err(Error::GroupNotFound(group_id))
    }
}
