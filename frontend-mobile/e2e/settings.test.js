// ABOUTME: E2E tests for Settings screen functionality
// ABOUTME: Tests screen rendering, expected sections, and absence of hallucinated UI elements

describe('Settings Screen', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    // Wait for login screen to be visible
    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    // Login first
    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    // Wait for keyboard to dismiss and button to be visible
    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

    // Wait for chat screen to load first
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);

    // Navigate to Settings via bottom tab
    await element(by.text('Settings')).tap();

    // Wait for settings screen
    await waitFor(element(by.id('settings-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  beforeEach(async () => {
    // Ensure we're on the settings screen
    try {
      await expect(element(by.id('settings-screen'))).toBeVisible();
    } catch (error) {
      // Navigate back if not on the settings screen
      await element(by.text('Settings')).tap();
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    }
  });

  describe('screen rendering - prevents blank screen regression', () => {
    it('should show settings screen (not blank)', async () => {
      await expect(element(by.id('settings-screen'))).toBeVisible();
    });

    it('should show profile section with user info', async () => {
      await expect(element(by.id('settings-profile-section'))).toBeVisible();
    });

    it('should show Edit Profile button', async () => {
      await expect(element(by.text('Edit Profile'))).toBeVisible();
    });
  });

  describe('expected sections - data from backend', () => {
    it('should show Data section', async () => {
      await expect(element(by.id('settings-data-section'))).toBeVisible();
      await expect(element(by.text('Data'))).toBeVisible();
    });

    it('should show Data Providers button', async () => {
      await expect(element(by.id('settings-data-providers-button'))).toBeVisible();
      await expect(element(by.text('Data Providers'))).toBeVisible();
    });

    it('should show Account section', async () => {
      await expect(element(by.id('settings-account-section'))).toBeVisible();
      await expect(element(by.text('Account'))).toBeVisible();
    });

    it('should show Personal Information option', async () => {
      await expect(element(by.text('Personal Information'))).toBeVisible();
    });

    it('should show Change Password option', async () => {
      await expect(element(by.text('Change Password'))).toBeVisible();
    });

    it('should show MCP Tokens option', async () => {
      await expect(element(by.text('MCP Tokens'))).toBeVisible();
    });

    it('should show Privacy section', async () => {
      await expect(element(by.text('Privacy'))).toBeVisible();
    });

    it('should show About section', async () => {
      await expect(element(by.text('About'))).toBeVisible();
    });

    it('should show Log Out button', async () => {
      await expect(element(by.id('settings-logout-button'))).toBeVisible();
      await expect(element(by.text('Log Out'))).toBeVisible();
    });
  });

  describe('hallucinated elements - must NOT exist', () => {
    // These tests ensure we don't accidentally add UI elements that don't have backend support
    // If a test fails, it means a hallucinated element was added - remove it or implement backend first

    it('should NOT show Apple Health (not implemented)', async () => {
      await expect(element(by.text('Apple Health'))).not.toBeVisible();
    });

    it('should NOT show Export Data (not implemented)', async () => {
      await expect(element(by.text('Export Data'))).not.toBeVisible();
    });

    it('should NOT show Push Notifications (not implemented - ASY-355)', async () => {
      await expect(element(by.text('Push Notifications'))).not.toBeVisible();
    });

    it('should NOT show Email Updates (not implemented - ASY-356)', async () => {
      await expect(element(by.text('Email Updates'))).not.toBeVisible();
    });

    it('should NOT show Notifications section (not implemented)', async () => {
      // Check that there's no "Notifications" section header
      await expect(element(by.text('Notifications')).atIndex(0)).not.toBeVisible();
    });

    it('should NOT show hardcoded user stats (activities, hours, insights)', async () => {
      // These were previously hardcoded values that don't come from backend
      await expect(element(by.text('127'))).not.toBeVisible();
      await expect(element(by.text('89'))).not.toBeVisible();
      await expect(element(by.text('12'))).not.toBeVisible();
    });
  });

  describe('Data Providers navigation', () => {
    it('should navigate to Connections screen when Data Providers is tapped', async () => {
      await element(by.id('settings-data-providers-button')).tap();

      // Should navigate to Connections screen
      await waitFor(element(by.text('Connections')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate back
      await device.pressBack();

      // Should be back on settings
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });
});
