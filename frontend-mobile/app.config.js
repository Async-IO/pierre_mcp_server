// ABOUTME: Expo configuration for Pierre mobile app
// ABOUTME: Uses Expo Go for development; native builds only needed for speech recognition testing

module.exports = {
  name: 'Pierre',
  slug: 'pierre-mobile',
  version: '1.0.0',
  runtimeVersion: {
    policy: 'sdkVersion',
  },
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
    bundleIdentifier: 'ai.dravr.app.dev',
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
    package: 'ai.dravr.app.dev',
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
  },
  plugins: [
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
  ],
};
