use crate::error::Error;
use std::path::PathBuf;
use std::process::Command;

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
