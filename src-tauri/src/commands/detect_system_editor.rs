#[tauri::command]
pub async fn detect_system_editor() -> String {
    // Try VISUAL first, then EDITOR
    if let Ok(editor) = std::env::var("VISUAL") {
        if !editor.is_empty() {
            return editor;
        }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.is_empty() {
            return editor;
        }
    }

    // Try to find common editors
    let common_editors = [
        "code", "cursor", "subl", "vim", "nvim", "nano", "emacs", "idea", "goland", "webstorm",
        "zed",
    ];

    for editor in common_editors {
        if tokio::process::Command::new("which")
            .arg(editor)
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return editor.to_string();
        }
    }

    String::new()
}
