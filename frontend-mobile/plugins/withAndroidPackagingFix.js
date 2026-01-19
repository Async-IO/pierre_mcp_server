// ABOUTME: Expo config plugin to fix Android resource packaging conflicts
// ABOUTME: Excludes duplicate META-INF files from AndroidX and Android Support libraries

const { withAppBuildGradle } = require('@expo/config-plugins');

/**
 * Expo config plugin to fix Android resource packaging conflicts.
 * Adds packaging options to exclude duplicate META-INF files that occur
 * when both AndroidX and legacy Android Support libraries are present.
 */
const withAndroidPackagingFix = (config) => {
  return withAppBuildGradle(config, async (config) => {
    const buildGradle = config.modResults.contents;

    // Check if we already have the fix
    if (buildGradle.includes('// META-INF duplicate exclusion fix')) {
      return config;
    }

    // Find the android { block and add packaging options
    const packagingFix = `
        // META-INF duplicate exclusion fix
        // Resolves conflicts between AndroidX and legacy Android Support libraries
        packagingOptions {
            resources {
                excludes += ['META-INF/*.version']
                excludes += ['META-INF/LICENSE*']
                excludes += ['META-INF/NOTICE*']
                excludes += ['META-INF/INDEX.LIST']
                excludes += ['META-INF/DEPENDENCIES']
                excludes += ['META-INF/AL2.0']
                excludes += ['META-INF/LGPL2.1']
                pickFirsts += ['**/libc++_shared.so']
                pickFirsts += ['**/libfbjni.so']
            }
        }`;

    // Find the closing brace of the android block and insert before it
    // Look for pattern like "android {" and add our code inside
    if (buildGradle.includes('android {')) {
      // Find the packagingOptions block if it exists
      if (buildGradle.includes('packagingOptions {')) {
        // Add exclusions to existing packagingOptions block
        config.modResults.contents = buildGradle.replace(
          /packagingOptions\s*\{/,
          `packagingOptions {
        // META-INF duplicate exclusion fix
        resources {
            excludes += ['META-INF/*.version']
            excludes += ['META-INF/LICENSE*']
            excludes += ['META-INF/NOTICE*']
            excludes += ['META-INF/INDEX.LIST']
            excludes += ['META-INF/DEPENDENCIES']
            excludes += ['META-INF/AL2.0']
            excludes += ['META-INF/LGPL2.1']
            pickFirsts += ['**/libc++_shared.so']
            pickFirsts += ['**/libfbjni.so']
        }`
        );
      } else {
        // Add new packagingOptions block before the last closing brace of android block
        // Find android block end and add before it
        const androidBlockRegex = /(android\s*\{[\s\S]*?)(\n\}[\s\S]*?(?:dependencies|$))/;
        const match = buildGradle.match(androidBlockRegex);
        if (match) {
          config.modResults.contents = buildGradle.replace(
            androidBlockRegex,
            `$1${packagingFix}\n$2`
          );
        }
      }
    }

    return config;
  });
};

module.exports = withAndroidPackagingFix;
