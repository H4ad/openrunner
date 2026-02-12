/**
 * CLI Installer Service
 * 
 * Handles installation and uninstallation of the CLI command.
 * - macOS/Linux: Creates symlink at /usr/local/bin/openrunner
 * - Windows: Creates batch file in user's AppData Scripts folder and adds to PATH
 */

import { app } from 'electron';
import { existsSync, unlinkSync, symlinkSync, writeFileSync, readFileSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { execSync, spawn } from 'child_process';
import { is } from '@electron-toolkit/utils';

export interface CliInstallResult {
  success: boolean;
  message: string;
  installed: boolean;
  path?: string;
  requiresSudo?: boolean;
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

  if (process.platform === 'win32') {
    return installCliWindows(sourcePath, targetPath);
  } else {
    return installCliUnix(sourcePath, targetPath);
  }
}

/**
 * Install CLI on Unix (macOS/Linux)
 */
async function installCliUnix(sourcePath: string, targetPath: string): Promise<CliInstallResult> {
  const targetDir = dirname(targetPath);

  // Check if we can write to target directory
  try {
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
    } catch (directError) {
      // Need elevated permissions - try with pkexec/sudo
      return installCliUnixElevated(sourcePath, targetPath, wrapperScript);
    }
  } catch (error) {
    return {
      success: false,
      message: `Failed to install CLI: ${error instanceof Error ? error.message : String(error)}`,
      installed: false,
      requiresSudo: true,
    };
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
  try {
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
  } catch (error) {
    return {
      success: false,
      message: `Failed to uninstall CLI: ${error instanceof Error ? error.message : String(error)}`,
      installed: true,
    };
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
