use crate::error::Error;
use crate::state::AppState;
use crate::storage;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tauri::State;

fn resolve_working_dir(group_dir: &str, project_cwd: &Option<String>) -> String {
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

#[tauri::command]
pub fn resolve_project_working_dir(
    state: State<'_, Arc<AppState>>,
    group_id: String,
    project_id: String,
) -> Result<String, Error> {
    let config = state.config.lock().unwrap();
    let group = config
        .groups
        .iter()
        .find(|g| g.id == group_id)
        .ok_or_else(|| Error::GroupNotFound(group_id.clone()))?;
    let project = group
        .projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id.clone()))?;
    Ok(resolve_working_dir(&group.directory, &project.cwd))
}

#[tauri::command]
pub fn resolve_working_dir_by_project(
    state: State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<String, Error> {
    let config = state.config.lock().unwrap();
    for group in &config.groups {
        if let Some(project) = group.projects.iter().find(|p| p.id == project_id) {
            return Ok(resolve_working_dir(&group.directory, &project.cwd));
        }
    }
    Err(Error::ProjectNotFound(project_id))
}

#[tauri::command]
pub fn open_file_in_editor(
    app_handle: tauri::AppHandle,
    file_path: String,
    line: Option<u32>,
    column: Option<u32>,
    working_dir: String,
) -> Result<(), Error> {
    // Resolve relative paths against working_dir
    let resolved = if PathBuf::from(&file_path).is_absolute() {
        PathBuf::from(&file_path)
    } else {
        PathBuf::from(&working_dir).join(&file_path)
    };

    // Canonicalize to resolve ../ and similar
    let resolved = resolved
        .canonicalize()
        .map_err(|_| Error::FileNotFound(file_path.clone()))?;

    let is_directory = resolved.is_dir();

    if !resolved.exists() {
        return Err(Error::FileNotFound(file_path));
    }

    // For directories, just open in file manager
    if is_directory {
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open")
                .arg(resolved.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open directory: {}", e)))?;
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(resolved.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open directory: {}", e)))?;
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(resolved.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open directory: {}", e)))?;
        }
        return Ok(());
    }

    let resolved_str = resolved.to_string_lossy().to_string();

    // Get configured editor or fall back to environment
    let settings = storage::load_settings(&app_handle).unwrap_or_default();
    let editor = settings
        .editor
        .filter(|e| !e.is_empty())
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_default();

    let editor_name = PathBuf::from(&editor)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let line_num = line.unwrap_or(1);
    let col_num = column.unwrap_or(1);

    let result = match editor_name.as_str() {
        "code" | "code-insiders" | "cursor" => Command::new(&editor)
            .arg("--goto")
            .arg(format!("{}:{}:{}", resolved_str, line_num, col_num))
            .spawn(),
        "zed" => Command::new(&editor)
            .arg(format!("{}:{}:{}", resolved_str, line_num, col_num))
            .spawn(),
        "subl" | "sublime_text" => Command::new(&editor)
            .arg(format!("{}:{}:{}", resolved_str, line_num, col_num))
            .spawn(),
        "vim" | "nvim" | "vi" => Command::new(&editor)
            .arg(format!("+{}", line_num))
            .arg(&resolved_str)
            .spawn(),
        "idea" | "goland" | "webstorm" | "phpstorm" | "pycharm" | "rustrover" | "clion" => {
            Command::new(&editor)
                .arg("--line")
                .arg(line_num.to_string())
                .arg("--column")
                .arg(col_num.to_string())
                .arg(&resolved_str)
                .spawn()
        }
        "emacs" | "emacsclient" => Command::new(&editor)
            .arg(format!("+{}:{}", line_num, col_num))
            .arg(&resolved_str)
            .spawn(),
        _ if !editor.is_empty() => Command::new(&editor).arg(&resolved_str).spawn(),
        _ => {
            // Fallback to platform default
            #[cfg(target_os = "linux")]
            {
                Command::new("xdg-open").arg(&resolved_str).spawn()
            }
            #[cfg(target_os = "macos")]
            {
                Command::new("open").arg(&resolved_str).spawn()
            }
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(["/C", "start", "", &resolved_str])
                    .spawn()
            }
        }
    };

    result.map_err(|e| Error::SpawnError(format!("Failed to open editor: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), Error> {
    let resolved = PathBuf::from(&path);

    if !resolved.exists() {
        return Err(Error::FileNotFound(path));
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| Error::SpawnError(format!("Failed to open path: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| Error::SpawnError(format!("Failed to open path: {}", e)))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| Error::SpawnError(format!("Failed to open path: {}", e)))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_terminal(path: String) -> Result<(), Error> {
    let resolved = PathBuf::from(&path);

    if !resolved.exists() {
        return Err(Error::FileNotFound(path));
    }

    let resolved_str = resolved.to_string_lossy().to_string();

    #[cfg(target_os = "linux")]
    {
        // Try common Linux terminals
        let terminals = [
            ("kitty", vec!["--directory", &resolved_str]),
            ("alacritty", vec!["--working-directory", &resolved_str]),
            ("wezterm", vec!["start", "--cwd", &resolved_str]),
            ("gnome-terminal", vec!["--working-directory", &resolved_str]),
            ("konsole", vec!["--workdir", &resolved_str]),
            ("xfce4-terminal", vec!["--working-directory", &resolved_str]),
            ("mate-terminal", vec!["--working-directory", &resolved_str]),
            ("lxterminal", vec!["--working-directory", &resolved_str]),
            ("terminator", vec!["--working-directory", &resolved_str]),
            ("tilix", vec!["--working-directory", &resolved_str]),
            ("xterm", vec!["-e", "cd", &resolved_str, "&&", "bash"]),
        ];

        for (terminal, args) in &terminals {
            if which::which(terminal).is_ok() {
                let mut cmd = std::process::Command::new(terminal);
                cmd.args(args);
                match cmd.spawn() {
                    Ok(_) => return Ok(()),
                    Err(_) => continue,
                }
            }
        }

        return Err(Error::SpawnError(
            "No supported terminal found. Please install kitty, alacritty, wezterm, gnome-terminal, konsole, xfce4-terminal, or xterm.".to_string()
        ));
    }

    #[cfg(target_os = "macos")]
    {
        // Try common macOS terminals
        // First check for iTerm2
        let iterm_path = "/Applications/iTerm.app";
        if std::path::Path::new(iterm_path).exists() {
            std::process::Command::new("open")
                .args(&["-a", "iTerm", &resolved_str])
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open iTerm: {}", e)))?;
            return Ok(());
        }

        // Check for WezTerm
        let wezterm_path = "/Applications/WezTerm.app";
        if std::path::Path::new(wezterm_path).exists() {
            std::process::Command::new("open")
                .args(&["-a", "WezTerm", &resolved_str])
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open WezTerm: {}", e)))?;
            return Ok(());
        }

        // Check for kitty
        if which::which("kitty").is_ok() {
            std::process::Command::new("kitty")
                .args(&["--directory", &resolved_str])
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open kitty: {}", e)))?;
            return Ok(());
        }

        // Check for alacritty
        if which::which("alacritty").is_ok() {
            std::process::Command::new("alacritty")
                .args(&["--working-directory", &resolved_str])
                .spawn()
                .map_err(|e| Error::SpawnError(format!("Failed to open alacritty: {}", e)))?;
            return Ok(());
        }

        // Fall back to Terminal.app
        std::process::Command::new("open")
            .args(&["-a", "Terminal", &resolved_str])
            .spawn()
            .map_err(|e| Error::SpawnError(format!("Failed to open Terminal: {}", e)))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // Try Windows Terminal first
        if which::which("wt").is_ok() {
            std::process::Command::new("wt")
                .args(&["-d", &resolved_str])
                .spawn()
                .map_err(|e| {
                    Error::SpawnError(format!("Failed to open Windows Terminal: {}", e))
                })?;
            return Ok(());
        }

        // Fall back to cmd.exe
        std::process::Command::new("cmd")
            .args(&["/C", "start", "cmd", "/K", "cd", "/d", &resolved_str])
            .spawn()
            .map_err(|e| Error::SpawnError(format!("Failed to open Command Prompt: {}", e)))?;
        return Ok(());
    }
}
