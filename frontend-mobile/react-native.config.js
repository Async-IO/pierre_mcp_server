// ABOUTME: React Native configuration for autolinking control
// ABOUTME: Excludes expo-dev-client in E2E builds to allow Detox communication

// Check if we're building for E2E tests (set in CI workflow)
const isE2EBuild = process.env.DETOX_E2E === 'true';

// Packages to exclude from autolinking during E2E builds
// expo-dev-client interferes with Detox's WebSocket communication
const e2eExcludedPackages = isE2EBuild
  ? {
      'expo-dev-client': { platforms: { ios: null, android: null } },
      'expo-dev-launcher': { platforms: { ios: null, android: null } },
      'expo-dev-menu': { platforms: { ios: null, android: null } },
      'expo-dev-menu-interface': { platforms: { ios: null, android: null } },
    }
  : {};

module.exports = {
  dependencies: {
    ...e2eExcludedPackages,
  },
};
