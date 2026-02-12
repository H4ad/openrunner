/**
 * File watcher service for auto-restarting processes on file changes.
 * Uses chokidar for file watching and respects .gitignore patterns.
 */

import chokidar, { FSWatcher } from 'chokidar';
import * as fs from 'fs';
import * as path from 'path';
import picomatch from 'picomatch';

const DEBOUNCE_MS = 500;

interface WatcherConfig {
  projectId: string;
  watchDir: string;
  groupDir: string;
  patterns?: string[];
  onChange: (changedFile: string) => void;
}

interface ActiveWatcher {
  watcher: FSWatcher;
  config: WatcherConfig;
}

// Map of projectId to active watcher
const activeWatchers = new Map<string, ActiveWatcher>();

/**
 * Convert a gitignore pattern to chokidar-compatible glob patterns.
 * Gitignore patterns have different semantics than glob patterns.
 * Returns an array because some patterns need multiple glob equivalents.
 */
function convertGitignoreToGlob(pattern: string, gitignoreDir: string): string[] {
  // Skip negation patterns (we don't support them for simplicity)
  if (pattern.startsWith('!')) {
    return [];
  }

  // Remove trailing spaces
  pattern = pattern.trimEnd();

  // Skip empty patterns
  if (!pattern) {
    return [];
  }

  // Handle patterns starting with / (relative to gitignore location)
  if (pattern.startsWith('/')) {
    // Remove leading slash and make it relative to gitignore dir
    pattern = pattern.slice(1);
    
    if (pattern.endsWith('/')) {
      // Directory pattern: /dir/ -> match dir and all contents
      const dirPath = path.join(gitignoreDir, pattern.slice(0, -1));
      return [dirPath, dirPath + '/**'];
    }
    
    // Could be file or directory, match both cases
    const fullPath = path.join(gitignoreDir, pattern);
    return [fullPath, fullPath + '/**'];
  }

  // Handle patterns ending with / (directory only)
  if (pattern.endsWith('/')) {
    // Match this directory anywhere in the tree and all its contents
    const dirName = pattern.slice(0, -1);
    return ['**/' + dirName, '**/' + dirName + '/**'];
  }

  // Handle patterns with / in the middle (path-based patterns)
  if (pattern.includes('/')) {
    // These are relative to the gitignore location
    const fullPath = path.join(gitignoreDir, pattern);
    return [fullPath, fullPath + '/**'];
  }

  // Simple pattern without wildcards (e.g., ".venv", "temp") - could be file or directory
  // Match both the item itself and if it's a directory, all its contents
  if (!pattern.includes('*') && !pattern.includes('?')) {
    return ['**/' + pattern, '**/' + pattern + '/**'];
  }

  // Simple pattern with wildcards (e.g., "*.log") - match anywhere
  return ['**/' + pattern];
}

/**
 * Parse .gitignore file and return array of chokidar-compatible glob patterns
 */
function parseGitignore(gitignorePath: string): string[] {
  try {
    const content = fs.readFileSync(gitignorePath, 'utf-8');
    const gitignoreDir = path.dirname(gitignorePath);

    return content
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith('#'))
      .flatMap((pattern) => convertGitignoreToGlob(pattern, gitignoreDir));
  } catch {
    return [];
  }
}

/**
 * Find and parse all .gitignore files from watchDir up to groupDir (inclusive).
 * Stops at groupDir to avoid traversing the entire filesystem.
 */
function collectGitignorePatterns(watchDir: string, groupDir: string): string[] {
  const patterns: string[] = [];
  let currentDir = path.resolve(watchDir);
  const normalizedGroupDir = path.resolve(groupDir);

  while (true) {
    const gitignorePath = path.join(currentDir, '.gitignore');
    if (fs.existsSync(gitignorePath)) {
      patterns.push(...parseGitignore(gitignorePath));
    }

    // Stop if we've reached the group directory
    if (currentDir === normalizedGroupDir) {
      break;
    }

    const parentDir = path.dirname(currentDir);

    // Stop if we've reached the filesystem root
    if (parentDir === currentDir) {
      break;
    }

    // Stop if parent is outside the group directory
    // (this handles cases where watchDir is not inside groupDir)
    if (!parentDir.startsWith(normalizedGroupDir)) {
      break;
    }

    currentDir = parentDir;
  }

  return patterns;
}

/**
 * Start watching a project directory for file changes
 */
