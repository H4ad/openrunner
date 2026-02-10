use crate::cli::detector::ProjectTemplate;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Detect package.json scripts
pub fn detect_package_json(directory: &Path) -> Option<Vec<ProjectTemplate>> {
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
