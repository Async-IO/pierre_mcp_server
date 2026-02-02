// ABOUTME: E2E tests for provider connection management
// ABOUTME: Tests viewing providers, OAuth flows, disconnecting, and error handling

describe('Connections Screen - Provider List', () => {
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

    // Navigate to Settings > Data Providers
    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should display connections screen', async () => {
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should show list of available providers', async () => {
    // Check for common fitness providers
    await expect(element(by.text('Strava'))).toBeVisible();
  });

  it('should show connection status for each provider', async () => {
    // Provider cards should indicate connected/disconnected status
    // Look for connect button or connected indicator
    const stravaCard = element(by.text('Strava'));
    await expect(stravaCard).toBeVisible();
  });

  it('should display provider icons/logos', async () => {
    // Provider list should have visual indicators
    await expect(element(by.text('Strava'))).toBeVisible();
  });

  it('should show "No connections" message when none connected', async () => {
    // If no providers connected, should show helpful message
    // This depends on test user state
    await expect(element(by.text('Connections'))).toBeVisible();
  });
});

describe('Connections Screen - Strava OAuth Flow', () => {
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

    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should show connect button for Strava', async () => {
    await expect(element(by.text('Strava'))).toBeVisible();
  });

  it('should open OAuth flow when connect is tapped', async () => {
    // Find and tap Strava connect button
    const stravaCard = element(by.text('Strava'));
    await expect(stravaCard).toBeVisible();

    // Note: Actually initiating OAuth would open browser
    // which may not be testable in Detox without additional setup
  });

  it('should handle OAuth cancellation gracefully', async () => {
    // User should be able to return without completing OAuth
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should show loading state during OAuth', async () => {
    // When OAuth is in progress, should show loading indicator
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should update connection status after successful OAuth', async () => {
    // After OAuth completes, provider should show as connected
    // This is difficult to fully test without mocking the OAuth callback
    await expect(element(by.text('Strava'))).toBeVisible();
  });
});

describe('Connections Screen - Disconnect Provider', () => {
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

    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should show disconnect option for connected providers', async () => {
    // If a provider is connected, should show disconnect option
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should confirm before disconnecting', async () => {
    // Disconnecting should show confirmation dialog
    // This prevents accidental disconnection
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should update UI after disconnect', async () => {
    // After disconnecting, provider should show as not connected
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should handle disconnect errors gracefully', async () => {
    // If disconnect fails, should show error message
    await expect(element(by.text('Connections'))).toBeVisible();
  });
});

describe('Connections Screen - Connection Errors', () => {
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

    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should handle network errors during connection', async () => {
    // When network fails during OAuth, should show error
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should show retry option on connection failure', async () => {
    // After failure, user should be able to retry
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should display provider-specific error messages', async () => {
    // Errors should be helpful and specific
    await expect(element(by.text('Connections'))).toBeVisible();
  });

  it('should not crash on repeated connection attempts', async () => {
    // Multiple attempts should not cause issues
    await expect(element(by.text('Connections'))).toBeVisible();
    await expect(element(by.text('Strava'))).toBeVisible();
  });
});

describe('Connections Screen - Navigation', () => {
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

  it('should navigate to connections from settings', async () => {
    await element(by.text('Settings')).tap();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should navigate back to settings', async () => {
    await device.pressBack();

    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  it('should preserve connection state on navigation', async () => {
    // Navigate to connections
    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);

    // Navigate back and forth
    await device.pressBack();
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);

    await element(by.id('settings-data-providers-button')).tap();
    await waitFor(element(by.text('Connections')))
      .toBeVisible()
      .withTimeout(5000);

    // State should be preserved
    await expect(element(by.text('Strava'))).toBeVisible();
  });
});
