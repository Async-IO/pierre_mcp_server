// ABOUTME: E2E tests for login flow
// ABOUTME: Tests authentication with real backend using test credentials

describe('Login Flow', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
  });

  beforeEach(async () => {
    await device.reloadReactNative();
    // Wait for login screen to be visible after reload
    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);
  });

  it('should show login screen on app launch', async () => {
    await expect(element(by.id('login-screen'))).toBeVisible();
  });

  it('should show email and password inputs', async () => {
    await expect(element(by.id('email-input'))).toBeVisible();
    await expect(element(by.id('password-input'))).toBeVisible();
  });

  it('should show sign in button', async () => {
    await expect(element(by.id('login-button'))).toBeVisible();
  });

  it('should show validation error for empty email', async () => {
    // Clear any pre-filled values
    await element(by.id('email-input')).clearText();
    await element(by.id('password-input')).clearText();

    // Scroll to make login button visible and tap
    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .whileElement(by.id('login-scroll-view'))
      .scroll(100, 'down');
    await element(by.id('login-button')).tap();

    await expect(element(by.text('Email is required'))).toBeVisible();
  });

  it('should show validation error for invalid email format', async () => {
    await element(by.id('email-input')).typeText('invalid-email');
    await element(by.id('password-input')).typeText('password123');

    // Scroll to make login button visible and tap
    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .whileElement(by.id('login-scroll-view'))
      .scroll(100, 'down');
    await element(by.id('login-button')).tap();

    await expect(element(by.text('Please enter a valid email'))).toBeVisible();
  });

  // NOTE: This test requires the Pierre backend server running with test user
  // mobile@test.com / mobiletest123 in the database
  it('should login successfully with valid credentials', async () => {
    // Use test credentials
    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123\n');

    // Wait for keyboard to dismiss and button to be visible
    await waitFor(element(by.id('login-button')))
      .toBeVisible()
      .withTimeout(5000);
    await element(by.id('login-button')).tap();

    // After successful login, should navigate away from login screen
    // Using longer timeout since login involves API call
    await waitFor(element(by.id('login-screen')))
      .not.toBeVisible()
      .withTimeout(15000);
  });
});
