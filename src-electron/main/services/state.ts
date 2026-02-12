/**
 * Application state management for the Electron main process.
 * This is the equivalent of src-tauri/src/state.rs
 */

import { app } from 'electron';
import { join } from 'path';
import { existsSync, mkdirSync, readFileSync, writeFileSync, appendFileSync } from 'fs';
import type {
  AppConfig,
  ProcessInfo,
  ManagedProcess,
  Group,
} from '../../shared/types';
import { Database } from './database';

/**
 * Application state singleton
 */
class AppState {
  private static instance: AppState | null = null;

  // Configuration
  config: AppConfig = { groups: [] };

  // Database
  database!: Database;
  databasePath: string = '';

  // Process management
  processes: Map<string, ManagedProcess> = new Map();
  processInfos: Map<string, ProcessInfo> = new Map();
  activeSessions: Map<string, string> = new Map();

  // Paths
  logDir: string = '';
  configDir: string = '';
  pidFilePath: string = '';

  private constructor() {
    // Private constructor for singleton
  }

  /**
   * Get the singleton instance
   */
  static getInstance(): AppState {
    if (!AppState.instance) {
      AppState.instance = new AppState();
    }
    return AppState.instance;
  }

  /**
   * Initialize the application state
   */
  async initialize(): Promise<void> {
    // Set up paths
    this.configDir = join(app.getPath('userData'));
    this.logDir = join(this.configDir, 'logs');
    this.pidFilePath = join(this.configDir, 'running_pids.txt');
    this.databasePath = join(this.configDir, 'runner-ui.db');

    // Ensure directories exist
    if (!existsSync(this.configDir)) {
      mkdirSync(this.configDir, { recursive: true });
    }
    if (!existsSync(this.logDir)) {
      mkdirSync(this.logDir, { recursive: true });
    }

    // Initialize database
    this.database = new Database(this.databasePath);

    // Load configuration from database
    await this.loadConfig();

    // Kill any orphaned processes from previous runs
    this.killOrphanedProcesses();
  }

  /**
   * Load configuration from database
   */
  private async loadConfig(): Promise<void> {
    try {
      const groups = this.database.getAllGroups();
      this.config = { groups };
    } catch (error) {
      console.error('Failed to load config from database:', error);
      this.config = { groups: [] };
    }
  }

  /**
   * Reload configuration from database
   */
  async reloadConfig(): Promise<void> {
    await this.loadConfig();
  }

  /**
   * Get log file path for a project
   */
  logFilePath(projectId: string): string {
    return join(this.logDir, `${projectId}.log`);
  }

  /**
   * Save a PID to the PID file (for orphan cleanup on restart)
   */
  savePid(pid: number): void {
    try {
      appendFileSync(this.pidFilePath, `${pid}\n`);
    } catch (error) {
      console.error('Failed to save PID:', error);
    }
  }

  /**
   * Remove a PID from the PID file
   */
  removePid(pid: number): void {
    try {
      if (!existsSync(this.pidFilePath)) return;

      const content = readFileSync(this.pidFilePath, 'utf-8');
      const remaining = content
        .split('\n')
        .filter((line) => {
          const parsed = parseInt(line.trim(), 10);
          return !isNaN(parsed) && parsed !== pid;
        })
        .join('\n');

      writeFileSync(this.pidFilePath, remaining);
    } catch (error) {
      console.error('Failed to remove PID:', error);
    }
  }

  /**
   * Read all stored PIDs from the PID file
   */
  readStoredPids(): number[] {
    try {
      if (!existsSync(this.pidFilePath)) return [];

      const content = readFileSync(this.pidFilePath, 'utf-8');
      return content
        .split('\n')
        .map((line) => parseInt(line.trim(), 10))
        .filter((pid) => !isNaN(pid));
    } catch (error) {
      console.error('Failed to read PIDs:', error);
      return [];
    }
  }

  /**
   * Clear the PID file
   */
  clearPidFile(): void {
    try {
      writeFileSync(this.pidFilePath, '');
    } catch (error) {
      console.error('Failed to clear PID file:', error);
    }
  }

  /**
   * Kill any orphaned processes from previous app runs
   */
  private killOrphanedProcesses(): void {
    const pids = this.readStoredPids();
    for (const pid of pids) {
      try {
        // Check if process is still running and kill it
        process.kill(pid, 0); // Check if process exists
        process.kill(pid, 'SIGKILL'); // Kill it
      } catch {
        // Process doesn't exist or can't be killed
      }
    }
    this.clearPidFile();
  }

  /**
   * Shutdown cleanup
   */
  shutdown(): void {
    // Kill all managed processes
    for (const [projectId, managed] of this.processes) {
      try {
        const pid = managed.realPid ?? managed.child?.pid;
        if (pid) {
          process.kill(pid, 'SIGKILL');
        }
        if (managed.ptyProcess) {
          managed.ptyProcess.kill();
        }
      } catch {
        // Process already dead
      }
    }
    this.processes.clear();
    this.clearPidFile();

    // Close database
    if (this.database) {
      this.database.close();
    }
  }

  /**
   * Find a group by ID
   */
  findGroup(groupId: string): Group | undefined {
    return this.config.groups.find((g) => g.id === groupId);
  }

  /**
   * Find a project by ID across all groups
   */
  findProject(projectId: string): { group: Group; project: Group['projects'][0] } | undefined {
    for (const group of this.config.groups) {
      const project = group.projects.find((p) => p.id === projectId);
      if (project) {
        return { group, project };
      }
    }
    return undefined;
  }
}

// Singleton accessor functions
let stateInstance: AppState | null = null;

/**
 * Initialize the application state (call once at startup)
 */
export async function initializeState(): Promise<void> {
  stateInstance = AppState.getInstance();
  await stateInstance.initialize();
}

/**
 * Get the application state
 */
export function getState(): AppState {
  if (!stateInstance) {
    throw new Error('State not initialized. Call initializeState() first.');
  }
  return stateInstance;
}

/**
 * Shutdown the application state
 */
export function shutdownState(): void {
  if (stateInstance) {
    stateInstance.shutdown();
    stateInstance = null;
  }
}
