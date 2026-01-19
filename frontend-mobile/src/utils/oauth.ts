// ABOUTME: OAuth utility functions for mobile app
// ABOUTME: Handles OAuth callback URL generation for development and production

import Constants from 'expo-constants';

// The app scheme defined in app.json
const APP_SCHEME = 'pierre';

/**
 * Creates the OAuth callback URL for the mobile app.
 *
 * In Expo Go development, Linking.createURL() returns exp://... URLs which
 * don't work on physical devices. This function returns the custom scheme
 * URL directly (pierre://oauth-callback) which works in both development
 * and production.
 *
 * For WebBrowser.openAuthSessionAsync to work, the scheme must be registered:
 * - In Expo Go: The pierre:// scheme is handled via the app.json scheme config
 * - In standalone builds: Universal links or app links handle the redirect
 */
export function getOAuthCallbackUrl(): string {
  // Always use the custom scheme for OAuth callbacks
  // This ensures consistent behavior across dev and prod
  return `${APP_SCHEME}://oauth-callback`;
}

/**
 * Checks if running in Expo Go (development) vs standalone build
 */
export function isExpoGo(): boolean {
  return Constants.appOwnership === 'expo';
}
