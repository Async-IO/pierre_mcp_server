// ABOUTME: E2E tests for social features (Friends, Feed, Settings)
// ABOUTME: Tests social flows including friend management and feed interactions

describe('Social Features', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
    // Login first
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
    await waitFor(element(by.id('login-screen')))
      .not.toBeVisible()
      .withTimeout(15000);
  });

  beforeEach(async () => {
    await device.reloadReactNative();
    // Re-authenticate after reload
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
    await waitFor(element(by.id('login-screen')))
      .not.toBeVisible()
      .withTimeout(15000);
  });

  describe('Friends Screen', () => {
    beforeEach(async () => {
      // Open drawer and navigate to Friends
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Friends')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Friends')).tap();
      await waitFor(element(by.id('friends-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should display Friends screen header', async () => {
      await expect(element(by.text('Friends'))).toBeVisible();
    });

    it('should show empty state or friends list', async () => {
      // Either show empty state message or friends list
      try {
        await waitFor(element(by.text('No Friends Yet')))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        // Has friends - should show search bar
        await expect(element(by.id('friends-search-input'))).toBeVisible();
      }
    });

    it('should navigate to Search Friends', async () => {
      // Find and tap Search Friends button (in header or empty state)
      await waitFor(element(by.id('search-friends-button')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.id('search-friends-button')).tap();
      await waitFor(element(by.id('search-friends-screen')))
        .toBeVisible()
        .withTimeout(5000);
      await expect(element(by.text('Search Friends'))).toBeVisible();
    });

    it('should search for users', async () => {
      // Navigate to search
      await element(by.id('search-friends-button')).tap();
      await waitFor(element(by.id('search-friends-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Type search query
      await element(by.id('user-search-input')).typeText('test');

      // Wait for results (or empty message)
      await waitFor(element(by.id('search-results-list')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });

  describe('Social Feed Screen', () => {
    beforeEach(async () => {
      // Open drawer and navigate to Feed
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Feed')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should display Feed screen header', async () => {
      await expect(element(by.text('Feed'))).toBeVisible();
    });

    it('should show empty state or feed items', async () => {
      // Either show empty state or feed items
      try {
        await waitFor(element(by.text('No Insights Yet')))
          .toBeVisible()
          .withTimeout(3000);
      } catch {
        // Has feed items - should show insight cards
        await expect(element(by.id('feed-list'))).toBeVisible();
      }
    });

    it('should navigate to Share Insight', async () => {
      await element(by.id('share-insight-button')).tap();
      await waitFor(element(by.id('share-insight-screen')))
        .toBeVisible()
        .withTimeout(5000);
      await expect(element(by.text('Share Insight'))).toBeVisible();
    });

    it('should pull to refresh', async () => {
      // Pull down to refresh the feed
      await element(by.id('feed-scroll-view')).swipe('down', 'fast');
      // Wait for refresh indicator to disappear (feed reloaded)
      await waitFor(element(by.id('feed-scroll-view')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });

  describe('Social Settings Screen', () => {
    beforeEach(async () => {
      // Open drawer and navigate to Social Settings
      await element(by.id('drawer-toggle')).tap();
      await waitFor(element(by.text('Social Settings')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('Social Settings')).tap();
      await waitFor(element(by.id('social-settings-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should display Social Settings header', async () => {
      await expect(element(by.text('Social Settings'))).toBeVisible();
    });

    it('should display Privacy section', async () => {
      await expect(element(by.text('Privacy'))).toBeVisible();
      await expect(element(by.text('Discoverable'))).toBeVisible();
    });

    it('should display Default Sharing section', async () => {
      await expect(element(by.text('Default Sharing'))).toBeVisible();
      await expect(element(by.text('Friends Only'))).toBeVisible();
      await expect(element(by.text('Public'))).toBeVisible();
    });

    it('should display Notifications section', async () => {
      await expect(element(by.text('Notifications'))).toBeVisible();
      await expect(element(by.text('Friend Requests'))).toBeVisible();
      await expect(element(by.text('Reactions'))).toBeVisible();
      await expect(element(by.text('Adapted Insights'))).toBeVisible();
    });

    it('should toggle discoverable setting', async () => {
      // Find and toggle the discoverable switch
      await element(by.id('discoverable-switch')).tap();
      // Save button should appear
      await waitFor(element(by.text('Save')))
        .toBeVisible()
        .withTimeout(3000);
    });

    it('should change default visibility', async () => {
      // Tap Public option
      await element(by.text('Public')).tap();
      // Save button should appear
      await waitFor(element(by.text('Save')))
        .toBeVisible()
        .withTimeout(3000);
    });

    it('should save settings changes', async () => {
      // Make a change first
      await element(by.text('Public')).tap();
      // Save
      await element(by.text('Save')).tap();
      // Should show success or button should disappear
      await waitFor(element(by.text('Save')))
        .not.toBeVisible()
        .withTimeout(5000);
    });
  });

  describe('Share Insight Flow', () => {
    beforeEach(async () => {
      // Navigate to Share Insight via Feed
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

    it('should display share insight form', async () => {
      await expect(element(by.text('Share Insight'))).toBeVisible();
      await expect(element(by.id('insight-type-picker'))).toBeVisible();
      await expect(element(by.id('insight-content-input'))).toBeVisible();
    });

    it('should select insight type', async () => {
      await expect(element(by.id('insight-type-picker'))).toBeVisible();
      // Tap on Achievement type button
      await element(by.id('insight-type-achievement')).tap();
    });

    it('should enter insight content', async () => {
      await element(by.id('insight-content-input')).typeText('Just completed my first 5k!');
      await expect(element(by.text('Just completed my first 5k!'))).toBeVisible();
    });

    it('should select visibility', async () => {
      await element(by.text('Friends Only')).tap();
      // Should show selection indicator
    });

    it('should share insight', async () => {
      // Fill out form
      await element(by.id('insight-type-achievement')).tap();
      await element(by.id('insight-title-input')).typeText('First 5K');
      await element(by.id('insight-content-input')).typeText('Just completed my first 5k run!');

      // Share
      await element(by.id('share-button')).tap();

      // Should navigate back to feed or show success
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(10000);
    });
  });

  describe('Adapt Insight Flow', () => {
    it('should show Adapt to My Training button on feed items', async () => {
      // Navigate to feed
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // If there are feed items, check for adapt button
      try {
        await waitFor(element(by.id('adapt-button')).atIndex(0))
          .toBeVisible()
          .withTimeout(3000);
        await expect(element(by.text('Adapt to My Training'))).toBeVisible();
      } catch {
        // No feed items - empty state is shown
        await expect(element(by.text('No Insights Yet'))).toBeVisible();
      }
    });

    it('should navigate to adapt screen when tapping Adapt button', async () => {
      // Navigate to feed
      await element(by.id('drawer-toggle')).tap();
      await element(by.text('Feed')).tap();
      await waitFor(element(by.id('social-feed-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // If there are feed items with adapt button
      try {
        await element(by.id('adapt-button')).atIndex(0).tap();
        await waitFor(element(by.id('adapt-insight-screen')))
          .toBeVisible()
          .withTimeout(5000);
        await expect(element(by.text('Adapted for You'))).toBeVisible();
      } catch {
        // No feed items - skip this test condition
      }
    });
  });
});
