// ABOUTME: Expo config plugin to fix Android dependency conflicts
// ABOUTME: Forces AndroidX dependencies and excludes legacy Support library

const { withProjectBuildGradle } = require('@expo/config-plugins');

/**
 * Expo config plugin to fix Android dependency resolution conflicts.
 * Forces all dependencies to use AndroidX versions and excludes conflicting
 * legacy Android Support library modules.
 */
const withAndroidDependencyFix = (config) => {
  return withProjectBuildGradle(config, async (config) => {
    const buildGradle = config.modResults.contents;

    // Check if we already have the fix
    if (buildGradle.includes('// AndroidX dependency resolution fix')) {
      return config;
    }

    // Add configuration to force AndroidX and exclude legacy Support libraries
    const dependencyFix = `
// AndroidX dependency resolution fix
allprojects {
    configurations.all {
        resolutionStrategy {
            // Force AndroidX versions
            force 'androidx.core:core:1.16.0'
            force 'androidx.versionedparcelable:versionedparcelable:1.2.0'

            // Exclude legacy Android Support libraries that conflict with AndroidX
            exclude group: 'com.android.support', module: 'support-compat'
            exclude group: 'com.android.support', module: 'animated-vector-drawable'
            exclude group: 'com.android.support', module: 'support-vector-drawable'
            exclude group: 'com.android.support', module: 'versionedparcelable'
        }
    }
}
`;

    // Find the position after the last buildscript block or after allprojects
    // Insert our fix at the end of the file
    if (buildGradle.includes('allprojects {')) {
      // Find existing allprojects block and add to it
      config.modResults.contents = buildGradle.replace(
        /allprojects\s*\{/,
        `allprojects {
    configurations.all {
        resolutionStrategy {
            // Force AndroidX versions
            force 'androidx.core:core:1.16.0'
            force 'androidx.versionedparcelable:versionedparcelable:1.2.0'

            // Exclude legacy Android Support libraries that conflict with AndroidX
            exclude group: 'com.android.support', module: 'support-compat'
            exclude group: 'com.android.support', module: 'animated-vector-drawable'
            exclude group: 'com.android.support', module: 'support-vector-drawable'
            exclude group: 'com.android.support', module: 'versionedparcelable'
        }
    }`
      );
    } else {
      // Add at the end of the file
      config.modResults.contents = buildGradle + dependencyFix;
    }

    return config;
  });
};

module.exports = withAndroidDependencyFix;
