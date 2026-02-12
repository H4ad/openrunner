#!/usr/bin/env node
/**
 * OpenRunner CLI standalone entry point
 * 
 * This file is the CLI entry point that runs without Electron.
 * It can be invoked directly via Node.js.
 */

import { program } from 'commander';
import { executeNew } from './commands';

const VERSION = '0.2.4';

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
