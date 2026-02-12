/**
 * CLI commands implementation
 */

import * as path from 'path';
import * as fs from 'fs';
import * as crypto from 'crypto';
import type { CLIGroup, CLIProject, ProjectTemplate } from './types';
import { detectProjects } from './detector';
import { showPreview, promptProjectSelection, promptConfirmation } from './ui';
import { saveGroup } from './storage';
import { findYamlFile, parseYaml } from '../main/services/yaml-config';

export async function executeNew(
  directory: string,
  name: string | undefined,
  dryRun: boolean
): Promise<void> {
  // Resolve directory path
  const resolvedDir = path.resolve(directory);

  if (!fs.existsSync(resolvedDir)) {
    console.error(`Error: Directory does not exist: ${resolvedDir}`);
    process.exit(1);
  }

  // Check for openrunner.yaml file first
  const yamlPath = findYamlFile(resolvedDir);
  if (yamlPath) {
    console.log(`Found YAML config: ${yamlPath}`);
    console.log('Importing group from YAML (bypassing auto-detection)...\n');

    const yamlGroup = parseYaml(yamlPath);
    const groupName = name || yamlGroup.name;

    if (dryRun) {
      console.log('\n[DRY RUN] Group to be created from YAML:');
      console.log(`  Name: ${groupName}`);
      console.log(`  Directory: ${resolvedDir}`);
      console.log(`  Projects: ${yamlGroup.projects.length}`);
      for (const project of yamlGroup.projects) {
        console.log(`    - ${project.name}: ${project.command}`);
      }
      console.log('\nNo changes were made.');
      return;
    }

    // Create group with YAML data
    const groupId = crypto.randomUUID();
    const directoryStr = resolvedDir;
    const yamlPathStr = yamlPath;

    const projects: CLIProject[] = yamlGroup.projects.map(yamlProject => ({
      id: crypto.randomUUID(),
      name: yamlProject.name,
      command: yamlProject.command,
      autoRestart: yamlProject.autoRestart ?? true,
      envVars: yamlProject.envVars || {},
      cwd: yamlProject.cwd || directoryStr,
      projectType: yamlProject.type,
      interactive: yamlProject.interactive ?? false
    }));

    const newGroup: CLIGroup = {
      id: groupId,
      name: groupName,
      directory: directoryStr,
      projects,
      envVars: yamlGroup.envVars || {},
      syncFile: yamlPathStr,
      syncEnabled: true
    };

    saveGroup(newGroup);

    console.log('\n╔════════════════════════════════════════════════════════╗');
    console.log('║       GROUP CREATED SUCCESSFULLY FROM YAML!            ║');
    console.log('╚════════════════════════════════════════════════════════╝');
    console.log(`  Group: ${groupName}`);
    console.log(`  Projects: ${projects.length}`);
    console.log(`  Config: ~/.config/openrunner/runner-ui.db`);
    console.log(`  Sync: ${yamlPath} (auto-sync enabled)`);
    console.log('\n  You can now open OpenRunner to manage this group.');
    console.log('  Changes will sync bidirectionally between the app and YAML file.');

    return;
  }

  // Get group name from directory
  const groupName = name || path.basename(resolvedDir);

  console.log(`Scanning directory: ${resolvedDir}\n`);

  // Detect projects
  const detectedProjects = detectProjects(resolvedDir);

  if (detectedProjects.length === 0) {
    console.log('No projects detected in this directory.');
    console.log('Supported project files:');
    console.log('  - package.json (npm scripts)');
    console.log('  - Makefile / makefile');
    console.log('  - docker-compose.yml');
    console.log('  - Cargo.toml');
    console.log('  - go.mod');
    console.log('  - pyproject.toml / requirements.txt');
    console.log('  - justfile / Justfile');
    console.log('  - Taskfile.yml');
    console.log('  - openrunner.yaml (for manual configuration)');
    return;
  }

  console.log(`Detected ${detectedProjects.length} project(s)\n`);

  // Show preview
  showPreview(groupName, resolvedDir, detectedProjects);

  if (dryRun) {
    console.log('[DRY RUN] No changes were made.');
    return;
  }

  // Interactive project selection
  const selectedIndices = await promptProjectSelection(detectedProjects);

  if (selectedIndices.length === 0) {
    console.log('No projects selected. Canceling.');
    return;
  }

  const selectedProjects = selectedIndices.map(i => detectedProjects[i]);

  console.log(`\nYou selected ${selectedProjects.length} project(s) to create:`);
  for (const project of selectedProjects) {
    console.log(`  - ${project.name}`);
  }

  // Final confirmation
  if (!await promptConfirmation('\nCreate this group with the selected projects?')) {
    console.log('Canceled.');
    return;
  }

  // Create the group and projects
  const groupId = crypto.randomUUID();
  const directoryStr = resolvedDir;

  const projects: CLIProject[] = selectedProjects.map(template => ({
    id: crypto.randomUUID(),
    name: template.name,
    command: template.command,
    autoRestart: false,
    envVars: {},
    cwd: directoryStr,
    projectType: 'service',
    interactive: false
  }));

  const newGroup: CLIGroup = {
    id: groupId,
    name: groupName,
    directory: directoryStr,
    projects,
    envVars: {},
    syncFile: null,
    syncEnabled: false
  };

  saveGroup(newGroup);

  // Success message
  console.log('\n╔════════════════════════════════════════════════════════╗');
  console.log('║              GROUP CREATED SUCCESSFULLY!               ║');
  console.log('╚════════════════════════════════════════════════════════╝');
  console.log(`  Group: ${groupName}`);
  console.log(`  Projects: ${selectedProjects.length}`);
  console.log(`  Config saved to: ~/.config/openrunner/runner-ui.db`);
  console.log('\n  You can now open OpenRunner to manage this group.');
}
