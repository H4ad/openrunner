/**
 * Stats collector for process CPU/memory monitoring.
 * This is the equivalent of src-tauri/src/stats_collector.rs
 *
 * Note: When spawning processes with shell wrappers (sh -c "command" or cmd /C "command"),
 * the shell process itself uses minimal resources. We use pidtree to find all child
 * processes and aggregate their stats for accurate monitoring.
 */

import type { BrowserWindow } from 'electron';
import pidusage from 'pidusage';
import pidtree from 'pidtree';
import { getState } from './state';
import { IPC_EVENTS } from '../../shared/events';
import type { ProcessInfo } from '../../shared/types';

let statsInterval: NodeJS.Timeout | null = null;
let mainWindowRef: BrowserWindow | null = null;

/**
 * Start collecting stats for all running processes
 * Stats are collected every 2 seconds
 */
export function startStatsCollection(mainWindow: BrowserWindow): void {
  if (statsInterval) {
    return; // Already running
  }

  mainWindowRef = mainWindow;

  statsInterval = setInterval(async () => {
    await collectStats();
  }, 2000);
}

/**
 * Stop stats collection
 */
export function stopStatsCollection(): void {
  if (statsInterval) {
    clearInterval(statsInterval);
    statsInterval = null;
  }
  mainWindowRef = null;
}

/**
 * Collect stats for all running processes
 *
 * Uses pidtree to find all child processes of the shell wrapper and aggregates
 * their CPU and memory usage for accurate monitoring.
 */
async function collectStats(): Promise<void> {
  const state = getState();
  const updates: ProcessInfo[] = [];

  for (const [projectId, managed] of state.processes) {
    // Get the PID to monitor (this is the shell wrapper PID)
    const rootPid = managed.realPid ?? managed.child?.pid;
    if (!rootPid) continue;

    try {
      // Get all child PIDs including the root process
      const pids = await pidtree(rootPid, { root: true });

      if (pids.length === 0) continue;

      // Get stats for all processes in the tree
      const allStats = await pidusage(pids);

      // Aggregate CPU and memory from all processes
      let totalCpu = 0;
      let totalMemory = 0;

      for (const pid of pids) {
        const stats = allStats[pid];
        if (stats) {
          totalCpu += stats.cpu;
          totalMemory += stats.memory;
        }
      }

      const info: ProcessInfo = {
        projectId,
        status: 'running',
        pid: rootPid,
        cpuUsage: totalCpu,
        memoryUsage: totalMemory,
      };

      state.processInfos.set(projectId, info);
      updates.push(info);

      // Store metric in database
      if (managed.sessionId) {
        try {
          state.database.insertMetric(
            managed.sessionId,
            totalCpu,
            totalMemory
          );
        } catch (error) {
          console.error('Failed to store metric:', error);
        }
      }
    } catch (error) {
      // Process might have exited
      // The process watcher will handle cleanup
    }
  }

  // Emit stats update to renderer
  if (updates.length > 0 && mainWindowRef && !mainWindowRef.isDestroyed()) {
    mainWindowRef.webContents.send(IPC_EVENTS.PROCESS_STATS_UPDATED, updates);
  }
}

/**
 * Get stats for a single process (one-time)
 * Aggregates stats from all child processes in the tree.
 */
export async function getProcessStats(pid: number): Promise<{ cpu: number; memory: number } | null> {
  try {
    // Get all child PIDs including the root process
    const pids = await pidtree(pid, { root: true });

    if (pids.length === 0) {
      return null;
    }

    // Get stats for all processes in the tree
    const allStats = await pidusage(pids);

    // Aggregate CPU and memory from all processes
    let totalCpu = 0;
    let totalMemory = 0;

    for (const pid of pids) {
      const stats = allStats[pid];
      if (stats) {
        totalCpu += stats.cpu;
        totalMemory += stats.memory;
      }
    }

    return {
      cpu: totalCpu,
      memory: totalMemory,
    };
  } catch {
    return null;
  }
}
