use tokio::process::Command;

/// Platform-specific process management trait
pub trait PlatformProcessManager: Send + Sync {
    /// Kill orphaned processes from previous runs
    fn kill_orphaned_processes(&self, pids: &[u32]);

    /// Configure command before spawning (platform-specific setup)
    fn configure_command(&self, cmd: &mut Command);

    /// Get the process group ID from a PID
    fn get_process_group_id(&self, pid: u32) -> Option<u32>;

    /// Send graceful shutdown signal to a process
    fn graceful_shutdown(&self, pid: u32);

    /// Force kill a process
    fn force_kill(&self, pid: u32);

    /// Check if a process is still running
    fn is_process_running(&self, pid: u32) -> bool;

    /// Get the shell command for the platform
    fn get_shell_command(&self) -> (&'static str, &'static [&'static str]);
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxProcessManager as PlatformImpl;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacProcessManager as PlatformImpl;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsProcessManager as PlatformImpl;

/// Create the platform-specific process manager
pub fn create_platform_manager() -> PlatformImpl {
    PlatformImpl::new()
}

/// Get the appropriate shell for the platform
pub fn get_platform_shell() -> (&'static str, &'static [&'static str]) {
    #[cfg(unix)]
    return ("sh", &["-c"]);

    #[cfg(windows)]
    return ("cmd", &["/C"]);
}
