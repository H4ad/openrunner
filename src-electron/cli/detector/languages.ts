/**
 * Language-specific project detectors (Docker, Python, Rust, Go)
 */

import * as fs from 'fs';
import * as path from 'path';
import type { ProjectTemplate } from '../types';

export function detectDocker(directory: string): ProjectTemplate[] | null {
  if (!fs.existsSync(path.join(directory, 'Dockerfile'))) {
    return null;
  }

  return [{
    name: 'docker: build',
    command: 'docker build -t $(basename $(pwd)) .',
    description: 'Build Docker image',
    priority: 30
  }];
}

export function detectPython(directory: string): ProjectTemplate[] {
  const projects: ProjectTemplate[] = [];

  if (fs.existsSync(path.join(directory, 'requirements.txt'))) {
    projects.push({
      name: 'pip: install',
      command: 'pip install -r requirements.txt',
      description: 'Install Python dependencies',
      priority: 31
    });
  }

  if (fs.existsSync(path.join(directory, 'pyproject.toml'))) {
    projects.push({
      name: 'poetry: install',
      command: 'poetry install',
      description: 'Install Poetry dependencies',
      priority: 32
    });
  }

  if (fs.existsSync(path.join(directory, 'Pipfile'))) {
    projects.push({
      name: 'pipenv: install',
      command: 'pipenv install',
      description: 'Install Pipenv dependencies',
      priority: 33
    });
  }

  return projects;
}

export function detectRust(directory: string): ProjectTemplate[] {
  const projects: ProjectTemplate[] = [];
  const cargoPath = path.join(directory, 'Cargo.toml');

  if (!fs.existsSync(cargoPath)) {
    return projects;
  }

  projects.push({
    name: 'cargo: build',
    command: 'cargo build',
    description: 'Build Rust project',
    priority: 34
  });

  projects.push({
    name: 'cargo: test',
    command: 'cargo test',
    description: 'Run Rust tests',
    priority: 35
  });

  // Check if it's a workspace
  try {
    const content = fs.readFileSync(cargoPath, 'utf-8');
    if (content.includes('[workspace]')) {
      projects.push({
        name: 'cargo: build --workspace',
        command: 'cargo build --workspace',
        description: 'Build entire workspace',
        priority: 36
      });
    }
  } catch {
    // Ignore read errors
  }

  return projects;
}

export function detectGo(directory: string): ProjectTemplate[] {
  const projects: ProjectTemplate[] = [];

  if (!fs.existsSync(path.join(directory, 'go.mod'))) {
    return projects;
  }

  projects.push({
    name: 'go: build',
    command: 'go build ./...',
    description: 'Build Go project',
    priority: 37
  });

  projects.push({
    name: 'go: test',
    command: 'go test ./...',
    description: 'Run Go tests',
    priority: 38
  });

  projects.push({
    name: 'go: run',
    command: 'go run .',
    description: 'Run Go application',
    priority: 39
  });

  return projects;
}
