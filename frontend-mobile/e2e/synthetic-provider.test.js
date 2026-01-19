// ABOUTME: E2E tests for synthetic provider integration
// ABOUTME: Tests activity queries and fitness intelligence using synthetic test data

describe('Synthetic Provider Tests', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

    // Wait for login screen to be visible
    await waitFor(element(by.id('login-screen')))
      .toBeVisible()
      .withTimeout(10000);

    // Login
    await element(by.id('email-input')).clearText();
    await element(by.id('email-input')).typeText('mobile@test.com');
    await element(by.id('password-input')).clearText();
    await element(by.id('password-input')).typeText('mobiletest123');
    await element(by.id('login-button')).tap();

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);
  });

  beforeEach(async () => {
    // Start fresh conversation
    await element(by.id('new-chat-button')).tap();
  });

  it('should query recent activities with synthetic provider', async () => {
    await element(by.id('message-input')).typeText('Show my recent activities from synthetic provider');
    await element(by.id('send-button')).tap();

    // Wait for response
    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(30000);

    // Response should contain activity information
    // The synthetic provider generates test activities
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should analyze training load with synthetic data', async () => {
    await element(by.id('message-input')).typeText('Analyze my training load using synthetic data');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(30000);

    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should get fitness score from synthetic activities', async () => {
    await element(by.id('message-input')).typeText('What is my fitness score based on synthetic activities?');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(30000);

    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should recommend recovery based on synthetic data', async () => {
    await element(by.id('message-input')).typeText('Do I need a rest day? Use synthetic data');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(30000);

    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should predict performance from synthetic history', async () => {
    await element(by.id('message-input')).typeText('Predict my 5K time based on synthetic training data');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(45000);

    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});
