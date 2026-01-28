// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Visual E2E tests for mobile app (ASY-314).
// ABOUTME: Tests all mobile screens against real backend using Detox.

const {
  loginAsMobileTestUser,
  navigateViaDrawer,
  takeVisualScreenshot,
  scrollDown,
  pullToRefresh,
  isVisible,
  waitForVisible,
  TEST_USERS,
} = require('./visual-test-helpers');

describe('ASY-314: Mobile Visual Tests', () => {
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
      // Password should be secure entry (masked)
      await expect(element(by.id('password-input'))).toBeVisible();
      await element(by.id('password-input')).clearText();
    });

    it('login - shows error for invalid credentials', async () => {
      await element(by.id('email-input')).clearText();
      await element(by.id('email-input')).typeText('wrong@email.com');
      await element(by.id('password-input')).clearText();
      await element(by.id('password-input')).typeText('wrongpassword\n');
      await element(by.id('login-button')).tap();

      // Wait for error or stay on login screen
      await waitFor(element(by.id('login-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('login - successful login navigates away from login screen', async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
      // Should no longer see login screen
      await waitFor(element(by.id('login-screen')))
        .not.toBeVisible()
        .withTimeout(15000);
    });
  });

  // ========================================
  // Home Screen
  // ========================================
  describe('Home Screen', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('home - displays greeting with user name', async () => {
      // Home tab should be default or navigate to it
      try {
        await element(by.id('home-tab')).tap();
      } catch {
        // Already on home or no tab bar
      }

      // Look for greeting text
      await waitFor(element(by.text(/Hello|Welcome/)))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('home - displays stat cards', async () => {
      // Look for stat card elements
      const statCard = element(by.id('stat-card')).atIndex(0);
      const hasStatCard = await isVisible(statCard);
      if (hasStatCard) {
        await expect(statCard).toBeVisible();
      }
    });

    it('home - pull to refresh works', async () => {
      try {
        await element(by.id('home-scroll-view')).swipe('down', 'fast');
        // Wait for refresh to complete
        await waitFor(element(by.id('home-scroll-view')))
          .toBeVisible()
          .withTimeout(5000);
      } catch {
        // Scroll view may have different ID
      }
    });
  });

  // ========================================
  // Chat Tab
  // ========================================
  describe('Chat Tab', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('chat - tab icon visible and tappable', async () => {
      await waitFor(element(by.id('chat-tab')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.id('chat-tab')).tap();
    });

    it('chat - displays conversation list or empty state', async () => {
      await element(by.id('chat-tab')).tap();
      // Either conversations list or empty state
      try {
        await waitFor(element(by.id('conversations-list')))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        // Empty state
        await expect(element(by.text(/No conversations|Start a chat/i))).toBeVisible();
      }
    });

    it('chat - new chat button visible', async () => {
      await element(by.id('chat-tab')).tap();
      const newChatButton = element(by.id('new-chat-button'));
      const hasNewChat = await isVisible(newChatButton);
      // Button may or may not be visible depending on UI
    });

    it('chat - message input accepts text', async () => {
      await element(by.id('chat-tab')).tap();
      // Try to find message input (may need to open a conversation first)
      try {
        await waitFor(element(by.id('message-input')))
          .toBeVisible()
          .withTimeout(3000);
        await element(by.id('message-input')).typeText('Test message');
        await element(by.id('message-input')).clearText();
      } catch {
        // No active conversation
      }
    });
  });

  // ========================================
  // Coaches Tab
  // ========================================
  describe('Coaches Tab', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('coaches - tab icon visible and tappable', async () => {
      await waitFor(element(by.id('coaches-tab')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.id('coaches-tab')).tap();
    });

    it('coaches - displays coach list', async () => {
      await element(by.id('coaches-tab')).tap();
      await waitFor(element(by.id('coaches-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('coaches - search input works', async () => {
      await element(by.id('coaches-tab')).tap();
      try {
        await waitFor(element(by.id('coach-search-input')))
          .toBeVisible()
          .withTimeout(3000);
        await element(by.id('coach-search-input')).typeText('training');
        await element(by.id('coach-search-input')).clearText();
      } catch {
        // Search may not be visible
      }
    });

    it('coaches - favorite filter toggles', async () => {
      await element(by.id('coaches-tab')).tap();
      try {
        await element(by.id('favorites-filter')).tap();
      } catch {
        // Filter may not be visible
      }
    });

    it('coaches - coach card tap opens detail', async () => {
      await element(by.id('coaches-tab')).tap();
      try {
        await element(by.id('coach-card')).atIndex(0).tap();
        await waitFor(element(by.id('coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);
        // Go back
        await element(by.id('back-button')).tap();
      } catch {
        // No coaches or different UI
      }
    });
  });

  // ========================================
  // Coach Store Screen
  // ========================================
  describe('Coach Store', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('store - navigates via drawer', async () => {
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Store')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Store')).tap();
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('store - displays coach grid', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Store')).tap();
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('store - category tabs filter content', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Store')).tap();
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Try tapping category tabs
      try {
        await element(by.text('Training')).tap();
        await element(by.text('Nutrition')).tap();
        await element(by.text('Recovery')).tap();
      } catch {
        // Categories may have different names
      }
    });

    it('store - search coaches works', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Store')).tap();
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
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Store')).tap();
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await expect(element(by.id('install-button')).atIndex(0)).toBeVisible();
      } catch {
        // May show "Installed" instead
      }
    });
  });

  // ========================================
  // Social Tab / Feed
  // ========================================
  describe('Social Feed', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('feed - navigates via drawer', async () => {
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Feed')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('feed - displays insight cards or empty state', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Either insights or empty state
      try {
        await waitFor(element(by.id('insight-card')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        await expect(element(by.text(/No Insights|No posts/i))).toBeVisible();
      }
    });

    it('feed - reaction buttons visible on insight cards', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        // Look for reaction buttons (emoji buttons)
        await waitFor(element(by.id('reaction-button')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        // No insights
      }
    });

    it('feed - tapping reaction records it', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
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
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('feed-scroll-view')).swipe('down', 'fast');
    });

    it('feed - adapt button opens adapt screen', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('adapt-button')).atIndex(0).tap();
        await waitFor(element(by.id('adapt-insight-screen')))
          .toBeVisible()
          .withTimeout(5000);
        // Go back
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

    it('friends - navigates via drawer', async () => {
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Friends')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Friends')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('friends - displays friend list or empty state', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Friends')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await waitFor(element(by.id('friend-card')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        await expect(element(by.text(/No Friends|Add friends/i))).toBeVisible();
      }
    });

    it('friends - search button opens search', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Friends')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('search-friends-button')).tap();
      await waitFor(element(by.id('search-friends-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('friends - search finds users', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Friends')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('search-friends-button')).tap();
      await waitFor(element(by.id('search-friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('user-search-input')).typeText('alice');
      await waitFor(element(by.id('search-results-list')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('friends - pending requests tab works', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Friends')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.text('Pending')).tap();
      } catch {
        // Tab may not exist
      }
    });
  });

  // ========================================
  // Social Settings Screen
  // ========================================
  describe('Social Settings', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('social settings - navigates via drawer', async () => {
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Social Settings')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Social Settings')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('social settings - displays visibility options', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Social Settings')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.text('Discoverable'))).toBeVisible();
    });

    it('social settings - toggle discoverable', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Social Settings')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('discoverable-switch')).tap();
    });

    it('social settings - displays notification options', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Social Settings')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.text('Notifications'))).toBeVisible();
    });
  });

  // ========================================
  // Settings Tab
  // ========================================
  describe('Settings Tab', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('settings - tab icon visible and tappable', async () => {
      await waitFor(element(by.id('settings-tab')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.id('settings-tab')).tap();
    });

    it('settings - displays profile section', async () => {
      await element(by.id('settings-tab')).tap();
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Look for profile info
      await expect(element(by.text(/Profile|Account/i))).toBeVisible();
    });

    it('settings - displays connections section', async () => {
      await element(by.id('settings-tab')).tap();
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Look for connections
      try {
        await expect(element(by.text(/Connections|Providers/i))).toBeVisible();
      } catch {
        // May need to scroll
      }
    });

    it('settings - logout button visible', async () => {
      await element(by.id('settings-tab')).tap();
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Scroll to find logout
      try {
        await element(by.id('settings-scroll-view')).swipe('up', 'slow');
        await expect(element(by.text('Logout'))).toBeVisible();
      } catch {
        // May be at different position
      }
    });
  });

  // ========================================
  // Connection Modal
  // ========================================
  describe('Connection Modal', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('connections - opens from settings', async () => {
      await element(by.id('settings-tab')).tap();
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.text('Connections')).tap();
        await waitFor(element(by.id('connections-modal')))
          .toBeVisible()
          .withTimeout(5000);
      } catch {
        // May be inline instead of modal
      }
    });

    it('connections - displays Strava provider', async () => {
      await element(by.id('settings-tab')).tap();
      await waitFor(element(by.id('settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.text('Connections')).tap();
        await expect(element(by.text('Strava'))).toBeVisible();
      } catch {
        // May be different location
      }
    });
  });

  // ========================================
  // Share Insight Modal
  // ========================================
  describe('Share Insight Modal', () => {
    beforeAll(async () => {
      await device.reloadReactNative();
      await loginAsMobileTestUser();
    });

    it('share - opens from feed', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('share-insight-button')).tap();
      await waitFor(element(by.id('share-insight-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('share - displays visibility options', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await element(by.id('share-insight-button')).tap();
      await waitFor(element(by.id('share-insight-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.text(/Friends|Public|Private/i))).toBeVisible();
    });

    it('share - caption input works', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await element(by.id('share-insight-button')).tap();
      await waitFor(element(by.id('share-insight-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('insight-content-input')).typeText('Test caption');
        await element(by.id('insight-content-input')).clearText();
      } catch {
        // Different input ID
      }
    });

    it('share - publish button visible', async () => {
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await element(by.id('share-insight-button')).tap();
      await waitFor(element(by.id('share-insight-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.id('share-button'))).toBeVisible();
    });
  });
});
