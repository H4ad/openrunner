/// Project template for detected projects
#[derive(Debug, Clone)]
pub struct ProjectTemplate {
    pub name: String,
    pub command: String,
    pub description: String,
}

mod npm;
mod makefile;
mod configs;
mod languages;

use std::path::Path;

/// Detect projects in a given directory
pub fn detect_projects(directory: &Path) -> Vec<ProjectTemplate> {
    let mut projects = Vec::new();

    // Detect package.json scripts
    if let Some(package_projects) = npm::detect_package_json(directory) {
        projects.extend(package_projects);
    }

    // Detect Makefile targets
    if let Some(makefile_projects) = makefile::detect_makefile(directory) {
        projects.extend(makefile_projects);
    }

    // Detect common config files
    projects.extend(configs::detect_common_configs(directory));

    // Detect Docker projects
    if let Some(docker_projects) = languages::detect_docker(directory) {
        projects.extend(docker_projects);
    }

    // Detect Python projects
    projects.extend(languages::detect_python(directory));

    // Detect Rust projects
    projects.extend(languages::detect_rust(directory));

    // Detect Go projects
    projects.extend(languages::detect_go(directory));

    projects
}
