use crate::models::{Group, Project, ProjectType};
use crate::storage;
use dialoguer::MultiSelect;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Project template for detected projects
#[derive(Debug, Clone)]
pub struct ProjectTemplate {
    pub name: String,
    pub command: String,
    pub description: String,
}

/// Detect projects in a given directory
pub fn detect_projects(directory: &Path) -> Vec<ProjectTemplate> {
    let mut projects = Vec::new();

    // Detect package.json scripts
    if let Some(package_projects) = detect_package_json(directory) {
        projects.extend(package_projects);
    }

    // Detect Makefile targets
    if let Some(makefile_projects) = detect_makefile(directory) {
        projects.extend(makefile_projects);
    }

    // Detect common config files
    projects.extend(detect_common_configs(directory));

    // Detect Docker projects
    if let Some(docker_projects) = detect_docker(directory) {
        projects.extend(docker_projects);
    }

    // Detect Python projects
    projects.extend(detect_python(directory));

    // Detect Rust projects
    projects.extend(detect_rust(directory));

    // Detect Go projects
    projects.extend(detect_go(directory));

    projects
}

/// Detect package.json scripts
fn detect_package_json(directory: &Path) -> Option<Vec<ProjectTemplate>> {
    let package_json_path = directory.join("package.json");

    if !package_json_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&package_json_path).ok()?;
    let package: Value = serde_json::from_str(&content).ok()?;
    let scripts = package.get("scripts")?.as_object()?;

    let mut projects = Vec::new();

    for (name, value) in scripts {
        if let Some(cmd) = value.as_str() {
            // Skip common meta scripts that aren't typically run standalone
            if is_ignored_npm_script(name) {
                continue;
            }

            let description = if name == "dev" || name == "start" {
                format!("Development server: {}", cmd)
            } else if name == "build" {
                format!("Build project: {}", cmd)
            } else if name == "test" {
                format!("Run tests: {}", cmd)
            } else if name.starts_with("test:") {
                format!("Test: {}", cmd)
            } else if name == "lint" {
                format!("Lint code: {}", cmd)
            } else {
                format!("Script: {}", cmd)
            };

            projects.push(ProjectTemplate {
                name: format!("npm: {}", name),
                command: format!("npm run {}", name),
                description,
            });
        }
    }

    if projects.is_empty() {
        None
    } else {
        Some(projects)
    }
}

/// Scripts to ignore in auto-detection
fn is_ignored_npm_script(name: &str) -> bool {
    matches!(
        name,
        "prepare"
            | "preinstall"
            | "postinstall"
            | "prepublish"
            | "prepublishOnly"
            | "publish"
            | "postpublish"
            | "prerestart"
            | "restart"
            | "postrestart"
            | "prestop"
            | "stop"
            | "poststop"
            | "prestart"
            | "poststart"
            | "predev"
            | "postdev"
            | "prebuild"
            | "postbuild"
            | "pretest"
            | "posttest"
            | "prelint"
            | "postlint"
    )
}

/// Detect Makefile targets
fn detect_makefile(directory: &Path) -> Option<Vec<ProjectTemplate>> {
    let makefile_names = ["Makefile", "makefile", "GNUmakefile"];
    let mut makefile_path = None;

    for name in &makefile_names {
        let path = directory.join(name);
        if path.exists() {
            makefile_path = Some(path);
            break;
        }
    }

    let makefile_path = makefile_path?;
    let content = fs::read_to_string(&makefile_path).ok()?;

    let mut projects = Vec::new();

    for line in content.lines() {
        // Detect common targets
        if let Some(target) = line.split(':').next() {
            let target = target.trim();

            // Skip special targets and variables
            if target.is_empty()
                || target.starts_with('.')
                || target.starts_with('#')
                || target.contains('=')
                || target.contains('$')
            {
                continue;
            }

            // Common targets to highlight
            let (_priority, description) = match target {
                "dev" | "develop" => (1, "Development server"),
                "start" | "run" | "serve" => (2, "Start application"),
                "build" | "compile" => (3, "Build project"),
                "test" | "tests" => (4, "Run tests"),
                "lint" | "linter" => (5, "Run linter"),
                "clean" => (100, "Clean build artifacts"),
                "install" | "deps" | "dependencies" => (6, "Install dependencies"),
                "watch" => (7, "Watch for changes"),
                "deploy" => (8, "Deploy application"),
                "fmt" | "format" => (9, "Format code"),
                "check" | "verify" => (10, "Verify/Check code"),
                _ => (50, "Makefile target"),
            };

            // Skip if already added
            if projects
                .iter()
                .any(|p: &ProjectTemplate| p.name == format!("make: {}", target))
            {
                continue;
            }

            projects.push(ProjectTemplate {
                name: format!("make: {}", target),
                command: format!("make {}", target),
                description: description.to_string(),
            });
        }
    }

    // Sort by priority
    projects.sort_by_key(|p| match p.name.as_str() {
        n if n.contains("dev") => 1,
        n if n.contains("start") || n.contains("run") || n.contains("serve") => 2,
        n if n.contains("build") => 3,
        n if n.contains("test") => 4,
        n if n.contains("lint") => 5,
        n if n.contains("install") || n.contains("deps") => 6,
        n if n.contains("watch") => 7,
        n if n.contains("deploy") => 8,
        n if n.contains("fmt") || n.contains("format") => 9,
        n if n.contains("check") || n.contains("verify") => 10,
        _ => 50,
    });

    if projects.is_empty() {
        None
    } else {
        Some(projects)
    }
}

