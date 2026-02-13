/**
 * YAML configuration service for reading/writing openrunner.yaml files.
 * This is the equivalent of src-tauri/src/yaml_config.rs
 */

import * as fs from 'fs';
import * as path from 'path';
import * as yaml from 'js-yaml';
import type { Group, Project, ProjectType } from '../../shared/types';

const YAML_FILENAME = 'openrunner.yaml';
const YML_FILENAME = 'openrunner.yml';

export interface YamlConfig {
  version: string;
  name: string;
  envVars?: Record<string, string>;
  projects: YamlProject[];
}

export interface YamlProject {
  name: string;
  command: string;
  type: 'service' | 'task';
  autoRestart?: boolean;
  cwd?: string;
  interactive?: boolean;
  envVars?: Record<string, string>;
  watchPatterns?: string[];
  autoStartOnLaunch?: boolean;
}

/**
 * Find an openrunner.yaml or openrunner.yml file in a directory
 */
export function findYamlFile(directory: string): string | null {
  const yamlPath = path.join(directory, YAML_FILENAME);
  if (fs.existsSync(yamlPath)) {
    return yamlPath;
  }

  const ymlPath = path.join(directory, YML_FILENAME);
  if (fs.existsSync(ymlPath)) {
    return ymlPath;
  }

  return null;
}

/**
 * Get the default YAML path for a directory (creates openrunner.yaml if neither exists)
 */
export function getYamlPath(directory: string): string {
  const yamlPath = path.join(directory, YAML_FILENAME);
  if (fs.existsSync(yamlPath)) {
    return yamlPath;
  }

  const ymlPath = path.join(directory, YML_FILENAME);
  if (fs.existsSync(ymlPath)) {
    return ymlPath;
  }

  // Default to .yaml extension
  return yamlPath;
}

/**
 * Parse a YAML config file
 */
export function parseYaml(filePath: string): YamlConfig {
  const content = fs.readFileSync(filePath, 'utf-8');
  const config = yaml.load(content) as YamlConfig;

  if (!config || typeof config !== 'object') {
    throw new Error('Invalid YAML file format');
  }

  if (!Array.isArray(config.projects)) {
    throw new Error('YAML file must contain a projects array');
  }

  return config;
}

/**
 * Write a group to a YAML file
 */
export function writeYaml(group: Group, filePath: string): void {
  const yamlProjects: YamlProject[] = group.projects.map((p) => {
    const yamlProject: YamlProject = {
      name: p.name,
      command: p.command,
      type: p.projectType,
      autoRestart: p.autoRestart,
      cwd: p.cwd ?? undefined,
      interactive: p.interactive,
    };

    // Only include envVars if non-empty
    if (Object.keys(p.envVars).length > 0) {
      yamlProject.envVars = p.envVars;
    }

    // Only include watchPatterns if defined and non-empty
    if (p.watchPatterns && p.watchPatterns.length > 0) {
      yamlProject.watchPatterns = p.watchPatterns;
    }

    // Only include autoStartOnLaunch if true
    if (p.autoStartOnLaunch) {
      yamlProject.autoStartOnLaunch = p.autoStartOnLaunch;
    }

    return yamlProject;
  });

  const yamlConfig: YamlConfig = {
    version: '1.0',
    name: group.name,
    projects: yamlProjects,
  };

  // Only include envVars if non-empty
  if (Object.keys(group.envVars).length > 0) {
    yamlConfig.envVars = group.envVars;
  }

  const yamlContent = yaml.dump(yamlConfig, {
    indent: 2,
    lineWidth: -1, // Disable line wrapping
    noRefs: true,
    quotingType: '"',
    forceQuotes: false,
  });

  fs.writeFileSync(filePath, yamlContent, 'utf-8');
}

/**
 * Convert a YAML config to a Group
 */
export function yamlToGroup(
  config: YamlConfig,
  directory: string,
  syncFile: string
): Group {
  const projects: Project[] = config.projects.map((p) =>
    yamlToProject(p, directory)
  );

  return {
    id: crypto.randomUUID(),
    name: config.name,
    directory,
    projects,
    envVars: config.envVars ?? {},
    syncFile,
    syncEnabled: true,
  };
}

/**
 * Convert a YAML project to a Project
 */
export function yamlToProject(yamlProject: YamlProject, _baseDir: string): Project {
  return {
    id: crypto.randomUUID(),
    name: yamlProject.name,
    command: yamlProject.command,
    projectType: yamlProject.type as ProjectType,
    autoRestart: yamlProject.autoRestart ?? true,
    cwd: yamlProject.cwd ?? null,
    interactive: yamlProject.interactive ?? false,
    envVars: yamlProject.envVars ?? {},
    watchPatterns: yamlProject.watchPatterns,
    autoStartOnLaunch: yamlProject.autoStartOnLaunch ?? false,
  };
}

/**
 * Update an existing group from a YAML config (preserves project IDs where names match)
 */
export function updateGroupFromYaml(
  group: Group,
  config: YamlConfig,
  baseDir: string
): Group {
  // Build a map of existing project names to IDs
  const existingProjectIds = new Map<string, string>();
  for (const project of group.projects) {
    existingProjectIds.set(project.name, project.id);
  }

  // Create new projects, preserving IDs where names match
  const projects: Project[] = config.projects.map((yp) => {
    const project = yamlToProject(yp, baseDir);
    const existingId = existingProjectIds.get(project.name);
    if (existingId) {
      project.id = existingId;
    }
    return project;
  });

  return {
    ...group,
    name: config.name,
    envVars: config.envVars ?? {},
    projects,
  };
}
