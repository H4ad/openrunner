use crate::models::{AppConfig, ProcessInfo};
use crate::platform::{create_platform_manager, PlatformProcessManager};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::process::Child;

pub struct ManagedProcess {
    pub child: Child,
    pub manually_stopped: bool,
    pub session_id: Option<String>,
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub processes: Mutex<HashMap<String, ManagedProcess>>,
    pub process_infos: Mutex<HashMap<String, ProcessInfo>>,
    pub log_dir: PathBuf,
    pub db: Mutex<rusqlite::Connection>,
    /// Maps project_id to active session_id
    pub active_sessions: Mutex<HashMap<String, String>>,
    /// Path to the PID file for tracking running processes
    pub pid_file_path: PathBuf,
    /// Platform-specific process manager
    platform_manager: Box<dyn PlatformProcessManager>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        log_dir: PathBuf,
        db: rusqlite::Connection,
        data_dir: PathBuf,
    ) -> Self {
        let _ = std::fs::create_dir_all(&log_dir);
        let pid_file_path = data_dir.join("running_pids.txt");
        let platform_manager = Box::new(create_platform_manager());

        Self {
            config: Mutex::new(config),
            processes: Mutex::new(HashMap::new()),
            process_infos: Mutex::new(HashMap::new()),
            log_dir,
            db: Mutex::new(db),
            active_sessions: Mutex::new(HashMap::new()),
            pid_file_path,
            platform_manager,
        }
    }

    pub fn log_file_path(&self, project_id: &str) -> PathBuf {
        self.log_dir.join(format!("{}.log", project_id))
    }

    /// Save a process ID to the PID file
    pub fn save_pid(&self, pid: u32) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.pid_file_path)
        {
            let _ = writeln!(file, "{}", pid);
        }
    }

    /// Remove a process ID from the PID file
    pub fn remove_pid(&self, pid: u32) {
        if let Ok(content) = std::fs::read_to_string(&self.pid_file_path) {
            let remaining: Vec<String> = content
                .lines()
                .filter(|line| line.trim().parse::<u32>().ok() != Some(pid))
                .map(|s| s.to_string())
                .collect();
            let _ = std::fs::write(&self.pid_file_path, remaining.join("\n"));
        }
    }

    /// Read all stored PIDs from the PID file
    pub fn read_stored_pids(&self) -> Vec<u32> {
        if let Ok(file) = std::fs::File::open(&self.pid_file_path) {
            let reader = std::io::BufReader::new(file);
            reader
                .lines()
                .filter_map(|line| line.ok()?.trim().parse::<u32>().ok())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Clear the PID file
    pub fn clear_pid_file(&self) {
        let _ = std::fs::write(&self.pid_file_path, "");
    }

    /// Get platform manager
    pub fn platform(&self) -> &dyn PlatformProcessManager {
        self.platform_manager.as_ref()
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        // Last-resort cleanup: kill all processes
        // This runs when AppState is dropped (e.g., during panic or abnormal shutdown)

        // First, kill any processes we're currently managing
        if let Ok(processes) = self.processes.lock() {
            for managed in processes.values() {
                if let Some(pid) = managed.child.id() {
                    self.platform_manager.force_kill(pid);
                }
            }
        }

        // Also kill any orphaned processes from the PID file
        for pid in self.read_stored_pids() {
            self.platform_manager.force_kill(pid);
        }
        self.clear_pid_file();
    }
}
