use crate::cli::detector::ProjectTemplate;
use std::fs;
use std::path::Path;

/// Detect Docker projects
pub fn detect_docker(directory: &Path) -> Option<Vec<ProjectTemplate>> {
    let dockerfile_path = directory.join("Dockerfile");

    if !dockerfile_path.exists() {
        return None;
    }

    let projects = vec![ProjectTemplate {
        name: "docker: build".to_string(),
        command: "docker build -t $(basename $(pwd)) .".to_string(),
        description: "Build Docker image".to_string(),
    }];

    Some(projects)
}

/// Detect Python projects
pub fn detect_python(directory: &Path) -> Vec<ProjectTemplate> {
    let mut projects = Vec::new();

    // Check for requirements.txt
    if directory.join("requirements.txt").exists() {
        projects.push(ProjectTemplate {
            name: "pip: install".to_string(),
            command: "pip install -r requirements.txt".to_string(),
            description: "Install Python dependencies".to_string(),
        });
    }

    // Check for poetry
    if directory.join("pyproject.toml").exists() {
        projects.push(ProjectTemplate {
            name: "poetry: install".to_string(),
            command: "poetry install".to_string(),
            description: "Install Poetry dependencies".to_string(),
        });
    }

    // Check for pipenv
    if directory.join("Pipfile").exists() {
        projects.push(ProjectTemplate {
            name: "pipenv: install".to_string(),
            command: "pipenv install".to_string(),
            description: "Install Pipenv dependencies".to_string(),
        });
    }

    projects
}

/// Detect Rust projects
pub fn detect_rust(directory: &Path) -> Vec<ProjectTemplate> {
    let mut projects = Vec::new();

    if directory.join("Cargo.toml").exists() {
        projects.push(ProjectTemplate {
            name: "cargo: build".to_string(),
            command: "cargo build".to_string(),
            description: "Build Rust project".to_string(),
        });

        projects.push(ProjectTemplate {
            name: "cargo: test".to_string(),
            command: "cargo test".to_string(),
            description: "Run Rust tests".to_string(),
        });

        // Check if it's a workspace
        if let Ok(content) = fs::read_to_string(directory.join("Cargo.toml")) {
            if content.contains("[workspace]") {
                projects.push(ProjectTemplate {
                    name: "cargo: build --workspace".to_string(),
                    command: "cargo build --workspace".to_string(),
                    description: "Build entire workspace".to_string(),
                });
            }
        }
    }

    projects
}

/// Detect Go projects
pub fn detect_go(directory: &Path) -> Vec<ProjectTemplate> {
    let mut projects = Vec::new();

    if directory.join("go.mod").exists() {
        projects.push(ProjectTemplate {
            name: "go: build".to_string(),
            command: "go build ./...".to_string(),
            description: "Build Go project".to_string(),
        });

        projects.push(ProjectTemplate {
            name: "go: test".to_string(),
            command: "go test ./...".to_string(),
            description: "Run Go tests".to_string(),
        });

        projects.push(ProjectTemplate {
            name: "go: run".to_string(),
            command: "go run .".to_string(),
            description: "Run Go application".to_string(),
        });
    }

    projects
}
