/**
 * Platform-specific process management abstraction.
 * This is the equivalent of src-tauri/src/platform/mod.rs
 */

import { spawn, ChildProcess } from 'child_process';

/**
 * Platform-specific process manager interface
 */
export interface PlatformProcessManager {
  /** Kill orphaned processes from previous runs */
  killOrphanedProcesses(pids: number[]): void;

  /** Configure spawn options for the platform */
  getSpawnOptions(): Record<string, unknown>;

  /** Get the process group ID from a PID */
  getProcessGroupId(pid: number): number | null;

  /** Send graceful shutdown signal to a process */
  gracefulShutdown(pid: number): void;

  /** Force kill a process */
  forceKill(pid: number): void;

  /** Check if a process is still running */
  isProcessRunning(pid: number): boolean;

  /** Get the shell command for the platform */
  getShellCommand(): { shell: string; args: string[] };
}

/**
 * Unix (Linux/macOS) process manager implementation
 */
class UnixProcessManager implements PlatformProcessManager {
  killOrphanedProcesses(pids: number[]): void {
    for (const pid of pids) {
      try {
        // Check if process exists (signal 0)
        process.kill(pid, 0);
        // Kill the process group
        process.kill(-pid, 'SIGKILL');
      } catch {
        // Process doesn't exist or can't be killed
      }
    }
  }

  getSpawnOptions(): Record<string, unknown> {
    return {
      detached: true, // Creates a new process group
    };
  }

  getProcessGroupId(pid: number): number | null {
    // On Unix with detached: true, PID == PGID
    return pid;
  }

  gracefulShutdown(pid: number): void {
    try {
      // Kill the process group with SIGTERM
      process.kill(-pid, 'SIGTERM');
    } catch {
      // Process doesn't exist
    }
  }

  forceKill(pid: number): void {
    try {
      // Kill the process group with SIGKILL
      process.kill(-pid, 'SIGKILL');
    } catch {
      // Process doesn't exist
    }
  }

  isProcessRunning(pid: number): boolean {
    try {
      // Signal 0 checks if process exists without sending a signal
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  }

  getShellCommand(): { shell: string; args: string[] } {
    return { shell: 'sh', args: ['-c'] };
  }
}

/**
 * Windows process manager implementation
 */
class WindowsProcessManager implements PlatformProcessManager {
  killOrphanedProcesses(pids: number[]): void {
    for (const pid of pids) {
      try {
        // Use taskkill to kill the process tree
        spawn('taskkill', ['/F', '/T', '/PID', pid.toString()], {
          stdio: 'ignore',
          detached: true,
        });
      } catch {
        // Process doesn't exist or can't be killed
      }
    }
  }

  getSpawnOptions(): Record<string, unknown> {
    return {
      // Windows doesn't have process groups like Unix
      windowsHide: true,
    };
  }

  getProcessGroupId(_pid: number): number | null {
    // Windows doesn't have process groups in the same way
    return null;
  }

  gracefulShutdown(pid: number): void {
    try {
      // Windows: Use taskkill with /T to kill process tree
      spawn('taskkill', ['/T', '/PID', pid.toString()], {
        stdio: 'ignore',
        detached: true,
      });
    } catch {
      // Process doesn't exist
    }
  }

  forceKill(pid: number): void {
    try {
      // Windows: Force kill with /F flag
      spawn('taskkill', ['/F', '/T', '/PID', pid.toString()], {
        stdio: 'ignore',
        detached: true,
      });
    } catch {
      // Process doesn't exist
    }
  }

  isProcessRunning(pid: number): boolean {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  }

  getShellCommand(): { shell: string; args: string[] } {
    return { shell: 'cmd.exe', args: ['/C'] };
  }
}

// Singleton instance
let platformManager: PlatformProcessManager | null = null;

/**
 * Get the platform-specific process manager
 */
export function getPlatformManager(): PlatformProcessManager {
  if (!platformManager) {
    platformManager = createPlatformManager();
  }
  return platformManager;
}

/**
 * Create the platform-specific process manager
 */
function createPlatformManager(): PlatformProcessManager {
  if (process.platform === 'win32') {
    return new WindowsProcessManager();
  }
  return new UnixProcessManager();
}
