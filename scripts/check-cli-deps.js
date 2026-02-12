#!/usr/bin/env node
/**
 * CLI Dependencies Check Script
 * 
 * This script verifies that all CLI dependencies are properly bundled.
 * It checks the built CLI output for external imports that aren't:
 * 1. Node.js built-in modules
 * 2. Native modules that are explicitly copied (like better-sqlite3)
 * 
 * Run after build: node scripts/check-cli-deps.js
 */

import { readFileSync, existsSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { builtinModules } from 'module';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, '..');

// Native modules that are copied separately by the CLI installer
// These are allowed to remain external
const NATIVE_MODULES = [
  'better-sqlite3',
];

// Node.js built-in modules (with and without 'node:' prefix)
const BUILTIN_MODULES = new Set([
  ...builtinModules,
  ...builtinModules.map(m => `node:${m}`),
]);

/**
 * Extract external imports from a JS file
 * Looks for: import ... from "package" or require("package")
 */
function extractExternalImports(content) {
  const imports = new Set();
  
  // Match ES module imports: import ... from "package"
  const esImportRegex = /import\s+(?:[^'"]*\s+from\s+)?['"]([^'"./][^'"]*)['"]/g;
  let match;
  while ((match = esImportRegex.exec(content)) !== null) {
    imports.add(match[1]);
  }
  
  // Match require calls: require("package")
  const requireRegex = /require\s*\(\s*['"]([^'"./][^'"]*)['"]\s*\)/g;
  while ((match = requireRegex.exec(content)) !== null) {
    imports.add(match[1]);
  }
  
  return imports;
}

/**
 * Get the package name from an import (handles scoped packages)
 */
function getPackageName(importPath) {
  if (importPath.startsWith('@')) {
    // Scoped package: @scope/package/path -> @scope/package
    const parts = importPath.split('/');
    return parts.slice(0, 2).join('/');
  }
  // Regular package: package/path -> package
  return importPath.split('/')[0];
}

function main() {
  const outDir = join(rootDir, 'out', 'main');
  const cliPath = join(outDir, 'cli.js');
  const chunksDir = join(outDir, 'chunks');
  
  if (!existsSync(cliPath)) {
    console.error('Error: CLI not built. Run `pnpm build` first.');
    process.exit(1);
  }
  
  console.log('Checking CLI dependencies...\n');
  
  const filesToCheck = [cliPath];
  
  // Add chunk files
  if (existsSync(chunksDir)) {
    const chunks = readdirSync(chunksDir).filter(f => f.endsWith('.js'));
    for (const chunk of chunks) {
      filesToCheck.push(join(chunksDir, chunk));
    }
  }
  
  const allExternalImports = new Set();
  
  for (const file of filesToCheck) {
    const content = readFileSync(file, 'utf-8');
    const imports = extractExternalImports(content);
    
    for (const imp of imports) {
      const pkgName = getPackageName(imp);
      
      // Skip Node.js built-ins
      if (BUILTIN_MODULES.has(imp) || BUILTIN_MODULES.has(pkgName)) {
        continue;
      }
      
      // Skip allowed native modules
      if (NATIVE_MODULES.includes(pkgName)) {
        continue;
      }
      
      allExternalImports.add(pkgName);
    }
  }
  
  if (allExternalImports.size > 0) {
    console.error('ERROR: Found unbundled external dependencies in CLI:\n');
    for (const dep of allExternalImports) {
      console.error(`  - ${dep}`);
    }
    console.error('\nTo fix this, add these packages to the `exclude` array in');
    console.error('electron.vite.config.ts under externalizeDepsPlugin():\n');
    console.error('  externalizeDepsPlugin({');
    console.error('    exclude: [...existing, ' + Array.from(allExternalImports).map(d => `'${d}'`).join(', ') + '],');
    console.error('  }),\n');
    console.error('If a package contains native code (like better-sqlite3), add it to');
    console.error('NATIVE_MODULES in scripts/check-cli-deps.js and update');
    console.error('src-electron/main/services/cli-installer.ts to copy it.\n');
    process.exit(1);
  }
  
  console.log('✓ All CLI dependencies are properly bundled or accounted for.');
  console.log(`  - Bundled into CLI: (checked ${filesToCheck.length} files)`);
  console.log(`  - Native modules (copied separately): ${NATIVE_MODULES.join(', ')}`);
}

main();
