/**
 * Database migrations for schema updates.
 * Runs after schema initialization to add missing columns/tables.
 */

import type { Database } from 'better-sqlite3';

/**
 * Check if a column exists in a table
 */
function columnExists(
  db: Database,
  tableName: string,
  columnName: string
): boolean {
  const result = db
    .prepare(
      `SELECT COUNT(*) as count FROM pragma_table_info(?) WHERE name = ?`
    )
    .get(tableName, columnName) as { count: number };
  return result.count > 0;
}

/**
 * Migration: Add watch_patterns column to projects table
 * Added: 2026-02-12
 */
function addWatchPatternsColumn(db: Database): void {
  if (!columnExists(db, 'projects', 'watch_patterns')) {
    db.exec(`
      ALTER TABLE projects 
      ADD COLUMN watch_patterns TEXT
    `);
    console.log('[Database] Migration: Added watch_patterns column to projects table');
  }
}

/**
 * Migration: Add auto_start_on_launch column to projects table
 * Added: 2026-02-12
 */
function addAutoStartOnLaunchColumn(db: Database): void {
  if (!columnExists(db, 'projects', 'auto_start_on_launch')) {
    db.exec(`
      ALTER TABLE projects 
      ADD COLUMN auto_start_on_launch INTEGER NOT NULL DEFAULT 0
    `);
    console.log('[Database] Migration: Added auto_start_on_launch column to projects table');
  }
}

/**
 * Run all migrations
 */
export function runMigrations(db: Database): void {
  console.log('[Database] Running migrations...');
  
  addWatchPatternsColumn(db);
  addAutoStartOnLaunchColumn(db);
  
  console.log('[Database] Migrations complete');
}
