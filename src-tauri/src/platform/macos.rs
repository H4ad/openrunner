use super::PlatformProcessManager;
use libc;
use tokio::process::Command;

pub struct MacProcessManager;

impl MacProcessManager {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProcessManager for MacProcessManager {
    fn kill_orphaned_processes(&self, pgids: &[u32]) {
        for pgid in pgids {
            // Check if process group still exists
            unsafe {
                // Signal 0 checks if process exists without sending a signal
                if libc::killpg(*pgid as i32, 0) == 0 {
                    // Process group exists, kill it
                    libc::killpg(*pgid as i32, libc::SIGKILL);
                }
            }
        }
    }

    fn configure_command(&self, cmd: &mut Command) {
        unsafe {
            cmd.pre_exec(|| {
                // Set process group so we can kill the entire tree
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }

    fn get_process_group_id(&self, pid: u32) -> Option<u32> {
        // On macOS with setpgid(0, 0), PID == PGID
        Some(pid)
    }

    fn graceful_shutdown(&self, pid: u32) {
        unsafe {
            libc::killpg(pid as i32, libc::SIGTERM);
        }
    }

    fn force_kill(&self, pid: u32) {
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
    }

    fn is_process_running(&self, pid: u32) -> bool {
        unsafe { libc::killpg(pid as i32, 0) == 0 }
    }

    fn get_shell_command(&self) -> (&'static str, &'static [&'static str]) {
        ("sh", &["-c"])
    }
}

impl Default for MacProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
