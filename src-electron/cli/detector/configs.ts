/**
 * Common config files detector (Docker Compose, Justfile, Taskfile)
 */

import * as fs from 'fs';
import * as path from 'path';
import type { ProjectTemplate } from '../types';

export function detectCommonConfigs(directory: string): ProjectTemplate[] {
  const projects: ProjectTemplate[] = [];

  // Docker Compose
  const dockerComposeFiles = [
    'docker-compose.yml', 'docker-compose.yaml',
    'compose.yml', 'compose.yaml'
  ];

  for (const file of dockerComposeFiles) {
    if (fs.existsSync(path.join(directory, file))) {
      projects.push({
        name: 'docker: compose up',
        command: `docker compose -f ${file} up`,
        description: 'Start Docker Compose services',
        priority: 20
      });
      break;
    }
  }

  // Justfile
  if (fs.existsSync(path.join(directory, 'justfile')) ||
      fs.existsSync(path.join(directory, 'Justfile'))) {
    projects.push({
      name: 'just: list',
      command: 'just --list',
      description: 'List just recipes',
      priority: 25
    });
  }

  // Taskfile
  if (fs.existsSync(path.join(directory, 'Taskfile.yml')) ||
      fs.existsSync(path.join(directory, 'Taskfile.yaml'))) {
    projects.push({
      name: 'task: list',
      command: 'task --list',
      description: 'List task targets',
      priority: 25
    });
  }

  return projects;
}
