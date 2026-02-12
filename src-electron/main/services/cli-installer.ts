/**
 * CLI Installer Service
 * 
 * Handles installation and uninstallation of the CLI command.
 * - macOS/Linux: Creates wrapper script at /usr/local/bin/openrunner
 * - Windows: Creates batch file in user's AppData Scripts folder and adds to PATH
 * 
 * For AppImage distributions, CLI files are copied to a persistent location
 * (~/.local/share/openrunner/cli/) since AppImages mount to temporary paths.
 */

import { app } from 'electron';
import { existsSync, unlinkSync, writeFileSync, mkdirSync, copyFileSync, rmSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { execSync, spawn } from 'child_process';
import { is } from '@electron-toolkit/utils';

/**
 * Recursively copy a directory
 */
function copyDirRecursive(src: string, dest: string): void {
  if (!existsSync(dest)) {
    mkdirSync(dest, { recursive: true });
  }
  
  const entries = readdirSync(src);
  for (const entry of entries) {
    const srcPath = join(src, entry);
    const destPath = join(dest, entry);
    const stat = statSync(srcPath);
    
    if (stat.isDirectory()) {
      copyDirRecursive(srcPath, destPath);
    } else {
      copyFileSync(srcPath, destPath);
    }
  }
}

export interface CliInstallResult {
  success: boolean;
  message: string;
  installed: boolean;
  path?: string;
  requiresSudo?: boolean;
}

/**
 * Check if running as an AppImage (Linux)
 */
function isAppImage(): boolean {
  return !!process.env.APPIMAGE;
}

/**
 * Get the persistent CLI directory for storing CLI files
 * Used for AppImage and potentially other portable distributions
 */
function getPersistentCliDir(): string {
  if (process.platform === 'win32') {
    const appData = app.getPath('userData');
    return join(dirname(appData), 'OpenRunner', 'cli');
  }
  // Linux/macOS: ~/.local/share/openrunner/cli
  return join(app.getPath('home'), '.local', 'share', 'openrunner', 'cli');
}

/**
 * Get the path to the CLI script inside the app bundle
 */
function getCliSourcePath(): string {
  if (is.dev) {
    // In development, use the built CLI directly
    return join(__dirname, 'cli.js');
  }

  // In production, the CLI is unpacked from asar
  // The path is: resources/app.asar.unpacked/out/main/cli.js
  const resourcesPath = process.resourcesPath;
  return join(resourcesPath, 'app.asar.unpacked', 'out', 'main', 'cli.js');
}

/**
 * Get the path to the chunks directory inside the app bundle
 */
function getChunksSourcePath(): string {
  if (is.dev) {
    return join(__dirname, 'chunks');
  }
  const resourcesPath = process.resourcesPath;
  return join(resourcesPath, 'app.asar.unpacked', 'out', 'main', 'chunks');
}

/**
 * Get the path to the node_modules directory with native modules
 */
function getNativeModulesPath(): string {
  if (is.dev) {
    // In development, use the project's node_modules
    return join(__dirname, '..', '..', '..', 'node_modules');
  }
  // In production, native modules are unpacked from asar
  const resourcesPath = process.resourcesPath;
  return join(resourcesPath, 'app.asar.unpacked', 'node_modules');
}

/**
 * Get the target installation path for the CLI symlink/script
 */
function getCliTargetPath(): string {
  if (process.platform === 'win32') {
    // Windows: Use a scripts folder in AppData
    const appData = app.getPath('userData');
    const scriptsDir = join(dirname(appData), 'OpenRunner', 'bin');
    return join(scriptsDir, 'openrunner.cmd');
  }
  
  // Unix: /usr/local/bin/openrunner
  return '/usr/local/bin/openrunner';
}

/**
 * Copy CLI files to persistent location (for AppImage)
 * Returns the path to the copied cli.js
 */
function copyCliToPersistentLocation(): string {
  const persistentDir = getPersistentCliDir();
  const chunksDir = join(persistentDir, 'chunks');
  const nodeModulesDir = join(persistentDir, 'node_modules');
  
  // Create directories
  if (!existsSync(persistentDir)) {
    mkdirSync(persistentDir, { recursive: true });
  }
  if (!existsSync(chunksDir)) {
    mkdirSync(chunksDir, { recursive: true });
  }

  // Copy cli.js
  const sourceCli = getCliSourcePath();
  const targetCli = join(persistentDir, 'cli.js');
  copyFileSync(sourceCli, targetCli);

  // Copy chunks directory contents
  const sourceChunks = getChunksSourcePath();
  if (existsSync(sourceChunks)) {
    const files = readdirSync(sourceChunks);
    for (const file of files) {
      copyFileSync(join(sourceChunks, file), join(chunksDir, file));
    }
  }

  // Copy native modules (better-sqlite3) required by CLI
  const sourceNodeModules = getNativeModulesPath();
  const nativeModules = ['better-sqlite3'];
  
  for (const moduleName of nativeModules) {
    const srcModule = join(sourceNodeModules, moduleName);
    const destModule = join(nodeModulesDir, moduleName);
    
    if (existsSync(srcModule)) {
      // Remove existing module directory to ensure clean copy
      if (existsSync(destModule)) {
        rmSync(destModule, { recursive: true, force: true });
      }
      copyDirRecursive(srcModule, destModule);
    }
  }

  // Create a minimal package.json so Node can resolve the modules
  const packageJson = {
    name: 'openrunner-cli',
    type: 'module',
    dependencies: {
      'better-sqlite3': '*'
    }
  };
  writeFileSync(join(persistentDir, 'package.json'), JSON.stringify(packageJson, null, 2));

  return targetCli;
}

/**
 * Remove CLI files from persistent location
 */
function removePersistentCliFiles(): void {
  const persistentDir = getPersistentCliDir();
  if (existsSync(persistentDir)) {
    try {
      rmSync(persistentDir, { recursive: true, force: true });
    } catch {
      // Ignore errors
    }
  }
}

/**
 * Check if the CLI is currently installed
 */
export function isCliInstalled(): boolean {
  const targetPath = getCliTargetPath();
  return existsSync(targetPath);
}

/**
 * Get CLI installation status details
 */
export function getCliStatus(): CliInstallResult {
  const targetPath = getCliTargetPath();
  const installed = existsSync(targetPath);
  
  return {
    success: true,
    message: installed ? 'CLI is installed' : 'CLI is not installed',
    installed,
    path: installed ? targetPath : undefined,
  };
}

/**
 * Install the CLI command
 */
export async function installCli(): Promise<CliInstallResult> {
  const sourcePath = getCliSourcePath();
  const targetPath = getCliTargetPath();

  // Verify source exists
  if (!existsSync(sourcePath)) {
    return {
      success: false,
      message: `CLI source not found at ${sourcePath}. Please reinstall the application.`,
      installed: false,
    };
  }

  // For AppImage or dev mode with temporary paths, copy CLI to persistent location
  let cliPath = sourcePath;
  if (isAppImage() || is.dev) {
    try {
      cliPath = copyCliToPersistentLocation();
    } catch (error) {
      return {
        success: false,
        message: `Failed to copy CLI files: ${error instanceof Error ? error.message : String(error)}`,
        installed: false,
      };
    }
  }

  if (process.platform === 'win32') {
    return installCliWindows(cliPath, targetPath);
  } else {
    return installCliUnix(cliPath, targetPath);
  }
}

/**
 * Install CLI on Unix (macOS/Linux)
 */
async function installCliUnix(sourcePath: string, targetPath: string): Promise<CliInstallResult> {
  // Create a wrapper script that invokes node with the CLI
  const wrapperScript = `#!/bin/sh
exec node "${sourcePath}" "$@"
`;
  
  // Try direct creation first (will work if user has permissions)
  try {
    if (existsSync(targetPath)) {
      unlinkSync(targetPath);
    }
    writeFileSync(targetPath, wrapperScript, { mode: 0o755 });
    
    return {
      success: true,
      message: `CLI installed successfully at ${targetPath}`,
      installed: true,
      path: targetPath,
    };
  } catch {
    // Need elevated permissions - try with pkexec/sudo
    return installCliUnixElevated(sourcePath, targetPath, wrapperScript);
  }
}

/**
 * Install CLI on Unix with elevated permissions
 */
async function installCliUnixElevated(
  sourcePath: string,
  targetPath: string,
  wrapperScript: string
): Promise<CliInstallResult> {
  return new Promise((resolve) => {
    // Create a temporary script to run with elevated permissions
    const tempScript = join(app.getPath('temp'), 'openrunner-install-cli.sh');
    const installScript = `#!/bin/sh
cat > "${targetPath}" << 'ENDOFSCRIPT'
${wrapperScript}ENDOFSCRIPT
chmod 755 "${targetPath}"
`;

    try {
      writeFileSync(tempScript, installScript, { mode: 0o755 });
    } catch (error) {
      resolve({
        success: false,
        message: `Failed to create installation script: ${error instanceof Error ? error.message : String(error)}`,
        installed: false,
      });
      return;
    }

    // Try pkexec first (works on most Linux desktops), then fallback to sudo with terminal
    const tryPkexec = () => {
      const pkexec = spawn('pkexec', ['sh', tempScript], {
        stdio: 'pipe',
      });

      pkexec.on('close', (code) => {
        // Clean up temp script
        try {
          unlinkSync(tempScript);
        } catch {}

        if (code === 0) {
          resolve({
            success: true,
            message: `CLI installed successfully at ${targetPath}`,
            installed: true,
            path: targetPath,
          });
        } else if (code === 126) {
          // User cancelled authentication
          resolve({
            success: false,
            message: 'Installation cancelled: Authentication required',
            installed: false,
            requiresSudo: true,
          });
        } else {
          // pkexec failed, try macOS osascript
          if (process.platform === 'darwin') {
            tryOsascript();
          } else {
            resolve({
              success: false,
              message: `Installation failed. You can manually install by running:\nsudo sh -c 'cat > ${targetPath} << EOF\n${wrapperScript}EOF' && sudo chmod 755 ${targetPath}`,
              installed: false,
              requiresSudo: true,
            });
          }
        }
      });

      pkexec.on('error', () => {
        // pkexec not available, try osascript on macOS
        if (process.platform === 'darwin') {
          tryOsascript();
        } else {
          // Clean up temp script
          try {
            unlinkSync(tempScript);
          } catch {}
          
          resolve({
            success: false,
            message: `Installation requires elevated permissions. Run:\nsudo sh -c 'cat > ${targetPath} << EOF\n${wrapperScript}EOF' && sudo chmod 755 ${targetPath}`,
            installed: false,
            requiresSudo: true,
          });
        }
      });
    };

    const tryOsascript = () => {
      // macOS: Use osascript to request admin privileges
      const osascript = spawn('osascript', [
        '-e',
        `do shell script "sh '${tempScript}'" with administrator privileges`,
      ], {
        stdio: 'pipe',
      });

      osascript.on('close', (code) => {
        // Clean up temp script
        try {
          unlinkSync(tempScript);
        } catch {}

        if (code === 0) {
          resolve({
            success: true,
            message: `CLI installed successfully at ${targetPath}`,
            installed: true,
            path: targetPath,
          });
        } else {
          resolve({
            success: false,
            message: 'Installation cancelled or failed. Administrator privileges required.',
            installed: false,
            requiresSudo: true,
          });
        }
      });

      osascript.on('error', () => {
        // Clean up temp script
        try {
          unlinkSync(tempScript);
        } catch {}
        
        resolve({
          success: false,
          message: `Installation requires elevated permissions. Run:\nsudo sh ${tempScript}`,
          installed: false,
          requiresSudo: true,
        });
      });
    };

    tryPkexec();
  });
}

/**
 * Install CLI on Windows
 */
async function installCliWindows(sourcePath: string, targetPath: string): Promise<CliInstallResult> {
  const targetDir = dirname(targetPath);

  try {
    // Create the scripts directory if it doesn't exist
    if (!existsSync(targetDir)) {
      mkdirSync(targetDir, { recursive: true });
    }

    // Create a batch file wrapper
    const batchScript = `@echo off
node "${sourcePath}" %*
`;

    writeFileSync(targetPath, batchScript);

    // Check if the directory is in PATH
    const currentPath = process.env.PATH || '';
    if (!currentPath.toLowerCase().includes(targetDir.toLowerCase())) {
      // Add to user PATH via setx
      try {
        // Get current user PATH
        const userPath = execSync('echo %PATH%', { encoding: 'utf-8' }).trim();
        if (!userPath.toLowerCase().includes(targetDir.toLowerCase())) {
          execSync(`setx PATH "%PATH%;${targetDir}"`, { stdio: 'pipe' });
          return {
            success: true,
            message: `CLI installed at ${targetPath}. Please restart your terminal to use the 'openrunner' command.`,
            installed: true,
            path: targetPath,
          };
        }
      } catch {
        return {
          success: true,
          message: `CLI installed at ${targetPath}. Add "${targetDir}" to your PATH manually to use the 'openrunner' command.`,
          installed: true,
          path: targetPath,
        };
      }
    }

    return {
      success: true,
      message: `CLI installed successfully at ${targetPath}`,
      installed: true,
      path: targetPath,
    };
  } catch (error) {
    return {
      success: false,
      message: `Failed to install CLI: ${error instanceof Error ? error.message : String(error)}`,
      installed: false,
    };
  }
}

/**
 * Uninstall the CLI command
 */
export async function uninstallCli(): Promise<CliInstallResult> {
  const targetPath = getCliTargetPath();

  // Also clean up persistent CLI files
  removePersistentCliFiles();

  if (!existsSync(targetPath)) {
    return {
      success: true,
      message: 'CLI is not installed',
      installed: false,
    };
  }

  if (process.platform === 'win32') {
    return uninstallCliWindows(targetPath);
  } else {
    return uninstallCliUnix(targetPath);
  }
}

/**
 * Uninstall CLI on Unix
 */
async function uninstallCliUnix(targetPath: string): Promise<CliInstallResult> {
  // Try direct removal first
  try {
    unlinkSync(targetPath);
    return {
      success: true,
      message: 'CLI uninstalled successfully',
      installed: false,
    };
  } catch {
    // Need elevated permissions
    return uninstallCliUnixElevated(targetPath);
  }
}

/**
 * Uninstall CLI on Unix with elevated permissions
 */
async function uninstallCliUnixElevated(targetPath: string): Promise<CliInstallResult> {
  return new Promise((resolve) => {
    const tryPkexec = () => {
      const pkexec = spawn('pkexec', ['rm', '-f', targetPath], {
        stdio: 'pipe',
      });

      pkexec.on('close', (code) => {
        if (code === 0) {
          resolve({
            success: true,
            message: 'CLI uninstalled successfully',
            installed: false,
          });
        } else if (code === 126) {
          resolve({
            success: false,
            message: 'Uninstallation cancelled: Authentication required',
            installed: true,
          });
        } else {
          if (process.platform === 'darwin') {
            tryOsascript();
          } else {
            resolve({
              success: false,
              message: `Uninstallation failed. Run manually: sudo rm -f ${targetPath}`,
              installed: true,
            });
          }
        }
      });

      pkexec.on('error', () => {
        if (process.platform === 'darwin') {
          tryOsascript();
        } else {
          resolve({
            success: false,
            message: `Uninstallation requires elevated permissions. Run: sudo rm -f ${targetPath}`,
            installed: true,
          });
        }
      });
    };

    const tryOsascript = () => {
      const osascript = spawn('osascript', [
        '-e',
        `do shell script "rm -f '${targetPath}'" with administrator privileges`,
      ], {
        stdio: 'pipe',
      });

      osascript.on('close', (code) => {
        if (code === 0) {
          resolve({
            success: true,
            message: 'CLI uninstalled successfully',
            installed: false,
          });
        } else {
          resolve({
            success: false,
            message: 'Uninstallation cancelled or failed. Administrator privileges required.',
            installed: true,
          });
        }
      });

      osascript.on('error', () => {
        resolve({
          success: false,
          message: `Uninstallation requires elevated permissions. Run: sudo rm -f ${targetPath}`,
          installed: true,
        });
      });
    };

    tryPkexec();
  });
}

/**
 * Uninstall CLI on Windows
 */
async function uninstallCliWindows(targetPath: string): Promise<CliInstallResult> {
  try {
    unlinkSync(targetPath);
    return {
      success: true,
      message: 'CLI uninstalled successfully',
      installed: false,
    };
  } catch (error) {
    return {
      success: false,
      message: `Failed to uninstall CLI: ${error instanceof Error ? error.message : String(error)}`,
      installed: true,
    };
  }
}
