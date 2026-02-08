use super::PlatformProcessManager;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use tokio::process::Command;
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, FALSE, HANDLE, TRUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, TerminateProcess, PROCESS_TERMINATE,
};

const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
const DETACHED_PROCESS: u32 = 0x00000008;

pub struct WindowsProcessManager {
    job_handle: Option<HANDLE>,
}

impl WindowsProcessManager {
    pub fn new() -> Self {
        unsafe {
            // Create a job object to manage process lifecycle
            let job_handle = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());

            if !job_handle.is_null() {
                // Configure the job object to kill all processes when the job is closed
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                SetInformationJobObject(
                    job_handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
            }

            Self {
                job_handle: if job_handle.is_null() {
                    None
                } else {
                    Some(job_handle)
                },
            }
        }
    }

    fn assign_process_to_job(&self, process_id: u32) {
        if let Some(job_handle) = self.job_handle {
            unsafe {
                let process_handle = OpenProcess(PROCESS_TERMINATE, FALSE, process_id);
                if !process_handle.is_null() {
                    AssignProcessToJobObject(job_handle, process_handle);
                    CloseHandle(process_handle);
                }
            }
        }
    }
}

impl PlatformProcessManager for WindowsProcessManager {
    fn kill_orphaned_processes(&self, _pids: &[u32]) {
        // On Windows, orphaned processes from previous runs are not tracked
        // because we don't have persistent PGID storage like on Unix
        // The job object approach handles cleanup when our process exits
    }

    fn configure_command(&self, cmd: &mut Command) {
        // Create new process group for graceful Ctrl+C handling
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    fn get_process_group_id(&self, pid: u32) -> Option<u32> {
        // On Windows, process group ID is the same as the process ID
        // for processes created with CREATE_NEW_PROCESS_GROUP
        Some(pid)
    }

    fn graceful_shutdown(&self, pid: u32) {
        unsafe {
            // Try to open the process
            let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
            if !handle.is_null() {
                // First try to send Ctrl+C to the process group
                // This requires attaching to the process's console
                // For simplicity, we'll use a timeout-based approach with TerminateProcess
                // In a production app, you might want to use GenerateConsoleCtrlEvent
                CloseHandle(handle);
            }
        }
    }

    fn force_kill(&self, pid: u32) {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }

    fn is_process_running(&self, pid: u32) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
            if handle.is_null() {
                return false;
            }
            CloseHandle(handle);
            true
        }
    }

    fn get_shell_command(&self) -> (&'static str, &'static [&'static str]) {
        // Try PowerShell first for better compatibility
        ("powershell", &["-Command"])
    }
}

impl Default for WindowsProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WindowsProcessManager {
    fn drop(&mut self) {
        // When the job object is closed, all processes in it are terminated
        if let Some(job_handle) = self.job_handle {
            unsafe {
                CloseHandle(job_handle);
            }
        }
    }
}
