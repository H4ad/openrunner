/**
 * CLI UI utilities for interactive prompts
 */

import { checkbox, confirm } from '@inquirer/prompts';
import * as path from 'path';
import type { ProjectTemplate } from './types';

export function showPreview(groupName: string, directory: string, projects: ProjectTemplate[]): void {
  console.log('\n╔════════════════════════════════════════════════════════╗');
  console.log('║           PREVIEW: Group to be created                 ║');
  console.log('╚════════════════════════════════════════════════════════╝');
  console.log(`  Group Name: ${groupName}`);
  console.log(`  Directory:  ${directory}`);
  console.log(`  Projects:   ${projects.length}`);
  console.log('  ───────────────────────────────────────────────────────');

  for (let i = 0; i < projects.length; i++) {
    const project = projects[i];
    console.log(`  ${i + 1}. ${project.name}`);
    console.log(`     └─ ${project.description}`);
    console.log(`     └─ $ ${project.command}`);
    if (i < projects.length - 1) {
      console.log();
    }
  }
  console.log('  ═══════════════════════════════════════════════════════\n');
}

export async function promptProjectSelection(projects: ProjectTemplate[]): Promise<number[]> {
  if (projects.length === 0) {
    return [];
  }

  const choices = projects.map((p, index) => ({
    name: `${p.name} - ${p.description}`,
    value: index,
    checked: true
  }));

  try {
    const selected = await checkbox({
      message: 'Select projects to include (use arrow keys to navigate, space to select/deselect, enter to confirm):',
      choices
    });

    return selected.sort((a, b) => a - b);
  } catch {
    return [];
  }
}

export async function promptConfirmation(message: string): Promise<boolean> {
  try {
    return await confirm({
      message,
      default: true
    });
  } catch {
    return false;
  }
}
