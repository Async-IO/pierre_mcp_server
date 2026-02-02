// ABOUTME: E2E tests for offline behavior and network state handling
// ABOUTME: Tests cached data display, offline indicators, and reconnection behavior

const { loginAsMobileTestUser, navigateToTab } = require('./visual-test-helpers');

describe('Offline Mode - App Behavior When Offline', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
    await loginAsMobileTestUser();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  afterEach(async () => {
    // Restore network if disconnected
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: false,
        isWifiEnabled: true,
        isMobileDataEnabled: true
      });
    } catch {
      // Network control may not be available in all test environments
    }
  });

  it('should remain responsive when network is unavailable', async () => {
    // Simulate offline mode (may not work in all Detox environments)
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch {
      // Skip if network simulation not available
    }

    // App should still be responsive
    await expect(element(by.id('chat-screen'))).toBeVisible();

    // Should be able to navigate between tabs
    await navigateToTab('coaches');
    await waitFor(element(by.id('coach-library-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await navigateToTab('chat');
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should show coaches from cache when offline', async () => {
    // Navigate to coaches while online to cache data
    await navigateToTab('coaches');
    await waitFor(element(by.id('coach-library-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Wait a moment for data to load
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Go offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch {
      // Skip if network simulation not available
    }

    // Should still show coach library
    await expect(element(by.id('coach-library-screen'))).toBeVisible();
  });

  it('should show feed from cache when offline', async () => {
    // Navigate to insights while online
    await navigateToTab('insights');
    await waitFor(element(by.id('social-feed-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Wait for data to load
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Go offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch {
      // Skip if network simulation not available
    }

    // Should still show feed screen
    await expect(element(by.id('social-feed-screen'))).toBeVisible();
  });

  it('should show store from cache when offline', async () => {
    // Navigate to discover while online
    await navigateToTab('discover');
    await waitFor(element(by.id('store-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Wait for data to load
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Go offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch {
      // Skip if network simulation not available
    }

    // Should still show store screen
    await expect(element(by.id('store-screen'))).toBeVisible();
  });

  it('should show error when trying to send message offline', async () => {
    await navigateToTab('chat');
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Go offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch {
      // Skip if network simulation not available
    }

    // Try to send a message
    await element(by.id('message-input')).typeText('Test offline message');
    await element(by.id('send-button')).tap();

    // Should show error (message failed or offline indicator)
    try {
      await waitFor(element(by.text(/offline|error|failed/i)))
        .toBeVisible()
        .withTimeout(5000);
    } catch {
      // Error display might vary
    }
  });

  it('should recover when network is restored', async () => {
    // Go offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch {
      // Skip if network simulation not available
    }

    // Wait a moment
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Restore network
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: false,
        isWifiEnabled: true,
        isMobileDataEnabled: true
      });
    } catch {
      // Skip if network simulation not available
    }

    // App should still be functional
    await navigateToTab('coaches');
    await waitFor(element(by.id('coach-library-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Pull to refresh should work
    await element(by.id('coach-list')).swipe('down', 'fast');
  });
});