export function startFileWatcher(config: WatcherConfig): void {
  // Stop existing watcher for this project if any
  stopFileWatcher(config.projectId);

  const { watchDir, groupDir, patterns } = config;

  // Collect .gitignore patterns from watchDir up to groupDir
  const gitignorePatterns = collectGitignorePatterns(watchDir, groupDir);

  // Build ignored patterns from .gitignore and defaults
  const ignoredPatterns: string[] = [
    '**/node_modules/**',
    '**/.git/**',
    '**/dist/**',
    '**/build/**',
    '**/.cache/**',
    ...gitignorePatterns,
  ];

  // Create a matcher function for ignored patterns using picomatch
  // Chokidar v5 no longer supports glob patterns directly in `ignored`
  const isIgnored = picomatch(ignoredPatterns, { dot: true });

  // Create a matcher for user-provided watch patterns (if any)
  // These patterns define which files to INCLUDE (not ignore)
  const hasWatchPatterns = patterns && patterns.length > 0;
  const matchesWatchPattern = hasWatchPatterns
    ? picomatch(patterns, { dot: true })
    : null;

  // Create the ignored function for chokidar
  // NOTE: Chokidar calls this function twice per path:
  // 1. First with just the path (stats is undefined) - for quick filtering
  // 2. Second with path and stats - for detailed filtering
  // We must be careful not to filter out directories during traversal
  const ignoredFn = (filePath: string, stats?: fs.Stats): boolean => {
    const basename = path.basename(filePath);
    
    // When stats is undefined, we only check against definite ignore patterns
    // We cannot apply watch patterns here because we don't know if it's a file or directory
    if (!stats) {
      // Only ignore paths that are definitely in the ignore list
      if (isIgnored(filePath) || isIgnored(basename)) {
        return true;
      }
      // Don't filter anything else without stats - let chokidar call us again with stats
      return false;
    }
    
    // For directories: only check against ignore patterns, NOT watch patterns
    // We need to traverse directories to find matching files inside
    if (stats.isDirectory()) {
      if (isIgnored(filePath) || isIgnored(basename)) {
        return true;
      }
      return false;
    }

    // For files: check against ignore patterns
    if (isIgnored(filePath) || isIgnored(basename)) {
      return true;
    }

    // For files: if user provided watch patterns, only include files that match
    if (matchesWatchPattern) {
      // Get relative path from watchDir for pattern matching
      const relativePath = path.relative(watchDir, filePath);
      
      // Check if file matches any of the user's watch patterns
      const matchesRelative = matchesWatchPattern(relativePath);
      const matchesBasename = matchesWatchPattern(basename);
      const matchesFull = matchesWatchPattern(filePath);
      
      if (!matchesRelative && !matchesBasename && !matchesFull) {
        return true; // Ignore files that don't match watch patterns
      }
    }

    return false;
  };

  // Always watch the directory (chokidar v5 doesn't support globs in watch paths)
  const watcher = chokidar.watch(watchDir, {
    ignored: ignoredFn,
    ignoreInitial: true,
    persistent: true,
    followSymlinks: false,
    awaitWriteFinish: {
      stabilityThreshold: 100,
      pollInterval: 100,
    },
  });

  // Debounced change handler
  let debounceTimer: NodeJS.Timeout | null = null;

  const handleChange = (changePath: string) => {
    // Clear existing timer
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    // Set new timer
    debounceTimer = setTimeout(() => {
      console.log(`[FileWatcher] File changed: ${changePath}, triggering restart for project ${config.projectId}`);
      config.onChange(changePath);
    }, DEBOUNCE_MS);
  };

  // Log when watcher is ready
  watcher.on('ready', () => {
    console.log(`[FileWatcher] Watcher ready for project ${config.projectId}`);
  });

  // Watch for all change events
  watcher.on('add', handleChange);
  watcher.on('change', handleChange);
  watcher.on('unlink', handleChange);
  watcher.on('addDir', handleChange);
  watcher.on('unlinkDir', handleChange);

  // Debug: log raw events to see what's being detected
  watcher.on('raw', (event, filePath, details) => {
    console.log(`[FileWatcher] Raw event: ${event} on ${filePath}`);
  });

  // Error handling
  watcher.on('error', (error) => {
    console.error(`[FileWatcher] Error watching project ${config.projectId}:`, error);
  });

  // Store active watcher
  activeWatchers.set(config.projectId, { watcher, config });

  console.log(`[FileWatcher] Started watching project ${config.projectId} in ${watchDir}`);
  console.log(`[FileWatcher] Watch patterns (user): ${JSON.stringify(patterns || [])}`);
  console.log(`[FileWatcher] Ignored patterns: ${JSON.stringify(ignoredPatterns)}`);
}

/**
 * Stop watching a project
 */
export function stopFileWatcher(projectId: string): void {
  const active = activeWatchers.get(projectId);
  if (active) {
    active.watcher.close();
    activeWatchers.delete(projectId);
    console.log(`[FileWatcher] Stopped watching project ${projectId}`);
  }
}

/**
 * Stop all file watchers
 */
export function stopAllFileWatchers(): void {
  for (const [projectId] of activeWatchers) {
    stopFileWatcher(projectId);
  }
}

/**
 * Check if a project has an active file watcher
 */
export function hasFileWatcher(projectId: string): boolean {
  return activeWatchers.has(projectId);
}
