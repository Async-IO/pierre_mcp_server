// esbuild configuration for bundling the SDK with Node 24 target
// This bundles all TypeScript and resolves ESM/CJS module issues
import * as esbuild from 'esbuild';
import { readFileSync, mkdirSync, copyFileSync, existsSync } from 'fs';
import { resolve, dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Common build options
const commonOptions = {
  bundle: true,
  platform: 'node',
  target: 'node24', // Target Node 24 LTS (has native fetch)
  format: 'cjs',
  sourcemap: true,
  external: [
    // Keep native Node modules external - they can't be bundled
    'keytar', // Native binary module for OS keychain
  ],
  logLevel: 'info',
};

console.log('🔨 Building SDK with esbuild (Node 24 target)...\n');

// Copy OAuth templates from project root to dist/templates/
const templatesDir = join(__dirname, 'dist', 'templates');
const sourceTemplatesDir = join(__dirname, '..', 'templates');
mkdirSync(templatesDir, { recursive: true });

const templateFiles = ['oauth_success.html', 'oauth_error.html'];
for (const file of templateFiles) {
  const src = join(sourceTemplatesDir, file);
  const dest = join(templatesDir, file);
  if (existsSync(src)) {
    copyFileSync(src, dest);
    console.log(`📋 Copied ${file} to dist/templates/`);
  } else {
    console.warn(`⚠️  Template not found: ${src}`);
  }
}

// Build CLI entry point (shebang already in source file)
await esbuild.build({
  ...commonOptions,
  entryPoints: ['src/cli.ts'],
  outfile: 'dist/cli.js',
});
console.log('✅ Built dist/cli.js');

// Build main library entry point
await esbuild.build({
  ...commonOptions,
  entryPoints: ['src/index.ts'],
  outfile: 'dist/index.js',
});
console.log('✅ Built dist/index.js');

// Build bridge entry point separately (it's used as a standalone module)
await esbuild.build({
  ...commonOptions,
  entryPoints: ['src/bridge.ts'],
  outfile: 'dist/bridge.js',
});
console.log('✅ Built dist/bridge.js');

console.log('\n✨ Build completed successfully with esbuild');
