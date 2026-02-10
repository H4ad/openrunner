use crate::cli::detector::{detect_projects, ProjectTemplate};
use crate::cli::ui::{prompt_confirmation, prompt_project_selection, show_preview};
use crate::models::{Group, Project, ProjectType};
use crate::storage;
use crate::yaml_config;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Execute the new command - create group with auto-detected projects
pub fn execute_new(
    directory: PathBuf,
    name: Option<String>,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve directory path
    let directory = directory.canonicalize()?;

    // Check for openrunner.yaml file first
    if let Some(yaml_path) = yaml_config::find_yaml_file(&directory) {
        println!("Found YAML config: {}", yaml_path.display());
        println!("Importing group from YAML (bypassing auto-detection)...");

        let yaml_group = yaml_config::parse_yaml(&yaml_path)?;

        // Get group name from YAML or CLI argument
        let group_name = name.unwrap_or_else(|| yaml_group.name.clone());

        if dry_run {
            println!("\n[DRY RUN] Group to be created from YAML:");
            println!("  Name: {}", group_name);
            println!("  Directory: {}", directory.display());
            println!("  Projects: {}", yaml_group.projects.len());
            for project in &yaml_group.projects {
                println!("    - {}: {}", project.name, project.command);
            }
            println!("\nNo changes were made.");
            return Ok(());
        }

        // Create group with YAML data
        let group_id = Uuid::new_v4().to_string();
        let directory_str = directory.to_string_lossy().to_string();
        let yaml_path_str = yaml_path.to_string_lossy().to_string();

        let projects: Vec<Project> = yaml_group
            .projects
            .into_iter()
            .map(|yaml_project| Project {
                id: Uuid::new_v4().to_string(),
                name: yaml_project.name,
                command: yaml_project.command,
                auto_restart: yaml_project.auto_restart.unwrap_or(true),
                env_vars: yaml_project.env_vars.unwrap_or_default(),
                cwd: Some(yaml_project.cwd.unwrap_or_else(|| directory_str.clone())),
                project_type: yaml_project.project_type.into(),
                interactive: yaml_project.interactive.unwrap_or(false),
            })
            .collect();

        let projects_count = projects.len();

        let new_group = Group {
            id: group_id,
            name: group_name.clone(),
            directory: directory_str,
            projects,
            env_vars: yaml_group.env_vars.unwrap_or_default(),
            sync_file: Some(yaml_path_str),
            sync_enabled: true,
        };

        // Load existing config and add the new group
        let mut config = storage::load_config_cli()?;
        config.groups.push(new_group);
        storage::save_config_cli(&config)?;

        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║       GROUP CREATED SUCCESSFULLY FROM YAML!            ║");
        println!("╚════════════════════════════════════════════════════════╝");
        println!("  Group: {}", group_name);
        println!("  Projects: {}", projects_count);
        println!("  Config: ~/.config/openrunner/config.json");
        println!("  Sync: {} (auto-sync enabled)", yaml_path.display());
        println!("\n  You can now open OpenRunner to manage this group.");
        println!("  Changes will sync bidirectionally between the app and YAML file.");

        return Ok(());
    }

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
        println!("  - openrunner.yaml (for manual configuration)");
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
        sync_file: None,
        sync_enabled: false,
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
