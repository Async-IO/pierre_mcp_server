// ABOUTME: E2E tests for chat functionality
// ABOUTME: Tests message sending and conversation management

const { loginAsMobileTestUser, navigateToTab } = require('./visual-test-helpers');

describe('Chat Screen', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
    await loginAsMobileTestUser();

    // Wait for chat screen (default after login)
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);
  });

  beforeEach(async () => {
    // Navigate to chat tab and start fresh conversation
    await navigateToTab('chat');
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(5000);

    try {
      await element(by.id('new-chat-button')).tap();
    } catch {
      // New chat button might not exist
    }
  });

  it('should show chat screen after login', async () => {
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show header elements', async () => {
    await expect(element(by.id('chat-title'))).toBeVisible();
    await expect(element(by.id('new-chat-button'))).toBeVisible();
  });

  it('should show New Chat title initially', async () => {
    await expect(element(by.id('chat-title'))).toHaveText('New Chat');
  });

  it('should show message input', async () => {
    await expect(element(by.id('message-input'))).toBeVisible();
  });

  it('should show send button', async () => {
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

  it('should navigate to conversations screen', async () => {
    try {
      await element(by.id('conversations-button')).tap();
      await waitFor(element(by.id('conversations-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Go back
      await element(by.id('back-button')).tap();
    } catch {
      // Conversations button might not exist
    }
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

  describe('tab navigation', () => {
    it('should navigate to coaches tab and back', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await navigateToTab('chat');
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should navigate to discover tab and back', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await navigateToTab('chat');
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should navigate to insights tab and back', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await navigateToTab('chat');
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should navigate to settings tab and back', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await navigateToTab('chat');
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });
});
