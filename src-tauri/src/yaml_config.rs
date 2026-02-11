use crate::models::{Group, Project};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const YAML_FILENAME: &str = "openrunner.yaml";
const YML_FILENAME: &str = "openrunner.yml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YamlConfig {
    pub version: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,
    pub projects: Vec<YamlProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YamlProject {
    pub name: String,
    pub command: String,
    #[serde(rename = "type")]
    pub project_type: YamlProjectType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_restart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YamlProjectType {
    Service,
    Task,
}

impl From<YamlProjectType> for crate::models::ProjectType {
    fn from(yaml_type: YamlProjectType) -> Self {
        match yaml_type {
            YamlProjectType::Service => crate::models::ProjectType::Service,
            YamlProjectType::Task => crate::models::ProjectType::Task,
        }
    }
}

impl From<crate::models::ProjectType> for YamlProjectType {
    fn from(project_type: crate::models::ProjectType) -> Self {
        match project_type {
            crate::models::ProjectType::Service => YamlProjectType::Service,
            crate::models::ProjectType::Task => YamlProjectType::Task,
        }
    }
}

pub fn find_yaml_file(directory: &Path) -> Option<PathBuf> {
    let yaml_path = directory.join(YAML_FILENAME);
    if yaml_path.exists() {
        return Some(yaml_path);
    }

    let yml_path = directory.join(YML_FILENAME);
    if yml_path.exists() {
        return Some(yml_path);
    }

    None
}

pub fn get_yaml_path(directory: &Path) -> PathBuf {
    let yaml_path = directory.join(YAML_FILENAME);
    if yaml_path.exists() {
        return yaml_path;
    }
    directory.join(YML_FILENAME)
}

pub fn parse_yaml(path: &Path) -> Result<YamlConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read YAML file: {}", e))?;

    let config: YamlConfig =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML file: {}", e))?;

    Ok(config)
}

pub fn write_yaml(group: &Group, path: &Path) -> Result<(), String> {
    let yaml_projects: Vec<YamlProject> = group
        .projects
        .iter()
        .map(|p| YamlProject {
            name: p.name.clone(),
            command: p.command.clone(),
            project_type: p.project_type.into(),
            auto_restart: if p.auto_restart { Some(true) } else { None },
            cwd: p.cwd.clone(),
            interactive: if p.interactive { Some(true) } else { None },
            env_vars: if p.env_vars.is_empty() {
                None
            } else {
                Some(p.env_vars.clone())
            },
        })
        .collect();

    let yaml_config = YamlConfig {
        version: "1.0".to_string(),
        name: group.name.clone(),
        env_vars: if group.env_vars.is_empty() {
            None
        } else {
            Some(group.env_vars.clone())
        },
        projects: yaml_projects,
    };

    let yaml_content = serde_yaml::to_string(&yaml_config)
        .map_err(|e| format!("Failed to serialize YAML: {}", e))?;

    std::fs::write(path, yaml_content).map_err(|e| format!("Failed to write YAML file: {}", e))?;

    Ok(())
}

pub fn yaml_to_group(
    config: &YamlConfig,
    directory: &std::path::Path,
    sync_file: &std::path::Path,
) -> Group {
    let directory_str = directory.to_string_lossy().to_string();
    let sync_file_str = sync_file.to_string_lossy().to_string();
    group_from_yaml(config.clone(), directory_str, sync_file_str)
}

pub fn yaml_to_project(
    yaml_project: &YamlProject,
    _base_dir: &std::path::Path,
) -> crate::models::Project {
    crate::models::Project {
        id: uuid::Uuid::new_v4().to_string(),
        name: yaml_project.name.clone(),
        command: yaml_project.command.clone(),
        project_type: yaml_project.project_type.clone().into(),
        auto_restart: yaml_project.auto_restart.unwrap_or(true),
        cwd: yaml_project.cwd.clone(),
        interactive: yaml_project.interactive.unwrap_or(false),
        env_vars: yaml_project.env_vars.clone().unwrap_or_default(),
    }
}

pub fn group_from_yaml(config: YamlConfig, directory: String, sync_file: String) -> Group {
    let projects: Vec<Project> = config
        .projects
        .into_iter()
        .map(|p| Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: p.name,
            command: p.command,
            project_type: p.project_type.into(),
            auto_restart: p.auto_restart.unwrap_or(true),
            cwd: p.cwd,
            interactive: p.interactive.unwrap_or(false),
            env_vars: p.env_vars.unwrap_or_default(),
        })
        .collect();

    Group {
        id: uuid::Uuid::new_v4().to_string(),
        name: config.name,
        directory,
        projects,
        env_vars: config.env_vars.unwrap_or_default(),
        sync_file: Some(sync_file),
        sync_enabled: true,
    }
}

pub fn sync_yaml(group: &Group, state: &crate::state::AppState) -> Result<(), String> {
    if !group.sync_enabled {
        return Ok(());
    }

    if let Some(ref yaml_path) = group.sync_file {
        let path = Path::new(yaml_path);
        write_yaml(group, path)?;

        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.update_yaml_timestamp(yaml_path);
        }
    }
    Ok(())
}

pub fn add_project_to_yaml(
    group: &Group,
    _project: &Project,
    state: &crate::state::AppState,
) -> Result<(), String> {
    if !group.sync_enabled {
        return Ok(());
    }

    if let Some(ref yaml_path) = group.sync_file {
        let path = Path::new(yaml_path);
        write_yaml(group, path)?;

        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.update_yaml_timestamp(yaml_path);
        }
    }
    Ok(())
}

pub fn update_project_in_yaml(
    group: &Group,
    _project: &Project,
    state: &crate::state::AppState,
) -> Result<(), String> {
    if !group.sync_enabled {
        return Ok(());
    }

    if let Some(ref yaml_path) = group.sync_file {
        let path = Path::new(yaml_path);
        write_yaml(group, path)?;

        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.update_yaml_timestamp(yaml_path);
        }
    }
    Ok(())
}

pub fn remove_project_from_yaml(
    group: &Group,
    _project_id: &str,
    state: &crate::state::AppState,
) -> Result<(), String> {
    if !group.sync_enabled {
        return Ok(());
    }

    if let Some(ref yaml_path) = group.sync_file {
        let path = Path::new(yaml_path);
        write_yaml(group, path)?;

        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.update_yaml_timestamp(yaml_path);
        }
    }
    Ok(())
}

pub fn update_group_in_yaml(group: &Group, state: &crate::state::AppState) -> Result<(), String> {
    if !group.sync_enabled {
        return Ok(());
    }

    if let Some(ref yaml_path) = group.sync_file {
        let path = Path::new(yaml_path);
        write_yaml(group, path)?;

        if let Ok(watcher) = state.yaml_watcher.lock() {
            let _ = watcher.update_yaml_timestamp(yaml_path);
        }
    }
    Ok(())
}
