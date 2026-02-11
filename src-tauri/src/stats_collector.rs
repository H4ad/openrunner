use crate::models::{ProcessInfo, ProcessStatus};
use crate::state::AppState;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};

/// Read private memory (RSS minus shared pages) from /proc/[pid]/statm.
/// This avoids the double-counting problem where shared library pages are included
/// in each process's RSS, causing summed RSS across a process tree to be wildly inflated.
#[cfg(target_os = "linux")]
fn read_private_memory(pid: Pid) -> Option<u64> {
    let content = std::fs::read_to_string(format!("/proc/{}/statm", pid)).ok()?;
    let mut fields = content.split_whitespace();
    let _size = fields.next()?;
    let resident: u64 = fields.next()?.parse().ok()?;
    let shared: u64 = fields.next()?.parse().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        return None;
    }
    Some(resident.saturating_sub(shared) * page_size as u64)
}

/// Check if a PID is a thread (not a process/thread-group leader).
/// On Linux, threads appear as top-level /proc entries and sysinfo treats them
/// as separate processes. Threads share their leader's address space, so their
/// memory must not be counted separately.
#[cfg(target_os = "linux")]
fn is_thread(pid: Pid) -> bool {
    let Ok(content) = std::fs::read_to_string(format!("/proc/{}/status", pid)) else {
        return false;
    };
    let mut tgid: Option<u64> = None;
    let mut pid_val: Option<u64> = None;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("Tgid:\t") {
            tgid = val.trim().parse().ok();
        } else if let Some(val) = line.strip_prefix("Pid:\t") {
            pid_val = val.trim().parse().ok();
        }
        if tgid.is_some() && pid_val.is_some() {
            break;
        }
    }
    matches!((tgid, pid_val), (Some(t), Some(p)) if t != p)
}

/// Aggregate CPU and memory for a process and all its descendants.
/// CPU is normalized by the number of logical CPUs (so 0-100% = total system capacity).
/// On Linux, threads are included for CPU (since /proc/[pid]/stat only has per-thread CPU)
/// but excluded from memory (threads share the address space with their leader).
fn aggregate_process_tree(sys: &System, root_pid: Pid, num_cpus: f32, _total_memory: u64) -> (f32, u64) {
    let mut total_cpu = 0.0f32;
    let mut total_mem = 0u64;

    // BFS: collect root + all descendants (including threads for CPU)
    let mut tree_pids = vec![root_pid];
    let mut i = 0;
    while i < tree_pids.len() {
        let parent_pid = tree_pids[i];
        if let Some(p) = sys.process(parent_pid) {
            total_cpu += p.cpu_usage();

            // On Linux, skip memory for threads (they share address space with their
            // thread group leader, so counting them would multiply the real value).
            // For process leaders, use private memory to avoid shared-page inflation.
            #[cfg(target_os = "linux")]
            {
                if !is_thread(parent_pid) {
                    total_mem += read_private_memory(parent_pid).unwrap_or(p.memory());
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                total_mem += p.memory();
            }
        }
        for (child_pid, child_proc) in sys.processes() {
            if child_proc.parent() == Some(parent_pid) && !tree_pids.contains(child_pid) {
                tree_pids.push(*child_pid);
            }
        }
        i += 1;
    }

    // Normalize CPU: sysinfo reports per-core %, divide by CPU count for system-wide %
    let normalized_cpu = total_cpu / num_cpus;

    (normalized_cpu, total_mem)
}

pub fn start_collector(app_handle: AppHandle, state: Arc<AppState>) {
    let system = Arc::new(Mutex::new(System::new()));
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get() as f32)
        .unwrap_or(1.0);
    let total_memory = {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.total_memory()
    };

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Collect active PIDs
            let active_pids: Vec<(String, u32)> = {
                let infos = state.process_infos.lock().unwrap();
                infos
                    .values()
                    .filter(|info| info.status == ProcessStatus::Running)
                    .filter_map(|info| info.pid.map(|pid| (info.project_id.clone(), pid)))
                    .collect()
            };

            if active_pids.is_empty() {
                continue;
            }

            // Refresh ALL processes so child/descendant processes are properly detected.
            // Using ProcessesToUpdate::Some only refreshes known PIDs and misses children
            // that weren't previously tracked by sysinfo.
            let updated_infos: Vec<ProcessInfo> = {
                let mut sys = system.lock().unwrap();

                sys.refresh_processes_specifics(
                    ProcessesToUpdate::All,
                    true,
                    ProcessRefreshKind::nothing().with_cpu().with_memory(),
                );

                active_pids
                    .iter()
                    .map(|(project_id, pid)| {
                        let root_pid = Pid::from(*pid as usize);
                        let (cpu, mem) = aggregate_process_tree(&sys, root_pid, num_cpus, total_memory);

                        ProcessInfo {
                            project_id: project_id.clone(),
                            status: ProcessStatus::Running,
                            pid: Some(*pid),
                            cpu_usage: cpu,
                            memory_usage: mem,
                        }
                    })
                    .collect()
            };

            // Update state and emit
            {
                let mut infos = state.process_infos.lock().unwrap();
                for info in &updated_infos {
                    if let Some(existing) = infos.get_mut(&info.project_id) {
                        if existing.status == ProcessStatus::Running {
                            existing.cpu_usage = info.cpu_usage;
                            existing.memory_usage = info.memory_usage;
                        }
                    }
                }
            }

            // Write metrics to SQLite
            {
                let _timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                // TODO: fix to use group_db_manager - need group_id for each session
                // if let Ok(sessions) = state.active_sessions.lock() {
                //     for info in &updated_infos {
                //         if let Some(session_id) = sessions.get(&info.project_id) {
                //             // Need to get group_id from somewhere
                //         }
                //     }
                // }
            }

            if !updated_infos.is_empty() {
                let _ = app_handle.emit("process-stats-updated", &updated_infos);
            }
        }
    });
}
