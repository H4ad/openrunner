use std::path::PathBuf;

/// Resolve working directory for a project, handling relative paths
pub fn resolve_working_dir(group_dir: &str, project_cwd: &Option<String>) -> String {
    match project_cwd {
        Some(cwd) if !cwd.is_empty() => {
            let path = PathBuf::from(cwd);
            if path.is_absolute() {
                cwd.clone()
            } else {
                PathBuf::from(group_dir)
                    .join(cwd)
                    .to_string_lossy()
                    .to_string()
            }
        }
        _ => group_dir.to_string(),
    }
}
