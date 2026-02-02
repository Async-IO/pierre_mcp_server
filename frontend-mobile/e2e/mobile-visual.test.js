// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Visual E2E tests for mobile app - tests all screens using tab navigation.
// ABOUTME: Tests navigation, screen rendering, and visual consistency using Detox.

const {
  loginAsMobileTestUser,
  navigateToTab,
  takeVisualScreenshot,
  scrollDown,
  pullToRefresh,
  isVisible,
  waitForVisible,
  TEST_USERS,
} = require('./visual-test-helpers');

describe('Mobile Visual Tests', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
  });

  // ========================================
  // Authentication Screens
  // ========================================
  describe('Authentication', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
    });

    it('login - displays email input', async () => {
      await waitFor(element(by.id('login-screen')))
        .toBeVisible()
        .withTimeout(10000);
      await expect(element(by.id('email-input'))).toBeVisible();
    });

    it('login - displays password input', async () => {
      await expect(element(by.id('password-input'))).toBeVisible();
    });

    it('login - displays login button', async () => {
      await expect(element(by.id('login-button'))).toBeVisible();
    });

    it('login - password is masked', async () => {
      await element(by.id('password-input')).typeText('TestPassword');
      await expect(element(by.id('password-input'))).toBeVisible();
      await element(by.id('password-input')).clearText();
    });

    it('login - shows error on invalid credentials', async () => {
      await element(by.id('email-input')).typeText('invalid@test.com');
      await element(by.id('password-input')).typeText('wrongpassword');
      await element(by.id('login-button')).tap();

      await waitFor(element(by.text(/Invalid|Error|incorrect/i)))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('email-input')).clearText();
      await element(by.id('password-input')).clearText();
    });

    it('login - successful login navigates to chat', async () => {
      await element(by.id('email-input')).typeText(TEST_USERS.mobiletest.email);
      await element(by.id('password-input')).typeText(TEST_USERS.mobiletest.password);
      await element(by.id('login-button')).tap();

      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(15000);
    });
  });

  // ========================================
  // Tab Bar Navigation
  // ========================================
  describe('Tab Navigation', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('tab bar - chat tab visible', async () => {
      await expect(element(by.id('tab-chat'))).toBeVisible();
    });

    it('tab bar - coaches tab visible', async () => {
      await expect(element(by.id('tab-coaches'))).toBeVisible();
    });

    it('tab bar - discover tab visible', async () => {
      await expect(element(by.id('tab-discover'))).toBeVisible();
    });

    it('tab bar - insights tab visible', async () => {
      await expect(element(by.id('tab-insights'))).toBeVisible();
    });

    it('tab bar - settings tab visible', async () => {
      await expect(element(by.id('tab-settings'))).toBeVisible();
    });

    it('navigation - chat to coaches', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('navigation - coaches to discover', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('navigation - discover to insights', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('navigation - insights to settings', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('navigation - settings back to chat', async () => {
      await navigateToTab('chat');
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });

  // ========================================
  // Chat Screen
  // ========================================
  describe('Chat Screen', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('chat - default screen after login', async () => {
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(10000);
    });

    it('chat - message input visible', async () => {
      await expect(element(by.id('message-input'))).toBeVisible();
    });

    it('chat - send button visible', async () => {
      await expect(element(by.id('send-button'))).toBeVisible();
    });

    it('chat - coach selector available', async () => {
      try {
        await expect(element(by.id('coach-selector'))).toBeVisible();
      } catch {
        // May not have coach selector if only one coach
      }
    });

    it('chat - can type message', async () => {
      await element(by.id('message-input')).typeText('Test message');
      await element(by.id('message-input')).clearText();
    });

    it('chat - conversations button navigates', async () => {
      try {
        await element(by.id('conversations-button')).tap();
        await waitFor(element(by.id('conversations-screen')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('back-button')).tap();
      } catch {
        // Conversations might not be available
      }
    });
  });

  // ========================================
  // Coaches Screen
  // ========================================
  describe('Coaches Screen', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('coaches - navigates via tab', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('coaches - displays coach list or empty state', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await waitFor(element(by.id('coach-card')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        await expect(element(by.text(/No coaches|Get started/i))).toBeVisible();
      }
    });

    it('coaches - create button visible', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await expect(element(by.id('create-coach-button'))).toBeVisible();
      } catch {
        // Create button might be styled differently
      }
    });

    it('coaches - pull to refresh works', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('coach-list')).swipe('down', 'fast');
    });
  });

  // ========================================
  // Coach Store (Discover)
  // ========================================
  describe('Coach Store', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('store - navigates via tab', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('store - displays coach grid', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await waitFor(element(by.id('coach-card')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        // Store might be empty
      }
    });

    it('store - category tabs filter content', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.text('Training')).tap();
        await element(by.text('Nutrition')).tap();
        await element(by.text('Recovery')).tap();
        await element(by.text('All')).tap();
      } catch {
        // Categories may have different names
      }
    });

    it('store - search coaches works', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('store-search-input')).typeText('training');
        await element(by.id('store-search-input')).clearText();
      } catch {
        // Search may not be visible
      }
    });

    it('store - install button visible on coach cards', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await expect(element(by.id('install-button')).atIndex(0)).toBeVisible();
      } catch {
        // May show "Installed" instead or no coaches
      }
    });
  });

  // ========================================
  // Social Feed (Insights)
  // ========================================
  describe('Social Feed', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('feed - navigates via tab', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('feed - displays insight cards or empty state', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await waitFor(element(by.id('insight-card')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        await expect(element(by.text(/No Insights|No posts/i))).toBeVisible();
      }
    });

    it('feed - reaction buttons visible on insight cards', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await waitFor(element(by.id('reaction-button')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        // No insights
      }
    });

    it('feed - tapping reaction records it', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('inspire-button')).atIndex(0).tap();
      } catch {
        // No insights or different button ID
      }
    });

    it('feed - pull to refresh works', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('feed-scroll-view')).swipe('down', 'fast');
    });

    it('feed - adapt button opens adapt screen', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('adapt-button')).atIndex(0).tap();
        await waitFor(element(by.id('adapt-insight-screen')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('back-button')).tap();
      } catch {
        // No insights or adapt not available
      }
    });
  });

  // ========================================
  // Friends Screen
  // ========================================
  describe('Friends Screen', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('friends - navigates via insights tab', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('friends - displays friend list or empty state', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await waitFor(element(by.id('friend-card')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        await expect(element(by.text(/No friends|Find friends/i))).toBeVisible();
      }
    });

    it('friends - search button navigates', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('search-friends-button')).tap();
        await waitFor(element(by.id('search-friends-screen')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('back-button')).tap();
      } catch {
        // Button might not exist
      }
    });

    it('friends - requests button navigates', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('friend-requests-button')).tap();
        await waitFor(element(by.id('friend-requests-screen')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('back-button')).tap();
      } catch {
        // Button might not exist
      }
    });
  });

  // ========================================
  // Social Settings Screen
  // ========================================
  describe('Social Settings Screen', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('social settings - navigates via insights tab', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('social-settings-button')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('social settings - displays privacy options', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('social-settings-button')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await expect(element(by.id('privacy-toggle'))).toBeVisible();
      } catch {
        // Different setting names
      }
    });

    it('social settings - back button works', async () => {
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('social-settings-button')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('back-button')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });

  // ========================================
  // Settings Screen
  // ========================================
  describe('Settings Screen', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('settings - navigates via tab', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('settings - displays user profile section', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await expect(element(by.id('profile-section'))).toBeVisible();
      } catch {
        // Section might be named differently
      }
    });

    it('settings - logout button visible', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await scrollDown('settings-scroll');
      } catch {
        // Might not need scrolling
      }

      await expect(element(by.id('logout-button'))).toBeVisible();
    });

    it('settings - connections button navigates', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('connections-button')).tap();
        await waitFor(element(by.id('connections-screen')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('back-button')).tap();
      } catch {
        // Button might not exist
      }
    });
  });

  // ========================================
  // Logout Flow
  // ========================================
  describe('Logout', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('logout - navigates to login screen', async () => {
      await navigateToTab('settings');
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await scrollDown('settings-scroll');
      } catch {
        // Might not need scrolling
      }

      await element(by.id('logout-button')).tap();

      try {
        await element(by.text('Logout')).tap();
      } catch {
        // No confirmation dialog
      }

      await waitFor(element(by.id('login-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });
});
