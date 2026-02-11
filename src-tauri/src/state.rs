use crate::database::Database;
use crate::models::{AppConfig, ProcessInfo};
use crate::platform::{create_platform_manager, PlatformProcessManager};
use portable_pty::MasterPty;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::process::Child;

pub struct ManagedProcess {
    pub child: Child,
    pub manually_stopped: bool,
    pub session_id: Option<String>,
    /// Group ID this process belongs to
    pub group_id: String,
    /// PTY master for interactive processes (optional)
    pub pty_master: Option<Box<dyn MasterPty + Send>>,
    /// PTY writer for sending input to interactive processes (optional)
    pub pty_writer: Option<Box<dyn Write + Send>>,
    /// Whether this process uses PTY for interactive mode
    pub is_interactive: bool,
    /// The actual process ID (for PTY processes, this is different from child.id())
    pub real_pid: Option<u32>,
}

/// Size of the PTY terminal (rows x columns)
#[derive(Debug, Clone, Copy, Default)]
pub struct PtyDimensions {
    pub rows: u16,
    pub cols: u16,
}

impl PtyDimensions {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { rows, cols }
    }
}

pub struct AppState {
    /// In-memory configuration
    pub config: Mutex<AppConfig>,
    /// Unified database for all data
    pub database: Mutex<Database>,
    /// Path to the database file
    pub database_path: PathBuf,
    pub processes: Mutex<HashMap<String, ManagedProcess>>,
    pub process_infos: Mutex<HashMap<String, ProcessInfo>>,
    pub log_dir: PathBuf,
    /// Maps project_id to active session_id
    pub active_sessions: Mutex<HashMap<String, String>>,
    /// Path to the PID file for tracking running processes
    pub pid_file_path: PathBuf,
    /// Platform-specific process manager
    platform_manager: Box<dyn PlatformProcessManager>,
    /// YAML file watcher
    pub yaml_watcher: Mutex<crate::file_watcher::YamlWatcher>,
    /// Configuration directory path
    pub config_dir: PathBuf,
}

impl AppState {
    pub fn new(config: AppConfig, log_dir: PathBuf, config_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&log_dir);
        let pid_file_path = config_dir.join("running_pids.txt");
        let platform_manager = Box::new(create_platform_manager());

        // Initialize unified database
        let database_path = config_dir.join("runner-ui.db");
        let database = Database::open(&database_path).expect("Failed to open database");

        Self {
            config: Mutex::new(config),
            database: Mutex::new(database),
            database_path: database_path.clone(),
            processes: Mutex::new(HashMap::new()),
            process_infos: Mutex::new(HashMap::new()),
            log_dir,
            active_sessions: Mutex::new(HashMap::new()),
            pid_file_path,
            platform_manager,
            yaml_watcher: Mutex::new(crate::file_watcher::YamlWatcher::new()),
            config_dir,
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
    #[allow(dead_code)]
    pub fn platform(&self) -> &dyn PlatformProcessManager {
        self.platform_manager.as_ref()
    }

    /// Get the database (for convenience)
    pub fn db(&self) -> std::sync::MutexGuard<'_, Database> {
        self.database.lock().unwrap()
    }

    /// Get the database path
    pub fn db_path(&self) -> &PathBuf {
        &self.database_path
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        // Last-resort cleanup: kill all processes
        if let Ok(processes) = self.processes.lock() {
            for managed in processes.values() {
                let pid_to_kill = if managed.is_interactive {
                    managed.real_pid
                } else {
                    managed.child.id()
                };

                if let Some(pid) = pid_to_kill {
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
