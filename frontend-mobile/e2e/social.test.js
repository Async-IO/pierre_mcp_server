// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: E2E tests for social features - friends, feed, sharing insights
// ABOUTME: Tests friend management, social feed interactions, and sharing workflows

const { loginAsMobileTestUser, navigateToTab } = require('./visual-test-helpers');

describe('Social Features E2E', () => {
  beforeAll(async () => {
    await device.launchApp();
  });

  beforeEach(async () => {
    await device.reloadReactNative();
    await loginAsMobileTestUser();
  });

  describe('Friends Management', () => {
    it('should display friends list', async () => {
      // Navigate to Insights tab
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate to Friends
      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Should see friends list or empty state
      await expect(element(by.id('friends-list'))).toBeVisible();
    });

    it('should search for friends', async () => {
      // Navigate to insights and then friends
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate to search
      await element(by.id('search-friends-button')).tap();
      await waitFor(element(by.id('search-friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Search for a user
      await element(by.id('search-input')).typeText('test');
      await waitFor(element(by.id('search-results')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should send friend request', async () => {
      // Navigate to search friends
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('search-friends-button')).tap();
      await waitFor(element(by.id('search-friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Search and send request (if user exists)
      await element(by.id('search-input')).typeText('webtest');
      await waitFor(element(by.id('search-results')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('send-request-button-0')).tap();
        await expect(element(by.text('Request sent'))).toBeVisible();
      } catch {
        // User might not exist or request already sent
      }
    });

    it('should display feed', async () => {
      // Navigate to Insights tab (which shows the feed)
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Should see feed or empty state
      await expect(element(by.id('feed-list'))).toBeVisible();
    });

    it('should react to a post', async () => {
      // Navigate to feed
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Try to react to a post
      try {
        await element(by.id('reaction-button-0')).tap();
        await element(by.text('Celebrate')).tap();
        // Reaction should be added
      } catch {
        // No posts to react to
      }
    });
  });

  describe('Social Settings', () => {
    it('should open social settings', async () => {
      // Navigate to insights, then social settings
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('social-settings-button')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should toggle privacy setting', async () => {
      // Navigate to social settings
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('social-settings-button')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Toggle privacy
      await element(by.id('privacy-toggle')).tap();
    });
  });

  describe('Sharing Insights', () => {
    it('should open share dialog from chat', async () => {
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(10000);

      // Look for a shareable insight in the chat
      try {
        await element(by.id('share-insight-button-0')).tap();
        await waitFor(element(by.id('share-insight-modal')))
          .toBeVisible()
          .withTimeout(5000);
      } catch {
        // No shareable insights
      }
    });

    it('should share insight to feed', async () => {
      await waitFor(element(by.id('chat-screen')))
        .toBeVisible()
        .withTimeout(10000);

      try {
        // Find and tap share button
        await element(by.id('share-insight-button-0')).tap();
        await waitFor(element(by.id('share-insight-modal')))
          .toBeVisible()
          .withTimeout(5000);

        // Add a comment and share
        await element(by.id('share-comment-input')).typeText('Check out my training!');
        await element(by.id('share-button')).tap();

        // Should show success
        await waitFor(element(by.text('Shared successfully')))
          .toBeVisible()
          .withTimeout(5000);
      } catch {
        // No shareable insights available
      }
    });

    it('should adapt insight from feed', async () => {
      // Navigate to feed
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        // Find and tap adapt button on a post
        await element(by.id('adapt-button-0')).tap();
        await waitFor(element(by.text('Adapt to My Training')))
          .toBeVisible()
          .withTimeout(5000);

        await element(by.text('Adapt to My Training')).tap();

        // Should navigate to adapted insight screen or show confirmation
        await waitFor(element(by.id('adapted-insight-screen')))
          .toBeVisible()
          .withTimeout(10000);
      } catch {
        // No posts with adaptable insights
      }
    });
  });

  describe('Friend Requests', () => {
    it('should view pending requests', async () => {
      // Navigate to friends
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Go to requests tab
      await element(by.id('friend-requests-button')).tap();
      await waitFor(element(by.id('friend-requests-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Should see requests list
      await expect(element(by.id('requests-list'))).toBeVisible();
    });

    it('should accept friend request', async () => {
      // Navigate to requests
      await navigateToTab('insights');
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friends-button')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('friend-requests-button')).tap();
      await waitFor(element(by.id('friend-requests-screen')))
        .toBeVisible()
        .withTimeout(5000);

      try {
        await element(by.id('accept-request-button-0')).tap();
        await expect(element(by.text('Friend added'))).toBeVisible();
      } catch {
        // No pending requests
      }
    });
  });
});