/// Detect common config files
fn detect_common_configs(directory: &Path) -> Vec<ProjectTemplate> {
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

/// Detect Docker projects
fn detect_docker(directory: &Path) -> Option<Vec<ProjectTemplate>> {
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
fn detect_python(directory: &Path) -> Vec<ProjectTemplate> {
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
fn detect_rust(directory: &Path) -> Vec<ProjectTemplate> {
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
fn detect_go(directory: &Path) -> Vec<ProjectTemplate> {
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

/// Show preview of what will be created
fn show_preview(group_name: &str, directory: &Path, projects: &[ProjectTemplate]) {
    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║           PREVIEW: Group to be created                 ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!("  Group Name: {}", group_name);
    println!("  Directory:  {}", directory.display());
    println!("  Projects:   {}", projects.len());
    println!("  ───────────────────────────────────────────────────────");

    for (i, project) in projects.iter().enumerate() {
        println!("  {}. {}", i + 1, project.name);
        println!("     └─ {}", project.description);
        println!("     └─ $ {}", project.command);
        if i < projects.len() - 1 {
            println!();
        }
    }
    println!("  ═══════════════════════════════════════════════════════\n");
}

/// Prompt user for project selection using dialoguer
fn prompt_project_selection(
    projects: &[ProjectTemplate],
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    if projects.is_empty() {
        return Ok(Vec::new());
    }

    let items: Vec<String> = projects
        .iter()
        .map(|p| format!("{} - {}", p.name, p.description))
        .collect();

    let defaults: Vec<bool> = vec![true; projects.len()];

    let selection = MultiSelect::new()
        .with_prompt("Select projects to include (use arrow keys to navigate, space to select/deselect, enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    Ok(selection)
}

/// Prompt user for confirmation
fn prompt_confirmation(message: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use dialoguer::Confirm;

    let confirmed = Confirm::new()
        .with_prompt(message)
        .default(true)
        .interact()?;

    Ok(confirmed)
}

/// Execute the new command - create group with auto-detected projects
pub fn execute_new(
    directory: PathBuf,
    name: Option<String>,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve directory path
    let directory = directory.canonicalize()?;

    // Get group name
    let group_name = name.unwrap_or_else(|| {
        directory
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string()
    });

    println!("Scanning directory: {}", directory.display());

    // Detect projects
    let detected_projects = detect_projects(&directory);

    if detected_projects.is_empty() {
        println!("\nNo projects detected in this directory.");
        println!("Supported project files:");
        println!("  - package.json (npm scripts)");
        println!("  - Makefile / makefile");
        println!("  - docker-compose.yml");
        println!("  - Cargo.toml");
        println!("  - go.mod");
        println!("  - pyproject.toml / requirements.txt");
        println!("  - justfile / Justfile");
        println!("  - Taskfile.yml");
        return Ok(());
    }

    println!("Detected {} project(s)", detected_projects.len());

    // Show preview
    show_preview(&group_name, &directory, &detected_projects);

    if dry_run {
        println!("[DRY RUN] No changes were made.");
        return Ok(());
    }

    // Interactive project selection
    let selected_indices = prompt_project_selection(&detected_projects)?;

    if selected_indices.is_empty() {
        println!("No projects selected. Canceling.");
        return Ok(());
    }

    let selected_projects: Vec<&ProjectTemplate> = selected_indices
        .iter()
        .map(|&i| &detected_projects[i])
        .collect();

    println!(
        "\nYou selected {} project(s) to create:",
        selected_projects.len()
    );
    for project in &selected_projects {
        println!("  - {}", project.name);
    }

    // Final confirmation
    if !prompt_confirmation("\nCreate this group with the selected projects?")? {
        println!("Canceled.");
        return Ok(());
    }

    // Create the group and projects
    let group_id = Uuid::new_v4().to_string();
    let directory_str = directory.to_string_lossy().to_string();

    let projects: Vec<Project> = selected_projects
        .iter()
        .map(|template| Project {
            id: Uuid::new_v4().to_string(),
            name: template.name.clone(),
            command: template.command.clone(),
            auto_restart: false,
            env_vars: HashMap::new(),
            cwd: Some(directory_str.clone()),
            project_type: ProjectType::Service,
            interactive: false,
        })
        .collect();

    let new_group = Group {
        id: group_id,
        name: group_name.clone(),
        directory: directory_str,
        projects,
        env_vars: HashMap::new(),
    };

    // Load existing config and add the new group
    let mut config = storage::load_config_cli()?;
    config.groups.push(new_group);
    storage::save_config_cli(&config)?;

    // Success message
    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║              GROUP CREATED SUCCESSFULLY!               ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!("  Group: {}", group_name);
    println!("  Projects: {}", selected_projects.len());
    println!("  Config saved to: ~/.config/openrunner/config.json");
    println!("\n  You can now open OpenRunner to manage this group.");

    Ok(())
}
