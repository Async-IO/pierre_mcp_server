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

    await element(by.id('login-button')).tap();

    await expect(element(by.text('Email is required'))).toBeVisible();
  });

  it('should show validation error for invalid email format', async () => {
    await element(by.id('email-input')).typeText('invalid-email');
    await element(by.id('password-input')).typeText('password123');

    await element(by.id('login-button')).tap();

    await expect(element(by.text('Please enter a valid email'))).toBeVisible();
  });

  it('should login successfully with valid credentials', async () => {
    // Use test credentials (pre-filled in dev mode)
    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123');

    await element(by.id('login-button')).tap();

    // After successful login, should navigate to chat screen
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);
  });
});
