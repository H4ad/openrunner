/**
 * Main project detector - combines all detection strategies
 */

import * as path from 'path';
import type { ProjectTemplate } from '../types';
import { detectPackageJson } from './npm';
import { detectMakefile } from './makefile';
import { detectCommonConfigs } from './configs';
import { detectDocker, detectPython, detectRust, detectGo } from './languages';

export { ProjectTemplate };

export function detectProjects(directory: string): ProjectTemplate[] {
  const projects: ProjectTemplate[] = [];

  // Detect package.json scripts
  const npmProjects = detectPackageJson(directory);
  if (npmProjects) {
    projects.push(...npmProjects);
  }

  // Detect Makefile targets
  const makefileProjects = detectMakefile(directory);
  if (makefileProjects) {
    projects.push(...makefileProjects);
  }

  // Detect common config files
  projects.push(...detectCommonConfigs(directory));

  // Detect Docker projects
  const dockerProjects = detectDocker(directory);
  if (dockerProjects) {
    projects.push(...dockerProjects);
  }

  // Detect Python projects
  projects.push(...detectPython(directory));

  // Detect Rust projects
  projects.push(...detectRust(directory));

  // Detect Go projects
  projects.push(...detectGo(directory));

  return projects;
}
