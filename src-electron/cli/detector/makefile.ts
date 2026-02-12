/**
 * Makefile detector
 */

import * as fs from 'fs';
import * as path from 'path';
import type { ProjectTemplate } from '../types';

const MAKEFILE_NAMES = ['Makefile', 'makefile', 'GNUmakefile'];

export function detectMakefile(directory: string): ProjectTemplate[] | null {
  let makefilePath: string | null = null;

  for (const name of MAKEFILE_NAMES) {
    const fullPath = path.join(directory, name);
    if (fs.existsSync(fullPath)) {
      makefilePath = fullPath;
      break;
    }
  }

  if (!makefilePath) {
    return null;
  }

  try {
    const content = fs.readFileSync(makefilePath, 'utf-8');
    const projects: ProjectTemplate[] = [];
    const seenTargets = new Set<string>();

    for (const line of content.split('\n')) {
      // Parse target: dependencies
      const colonIndex = line.indexOf(':');
      if (colonIndex === -1) continue;

      const target = line.substring(0, colonIndex).trim();

      // Skip special targets and variables
      if (!target || target.startsWith('.') || target.startsWith('#')) continue;
      if (target.includes('=') || target.includes('$')) continue;
      if (seenTargets.has(target)) continue;

      seenTargets.add(target);

      const { description, priority } = getMakefileTargetInfo(target);

      projects.push({
        name: `make: ${target}`,
        command: `make ${target}`,
        description,
        priority
      });
    }

    // Sort by priority
    projects.sort((a, b) => (a.priority || 50) - (b.priority || 50));

    return projects.length > 0 ? projects : null;
  } catch {
    return null;
  }
}

function getMakefileTargetInfo(target: string): { description: string; priority: number } {
  const targetMap: Record<string, { description: string; priority: number }> = {
    'dev': { description: 'Development server', priority: 1 },
    'develop': { description: 'Development server', priority: 1 },
    'start': { description: 'Start application', priority: 2 },
    'run': { description: 'Start application', priority: 2 },
    'serve': { description: 'Start application', priority: 2 },
    'build': { description: 'Build project', priority: 3 },
    'compile': { description: 'Build project', priority: 3 },
    'test': { description: 'Run tests', priority: 4 },
    'tests': { description: 'Run tests', priority: 4 },
    'lint': { description: 'Run linter', priority: 5 },
    'linter': { description: 'Run linter', priority: 5 },
    'install': { description: 'Install dependencies', priority: 6 },
    'deps': { description: 'Install dependencies', priority: 6 },
    'dependencies': { description: 'Install dependencies', priority: 6 },
    'watch': { description: 'Watch for changes', priority: 7 },
    'deploy': { description: 'Deploy application', priority: 8 },
    'fmt': { description: 'Format code', priority: 9 },
    'format': { description: 'Format code', priority: 9 },
    'check': { description: 'Verify/Check code', priority: 10 },
    'verify': { description: 'Verify/Check code', priority: 10 },
    'clean': { description: 'Clean build artifacts', priority: 100 }
  };

  return targetMap[target] || { description: 'Makefile target', priority: 50 };
}
