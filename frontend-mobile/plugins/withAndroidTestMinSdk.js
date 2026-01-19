// ABOUTME: Expo config plugin to fix minSdkVersion in library androidTest manifests
// ABOUTME: Adds tools:overrideLibrary to allow running tests with newer AndroidX libraries

const { withDangerousMod } = require('@expo/config-plugins');
const fs = require('fs');
const path = require('path');

/**
 * Expo config plugin to fix minSdkVersion conflicts in androidTest builds.
 * The @react-native-voice/voice library and other older libraries have
 * androidTest manifests with low minSdkVersion that conflicts with AndroidX.
 * This plugin creates an override in the app's androidTest manifest.
 */
const withAndroidTestMinSdk = (config) => {
  return withDangerousMod(config, [
    'android',
    async (config) => {
      const androidTestManifestDir = path.join(
        config.modRequest.platformProjectRoot,
        'app',
        'src',
        'androidTest'
      );
      const androidTestManifestPath = path.join(androidTestManifestDir, 'AndroidManifest.xml');

      // Create the androidTest directory if it doesn't exist
      if (!fs.existsSync(androidTestManifestDir)) {
        fs.mkdirSync(androidTestManifestDir, { recursive: true });
      }

      // Create or update the androidTest manifest with overrideLibrary
      const manifest = `<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:tools="http://schemas.android.com/tools">

    <!-- Override minSdkVersion for libraries with outdated test manifests -->
    <uses-sdk
        android:minSdkVersion="24"
        tools:overrideLibrary="androidx.appcompat.resources,androidx.appcompat,androidx.core,android.support.v4,com.reactnativecommunity.voice" />

    <application tools:replace="android:appComponentFactory"
        android:appComponentFactory="androidx.core.app.CoreComponentFactory">
    </application>
</manifest>
`;

      fs.writeFileSync(androidTestManifestPath, manifest);
      console.log('Created androidTest manifest with minSdkVersion override');

      return config;
    },
  ]);
};

module.exports = withAndroidTestMinSdk;
