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
  getShellCommand(customShell?: string | null): { shell: string; args: string[] };
}

/**
 * Detect the user's default shell from environment
 */
export function detectUserShell(): string {
  // Try to get shell from environment variables
  const shell = process.env.SHELL || process.env.ComSpec;
  
  if (shell) {
    return shell;
  }
  
  // Fallback to platform defaults
  if (process.platform === 'win32') {
    return 'cmd.exe';
  }
  
  // For Unix-like systems, try common shells
  return 'sh';
}

/**
 * Get shell command with args based on shell type
 */
function getShellWithArgs(shell: string): { shell: string; args: string[] } {
  const shellName = shell.toLowerCase();
  
  // Windows shells
  if (shellName.includes('cmd.exe') || shellName.includes('cmd')) {
    return { shell, args: ['/C'] };
  }
  
  if (shellName.includes('powershell') || shellName.includes('pwsh')) {
    return { shell, args: ['-Command'] };
  }
  
  // Unix shells - use interactive login shell (-i -l) to load rc files
  // -l (login) loads profile files (~/.zprofile, ~/.profile)
  // -i (interactive) loads rc files (~/.zshrc, ~/.bashrc) where nvm/fnm are typically configured
  if (shellName.includes('bash')) {
    return { shell, args: ['-i', '-l', '-c'] };
  }
  
  if (shellName.includes('zsh')) {
    return { shell, args: ['-i', '-l', '-c'] };
  }
  
  if (shellName.includes('fish')) {
    return { shell, args: ['-i', '-l', '-c'] };
  }
  
  // Default for other shells (sh, dash, etc.) - these don't support -i well
  return { shell, args: ['-l', '-c'] };
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

  getShellCommand(customShell?: string | null): { shell: string; args: string[] } {
    // Use custom shell if provided, otherwise detect user's shell
    const shell = customShell && customShell.trim() ? customShell : detectUserShell();
    return getShellWithArgs(shell);
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

  getShellCommand(customShell?: string | null): { shell: string; args: string[] } {
    // Use custom shell if provided, otherwise detect user's shell
    const shell = customShell && customShell.trim() ? customShell : detectUserShell();
    return getShellWithArgs(shell);
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


