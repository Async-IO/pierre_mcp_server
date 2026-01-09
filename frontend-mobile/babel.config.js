// ABOUTME: Babel configuration for Expo with NativeWind support
// ABOUTME: Includes react-native-reanimated plugin for gesture handling

module.exports = function(api) {
  api.cache(true);
  return {
    presets: ['babel-preset-expo'],
    plugins: [
      'nativewind/babel',
      'react-native-reanimated/plugin',
    ],
  };
};
