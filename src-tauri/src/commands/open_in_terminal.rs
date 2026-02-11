use crate::error::Error;
use std::path::PathBuf;

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

        Err(Error::SpawnError(
            "No supported terminal found. Please install kitty, alacritty, wezterm, gnome-terminal, konsole, xfce4-terminal, or xterm.".to_string()
        ))
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
