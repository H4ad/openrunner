#!/usr/bin/env node
/**
 * OpenRunner CLI entry point
 * 
 * This module handles CLI argument parsing and command execution.
 * It runs before the Electron app launches and can prevent the GUI from starting
 * if a CLI command is executed.
 */

import { program } from 'commander';
import { executeNew } from './commands';

const VERSION = '0.2.4';

/**
 * Setup and parse CLI arguments
 */
export function setupCLI(): void {
  program
    .name('openrunner')
    .description('OpenRunner - Desktop process manager for local development')
    .version(VERSION, '-v, --version');

  // Define the 'new' command
  program
    .command('new [directory]')
    .description('Create a new group and auto-detect projects')
    .option('-n, --name <name>', 'Group name (default: directory name)')
    .option('--dry-run', 'Show what would be created without making changes')
    .action(async (directory = '.', options: { name?: string; dryRun?: boolean }) => {
      try {
        await executeNew(directory, options.name, options.dryRun || false);
        process.exit(0);
      } catch (error) {
        console.error('Error:', error instanceof Error ? error.message : error);
        process.exit(1);
      }
    });

  // Parse arguments
  program.parse();
}

/**
 * Check if CLI mode should be run
 * Returns true if CLI was invoked, false to run GUI
 */
export function shouldRunCLI(): boolean {
  const args = process.argv.slice(2);
  
  // Check if any CLI-specific arguments are present
  if (args.length === 0) {
    return false; // No arguments, run GUI
  }
  
  // Check for help or version flags
  if (args.includes('--help') || args.includes('-h') || 
      args.includes('--version') || args.includes('-v')) {
    return true;
  }
  
  // Check for the 'new' command
  if (args[0] === 'new') {
    return true;
  }
  
  // Check for other subcommands (future expansion)
  const subcommands = ['new', 'list', 'delete', 'start', 'stop'];
  if (subcommands.includes(args[0])) {
    return true;
  }
  
  return false;
}

/**
 * Run CLI mode
 * Returns a promise that resolves when CLI completes
 */
export async function runCLI(): Promise<void> {
  setupCLI();
}
