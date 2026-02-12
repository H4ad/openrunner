/**
 * Storage utilities for CLI mode
 * Direct database access without Electron app
 */

import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs';
import { Database } from '../main/services/database';
import type { CLIGroup, AppConfig } from './types';

function getConfigDir(): string {
  const configDir = path.join(os.homedir(), '.config', 'openrunner');
  if (!fs.existsSync(configDir)) {
    fs.mkdirSync(configDir, { recursive: true });
  }
  return configDir;
}

function getDatabasePath(): string {
  return path.join(getConfigDir(), 'runner-ui.db');
}

export function openDatabase(): Database {
  const dbPath = getDatabasePath();
  return new Database(dbPath);
}

export function loadConfig(): { groups: import('../shared/types').Group[] } {
  const db = openDatabase();
  try {
    const groups = db.getAllGroups();
    return { groups };
  } finally {
    db.close();
  }
}

export function saveGroup(group: CLIGroup): void {
  const db = openDatabase();
  try {
    // Check if group already exists
    const existingGroup = db.getGroup(group.id);
    if (existingGroup) {
      // Delete existing group to replace it
      db.deleteGroup(group.id);
    }

    // Create the group
    db.createGroup({
      id: group.id,
      name: group.name,
      directory: group.directory,
      projects: group.projects.map(p => ({
        id: p.id,
        name: p.name,
        command: p.command,
        autoRestart: p.autoRestart,
        envVars: p.envVars,
        cwd: p.cwd || group.directory,
        projectType: p.projectType,
        interactive: p.interactive
      })),
      envVars: group.envVars,
      syncFile: group.syncFile || undefined,
      syncEnabled: group.syncEnabled
    });
  } finally {
    db.close();
  }
}
