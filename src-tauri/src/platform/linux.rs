use super::PlatformProcessManager;
use tokio::process::Command;

pub struct LinuxProcessManager;

impl LinuxProcessManager {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProcessManager for LinuxProcessManager {
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
                // Set PDEATHSIG so children get SIGTERM if parent dies
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }

    fn get_process_group_id(&self, pid: u32) -> Option<u32> {
        // On Linux with setpgid(0, 0), PID == PGID
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

impl Default for LinuxProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
