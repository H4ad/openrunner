use crate::cli::detector::ProjectTemplate;
use std::path::Path;

/// Detect common config files
pub fn detect_common_configs(directory: &Path) -> Vec<ProjectTemplate> {
    let mut projects = Vec::new();

    // Docker Compose
    let docker_compose_files = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    for file in &docker_compose_files {
        if directory.join(file).exists() {
            projects.push(ProjectTemplate {
                name: "docker: compose up".to_string(),
                command: format!("docker compose -f {} up", file),
                description: "Start Docker Compose services".to_string(),
            });
            break;
        }
    }

    // Justfile
    if directory.join("justfile").exists() || directory.join("Justfile").exists() {
        projects.push(ProjectTemplate {
            name: "just: list".to_string(),
            command: "just --list".to_string(),
            description: "List just recipes".to_string(),
        });
    }

    // Taskfile
    if directory.join("Taskfile.yml").exists() || directory.join("Taskfile.yaml").exists() {
        projects.push(ProjectTemplate {
            name: "task: list".to_string(),
            command: "task --list".to_string(),
            description: "List task targets".to_string(),
        });
    }

    projects
}
