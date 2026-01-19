// ABOUTME: Expo config plugin to fix Android manifest merger conflicts
// ABOUTME: Resolves AndroidX vs Android Support library conflicts

const { withAndroidManifest } = require('@expo/config-plugins');

/**
 * Expo config plugin to fix AndroidManifest merger issues.
 * This adds necessary tools:replace attributes to resolve conflicts between
 * AndroidX and legacy Android Support libraries.
 */
const withAndroidManifestFix = (config) => {
  return withAndroidManifest(config, async (config) => {
    const androidManifest = config.modResults;

    // Get the application node
    const application = androidManifest.manifest.application?.[0];

    if (application) {
      // Ensure tools namespace is present
      if (!androidManifest.manifest.$['xmlns:tools']) {
        androidManifest.manifest.$['xmlns:tools'] = 'http://schemas.android.com/tools';
      }

      // Set appComponentFactory to AndroidX version
      application.$['android:appComponentFactory'] = 'androidx.core.app.CoreComponentFactory';

      // Add tools:replace to override conflicting attributes
      const currentReplace = application.$['tools:replace'] || '';
      const replaceAttrs = new Set(currentReplace.split(',').filter(Boolean));
      replaceAttrs.add('android:appComponentFactory');
      application.$['tools:replace'] = Array.from(replaceAttrs).join(',');
    }

    return config;
  });
};

module.exports = withAndroidManifestFix;
