// ABOUTME: Metro bundler configuration for Expo with NativeWind v4 and npm workspaces
// ABOUTME: Enables CSS support for Tailwind-style classes and resolves workspace packages

const { getDefaultConfig } = require('expo/metro-config');
const { withNativeWind } = require('nativewind/metro');
const path = require('path');

// Get the project root (monorepo root)
const projectRoot = __dirname;
const monorepoRoot = path.resolve(projectRoot, '..');

const config = getDefaultConfig(projectRoot);

// Watch all files in the monorepo
config.watchFolders = [monorepoRoot];

// Let Metro know where to resolve packages from
config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, 'node_modules'),
  path.resolve(monorepoRoot, 'node_modules'),
];

// Resolve packages from the monorepo root
config.resolver.disableHierarchicalLookup = false;

// Ensure @pierre/* packages resolve correctly
config.resolver.extraNodeModules = {
  '@pierre/shared-types': path.resolve(monorepoRoot, 'packages/shared-types'),
  '@pierre/shared-constants': path.resolve(monorepoRoot, 'packages/shared-constants'),
  '@pierre/api-client': path.resolve(monorepoRoot, 'packages/api-client'),
};

// Use resolveRequest to redirect @pierre/api-client to mobile-specific entry
// This avoids importing web.ts which uses import.meta (not supported by Hermes)
config.resolver.resolveRequest = (context, moduleName, platform) => {
  if (moduleName === '@pierre/api-client') {
    return {
      filePath: path.resolve(monorepoRoot, 'packages/api-client/src/index-mobile.ts'),
      type: 'sourceFile',
    };
  }
  return context.resolveRequest(context, moduleName, platform);
};

// Use port 8082 to avoid conflict with Pierre MCP server on 8081
config.server = {
  ...config.server,
  port: 8082,
};

module.exports = withNativeWind(config, { input: './global.css' });
