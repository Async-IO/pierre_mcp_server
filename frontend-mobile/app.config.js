// ABOUTME: Dynamic Expo configuration that conditionally excludes expo-dev-client for E2E testing
// ABOUTME: expo-dev-client interferes with Detox's ability to connect to the app in CI

// Check if we're building for E2E tests (set in CI workflow)
const isE2EBuild = process.env.DETOX_E2E === 'true';

// Base plugins that are always included
const basePlugins = [
  [
    'expo-build-properties',
    {
      android: {
        minSdkVersion: 24,
        compileSdkVersion: 35,
        targetSdkVersion: 35,
      },
      ios: {
        useFrameworks: 'static',
      },
    },
  ],
  [
    'expo-speech-recognition',
    {
      microphonePermission:
        'Pierre needs microphone access to capture your voice for speech-to-text transcription.',
      speechRecognitionPermission:
        'Pierre uses speech recognition to transcribe your voice messages into text queries.',
      androidSpeechServicePackages: ['com.google.android.googlequicksearchbox'],
    },
  ],
];

// expo-dev-client plugin - only include in non-E2E builds
// In E2E builds, expo-dev-client's native code interferes with Detox communication
const devClientPlugin = isE2EBuild ? [] : ['expo-dev-client'];

module.exports = {
  name: 'Pierre',
  slug: 'pierre-mobile',
  version: '1.0.0',
  orientation: 'portrait',
  icon: './assets/icon.png',
  userInterfaceStyle: 'dark',
  newArchEnabled: true,
  scheme: 'pierre',
  splash: {
    image: './assets/splash-icon.png',
    resizeMode: 'contain',
    backgroundColor: '#0f0f0f',
  },
  ios: {
    supportsTablet: true,
    bundleIdentifier: 'com.pierre.fitness',
    infoPlist: {
      ITSAppUsesNonExemptEncryption: false,
      NSMicrophoneUsageDescription:
        'Pierre needs microphone access to capture your voice for speech-to-text transcription.',
      NSSpeechRecognitionUsageDescription:
        'Pierre uses speech recognition to transcribe your voice messages into text queries.',
    },
  },
  android: {
    adaptiveIcon: {
      foregroundImage: './assets/adaptive-icon.png',
      backgroundColor: '#0f0f0f',
    },
    edgeToEdgeEnabled: true,
    package: 'com.pierre.fitness',
    permissions: ['android.permission.RECORD_AUDIO'],
  },
  web: {
    favicon: './assets/favicon.png',
  },
  owner: 'dravr',
  extra: {
    eas: {
      projectId: '6c63325e-a0da-49cf-b4bd-e6aeae6fd981',
    },
    // Flag to indicate if this is an E2E build (for runtime checks if needed)
    isE2EBuild,
  },
  plugins: [...basePlugins, ...devClientPlugin],
};
