// ABOUTME: Babel configuration for Expo with NativeWind v4 support
// ABOUTME: Includes react-native-reanimated plugin for gesture handling

module.exports = function(api) {
  api.cache(true);
  return {
    presets: [
      ['babel-preset-expo', { jsxImportSource: 'nativewind' }],
    ],
    plugins: [
      'react-native-reanimated/plugin',
    ],
  };
};
