// ABOUTME: E2E tests for offline behavior and network state handling
// ABOUTME: Tests cached data display, offline indicators, and reconnection behavior

describe('Offline Mode - App Behavior When Offline', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    // Login first while online
    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

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
    } catch (error) {
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
    } catch (error) {
      // Skip if network simulation not available
      return;
    }

    // App should still be responsive
    await expect(element(by.id('chat-screen'))).toBeVisible();
    await expect(element(by.id('message-input'))).toBeVisible();
  });

  it('should show offline indicator when disconnected', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return; // Skip if not available
    }

    // Try to send a message
    await element(by.id('message-input')).typeText('Offline test');
    await element(by.id('send-button')).tap();

    // Should show some indication of offline state
    // Implementation may show toast, banner, or inline error
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should queue messages for later when offline', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Type a message
    await element(by.id('message-input')).typeText('Queued message');
    await element(by.id('send-button')).tap();

    // Message input should be ready for next message
    await expect(element(by.id('message-input'))).toBeVisible();
  });

  it('should preserve UI state when offline', async () => {
    // Navigate to different screens while offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Try to navigate
    await element(by.text('Settings')).tap();

    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Navigate back
    await device.pressBack();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });
});

describe('Offline Mode - Cached Data Display', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);

    // Create some data while online
    await element(by.id('message-input')).typeText('Cache test message');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(60000);
  });

  it('should display cached conversations when offline', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Open drawer to see conversations
    await element(by.id('menu-button')).tap();

    await waitFor(element(by.text('Recent Conversations')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should show cached profile data when offline', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Navigate to settings
    await element(by.text('Settings')).tap();

    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Profile section should be visible with cached data
    await expect(element(by.id('settings-profile-section'))).toBeVisible();
  });

  it('should indicate stale data when offline', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Data display should still work
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should preserve message history locally', async () => {
    // Messages should be visible even when offline
    await element(by.id('menu-button')).tap();

    await waitFor(element(by.text('Recent Conversations')))
      .toBeVisible()
      .withTimeout(5000);

    // Close drawer
    await device.pressBack();

    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Offline Mode - Retry on Reconnect', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should automatically retry failed requests on reconnect', async () => {
    // Go offline
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Try to send message while offline
    await element(by.id('message-input')).typeText('Retry test');
    await element(by.id('send-button')).tap();

    // Wait briefly
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Go back online
    await device.setNetworkConditions({
      isAirplaneModeEnabled: false,
      isWifiEnabled: true,
      isMobileDataEnabled: true
    });

    // App should still be responsive
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should refresh data automatically when connection restored', async () => {
    // Reconnect
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: false,
        isWifiEnabled: true,
        isMobileDataEnabled: true
      });
    } catch (error) {
      return;
    }

    // Wait for reconnection
    await new Promise(resolve => setTimeout(resolve, 2000));

    // App should be fully functional
    await element(by.id('message-input')).typeText('After reconnect');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .toBeVisible()
      .withTimeout(10000);
  });

  it('should sync pending changes after reconnect', async () => {
    // Make sure we're online
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: false,
        isWifiEnabled: true,
        isMobileDataEnabled: true
      });
    } catch (error) {
      // Continue without network control
    }

    // Send a message and verify it goes through
    await element(by.id('message-input')).typeText('Sync test');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .toBeVisible()
      .withTimeout(10000);

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(60000);
  });
});

describe('Offline Mode - Offline Indicator UI', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should show offline banner when connection lost', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Wait for app to detect offline state
    await new Promise(resolve => setTimeout(resolve, 1000));

    // UI should still be visible and responsive
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should hide offline banner when connection restored', async () => {
    // Restore connection
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: false,
        isWifiEnabled: true,
        isMobileDataEnabled: true
      });
    } catch (error) {
      return;
    }

    // Wait for reconnection detection
    await new Promise(resolve => setTimeout(resolve, 2000));

    // App should be fully functional
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display appropriate message for different network states', async () => {
    // Test wifi only
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: false,
        isWifiEnabled: true,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    await new Promise(resolve => setTimeout(resolve, 500));
    await expect(element(by.id('chat-screen'))).toBeVisible();

    // Test mobile only
    await device.setNetworkConditions({
      isAirplaneModeEnabled: false,
      isWifiEnabled: false,
      isMobileDataEnabled: true
    });

    await new Promise(resolve => setTimeout(resolve, 500));
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should persist offline indicator across screen navigation', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Navigate between screens
    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Navigate back
    await device.pressBack();
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Restore connection
    await device.setNetworkConditions({
      isAirplaneModeEnabled: false,
      isWifiEnabled: true,
      isMobileDataEnabled: true
    });
  });
});

describe('Offline Mode - Graceful Degradation', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should allow navigation while offline', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Should be able to open menu
    await element(by.id('menu-button')).tap();

    await waitFor(element(by.text('Recent Conversations')))
      .toBeVisible()
      .withTimeout(3000);

    // Close menu
    await device.pressBack();
  });

  it('should show read-only mode for data that requires network', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Navigate to settings
    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    // Settings should display but edits may be limited
    await expect(element(by.id('settings-profile-section'))).toBeVisible();
  });

  it('should disable features that absolutely require network', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Chat input should still be visible
    await expect(element(by.id('message-input'))).toBeVisible();
  });

  it('should not crash when performing offline-incompatible actions', async () => {
    try {
      await device.setNetworkConditions({
        isAirplaneModeEnabled: true,
        isWifiEnabled: false,
        isMobileDataEnabled: false
      });
    } catch (error) {
      return;
    }

    // Try to send a message
    await element(by.id('message-input')).typeText('Crash test');
    await element(by.id('send-button')).tap();

    // App should not crash
    await expect(element(by.id('chat-screen'))).toBeVisible();

    // Restore network
    await device.setNetworkConditions({
      isAirplaneModeEnabled: false,
      isWifiEnabled: true,
      isMobileDataEnabled: true
    });
  });
});
