// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Check if running in CLI mode
    if runner_ui_lib::cli::run_cli() {
        // CLI command executed, exit
        return;
    }

    // Load settings to check GPU optimization before Tauri setup
    let settings = runner_ui_lib::storage::load_settings_cli().unwrap_or_default();

    if settings.linux_gpu_optimization.unwrap_or(true) {
        #[cfg(target_os = "linux")]
        {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            std::env::set_var("NODEVICE_SELECT", "1");
        }
    }

    // Run GUI mode
    runner_ui_lib::run()
}
