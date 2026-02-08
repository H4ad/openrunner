// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Check if running in CLI mode
    if runner_ui_lib::cli::run_cli() {
        // CLI command executed, exit
        return;
    }

    // Run GUI mode
    runner_ui_lib::run()
}
