/**
 * CLI types and interfaces
 */

export interface ProjectTemplate {
  name: string;
  command: string;
  description: string;
  priority?: number;
}

export interface CLIGroup {
  id: string;
  name: string;
  directory: string;
  projects: CLIProject[];
  envVars: Record<string, string>;
  syncFile: string | null;
  syncEnabled: boolean;
}

export interface CLIProject {
  id: string;
  name: string;
  command: string;
  autoRestart: boolean;
  autoStartOnLaunch: boolean;
  envVars: Record<string, string>;
  cwd: string | null;
  projectType: 'task' | 'service';
  interactive: boolean;
}

export interface AppConfig {
  groups: CLIGroup[];
}
