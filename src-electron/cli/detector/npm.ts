/**
 * npm package.json detector
 */

import * as fs from 'fs';
import * as path from 'path';
import type { ProjectTemplate } from '../types';

const IGNORED_NPM_SCRIPTS = new Set([
  'prepare', 'preinstall', 'postinstall', 'prepublish', 'prepublishOnly',
  'publish', 'postpublish', 'prerestart', 'restart', 'postrestart',
  'prestop', 'stop', 'poststop', 'prestart', 'poststart',
  'predev', 'postdev', 'prebuild', 'postbuild', 'pretest', 'posttest',
  'prelint', 'postlint'
]);

export function detectPackageJson(directory: string): ProjectTemplate[] | null {
  const packageJsonPath = path.join(directory, 'package.json');

  if (!fs.existsSync(packageJsonPath)) {
    return null;
  }

  try {
    const content = fs.readFileSync(packageJsonPath, 'utf-8');
    const package_ = JSON.parse(content);
    const scripts = package_.scripts;

    if (!scripts || typeof scripts !== 'object') {
      return null;
    }

    const projects: ProjectTemplate[] = [];

    for (const [name, cmd] of Object.entries(scripts)) {
      if (typeof cmd !== 'string') continue;
      if (IGNORED_NPM_SCRIPTS.has(name)) continue;

      const description = getNpmScriptDescription(name, cmd);

      projects.push({
        name: `npm: ${name}`,
        command: `npm run ${name}`,
        description,
        priority: getNpmScriptPriority(name)
      });
    }

    return projects.length > 0 ? projects : null;
  } catch {
    return null;
  }
}

function getNpmScriptDescription(name: string, cmd: string): string {
  if (name === 'dev' || name === 'start') {
    return `Development server: ${cmd}`;
  } else if (name === 'build') {
    return `Build project: ${cmd}`;
  } else if (name === 'test') {
    return `Run tests: ${cmd}`;
  } else if (name.startsWith('test:')) {
    return `Test: ${cmd}`;
  } else if (name === 'lint') {
    return `Lint code: ${cmd}`;
  } else {
    return `Script: ${cmd}`;
  }
}

function getNpmScriptPriority(name: string): number {
  const priorities: Record<string, number> = {
    'dev': 1, 'start': 2, 'build': 3, 'test': 4,
    'lint': 5, 'install': 6
  };
  return priorities[name] || 50;
}
