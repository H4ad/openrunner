use crate::cli::detector::ProjectTemplate;
use std::fs;
use std::path::Path;

/// Detect Makefile targets
pub fn detect_makefile(directory: &Path) -> Option<Vec<ProjectTemplate>> {
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
