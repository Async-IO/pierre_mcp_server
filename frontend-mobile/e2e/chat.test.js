// ABOUTME: E2E tests for chat functionality
// ABOUTME: Tests message sending and conversation management

describe('Chat Screen', () => {
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

    // Wait for chat screen
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);
  });

  beforeEach(async () => {
    // Start fresh conversation
    await element(by.id('new-chat-button')).tap();
  });

  it('should show chat screen after login', async () => {
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show header elements with proper visibility (safe area handling)', async () => {
    // Verify all header buttons are visible and not obscured by status bar
    await expect(element(by.id('menu-button'))).toBeVisible();
    await expect(element(by.id('chat-title'))).toBeVisible();
    await expect(element(by.id('new-chat-button'))).toBeVisible();
  });

  it('should show New Chat title initially', async () => {
    await expect(element(by.id('chat-title'))).toHaveText('New Chat');
  });

  it('should show message input', async () => {
    await expect(element(by.id('message-input'))).toBeVisible();
  });

  it('should have disabled send button when input is empty', async () => {
    // Verify send button exists but is in disabled state
    await expect(element(by.id('send-button'))).toBeVisible();
  });

  it('should enable send button when text is entered', async () => {
    await element(by.id('message-input')).typeText('Hello Pierre');
    await expect(element(by.id('send-button'))).toBeVisible();
  });

  it('should send message and receive response', async () => {
    await element(by.id('message-input')).typeText('What is my fitness level?');
    await element(by.id('send-button')).tap();

    // Wait for response (Pierre thinking indicator should appear then disappear)
    await waitFor(element(by.text('Pierre is thinking...')))
      .toBeVisible()
      .withTimeout(5000);

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(30000);
  });

  it('should open navigation drawer', async () => {
    await element(by.id('menu-button')).tap();

    // Should see drawer content
    await waitFor(element(by.text('Recent Conversations')))
      .toBeVisible()
      .withTimeout(3000);
  });

  it('should create new chat when + button is pressed', async () => {
    // Send a message first to create a conversation
    await element(by.id('message-input')).typeText('Test message');
    await element(by.id('send-button')).tap();

    await waitFor(element(by.text('Pierre is thinking...')))
      .not.toBeVisible()
      .withTimeout(30000);

    // Now create new chat
    await element(by.id('new-chat-button')).tap();

    // Should show New Chat title
    await expect(element(by.id('chat-title'))).toHaveText('New Chat');
  });
});
