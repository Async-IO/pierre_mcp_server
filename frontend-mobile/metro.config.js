// ABOUTME: Metro bundler configuration for Expo with NativeWind v4
// ABOUTME: Enables CSS support for Tailwind-style classes in React Native

const { getDefaultConfig } = require('expo/metro-config');
const { withNativeWind } = require('nativewind/metro');

const config = getDefaultConfig(__dirname);

// Use port 8082 to avoid conflict with Pierre MCP server on 8081
config.server = {
  ...config.server,
  port: 8082,
};

module.exports = withNativeWind(config, { input: './global.css' });
