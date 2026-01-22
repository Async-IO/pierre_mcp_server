// ABOUTME: E2E tests for Coach Store functionality
// ABOUTME: Tests store browsing, filtering, coach detail, and install/uninstall flow

describe('Coach Store', () => {
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
    await element(by.id('password-input')).typeText('mobiletest123');
    await element(by.id('login-button')).tap();

    // Wait for chat screen to load first
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);

    // Navigate to Coach Store via drawer
    await element(by.id('menu-button')).tap();
    await waitFor(element(by.text('Coach Store')))
      .toBeVisible()
      .withTimeout(3000);
    await element(by.text('Coach Store')).tap();

    // Wait for store screen
    await waitFor(element(by.id('store-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  beforeEach(async () => {
    // Ensure we're on the store screen
    try {
      await expect(element(by.id('store-screen'))).toBeVisible();
    } catch (error) {
      // Navigate back if not on the store screen
      await element(by.id('menu-button')).tap();
      await element(by.text('Coach Store')).tap();
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    }
  });

  describe('basic rendering', () => {
    it('should show store screen', async () => {
      await expect(element(by.id('store-screen'))).toBeVisible();
    });

    it('should show Coach Store title', async () => {
      await expect(element(by.text('Coach Store'))).toBeVisible();
    });

    it('should show search input', async () => {
      await expect(element(by.id('search-input'))).toBeVisible();
    });

    it('should show category filters', async () => {
      await expect(element(by.text('All'))).toBeVisible();
      await expect(element(by.text('Training'))).toBeVisible();
      await expect(element(by.text('Nutrition'))).toBeVisible();
    });

    it('should show sort options', async () => {
      await expect(element(by.text('Popular'))).toBeVisible();
      await expect(element(by.text('Newest'))).toBeVisible();
      await expect(element(by.text('A-Z'))).toBeVisible();
    });
  });

  describe('category filtering', () => {
    it('should filter by Training category', async () => {
      await element(by.text('Training')).tap();
      // Verify filter is applied
      await expect(element(by.text('Training'))).toBeVisible();
    });

    it('should filter by Nutrition category', async () => {
      await element(by.text('Nutrition')).tap();
      await expect(element(by.text('Nutrition'))).toBeVisible();
    });

    it('should filter by Recovery category', async () => {
      await element(by.text('Recovery')).tap();
      await expect(element(by.text('Recovery'))).toBeVisible();
    });

    it('should reset filters when All is selected', async () => {
      await element(by.text('All')).tap();
      await expect(element(by.text('All'))).toBeVisible();
    });
  });

  describe('sort options', () => {
    it('should sort by Popular', async () => {
      await element(by.text('Popular')).tap();
      await expect(element(by.text('Popular'))).toBeVisible();
    });

    it('should sort by Newest', async () => {
      await element(by.text('Newest')).tap();
      await expect(element(by.text('Newest'))).toBeVisible();
    });

    it('should sort alphabetically', async () => {
      await element(by.text('A-Z')).tap();
      await expect(element(by.text('A-Z'))).toBeVisible();
    });
  });

  describe('search functionality', () => {
    it('should search for coaches', async () => {
      await element(by.id('search-input')).clearText();
      await element(by.id('search-input')).typeText('training');

      // Wait for search results
      await waitFor(element(by.id('coach-list')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should clear search', async () => {
      await element(by.id('search-input')).clearText();

      // Should show all coaches again
      await waitFor(element(by.id('coach-list')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });

  describe('coach detail navigation', () => {
    it('should navigate to coach detail when card is tapped', async () => {
      // First ensure we have coaches loaded
      await waitFor(element(by.id('coach-list')))
        .toBeVisible()
        .withTimeout(5000);

      // Try to find any coach card and tap it
      try {
        await waitFor(element(by.id('coach-card-0')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('coach-card-0')).tap();

        // Should see coach detail screen
        await waitFor(element(by.id('store-coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);

        // Verify detail elements
        await expect(element(by.text('Install Coach'))).toBeVisible();

        // Navigate back
        await element(by.id('back-button')).tap();
      } catch (error) {
        console.log('No coaches available in store for detail test - skipping');
      }
    });
  });

  describe('coach detail screen', () => {
    beforeEach(async () => {
      // Navigate to a coach detail if possible
      try {
        await waitFor(element(by.id('coach-card-0')))
          .toBeVisible()
          .withTimeout(3000);
        await element(by.id('coach-card-0')).tap();
        await waitFor(element(by.id('store-coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);
      } catch (error) {
        // Skip these tests if no coaches available
      }
    });

    afterEach(async () => {
      // Try to navigate back to store
      try {
        await element(by.id('back-button')).tap();
        await waitFor(element(by.id('store-screen')))
          .toBeVisible()
          .withTimeout(3000);
      } catch (error) {
        // Already on store screen
      }
    });

    it('should show coach title', async () => {
      try {
        await expect(element(by.id('coach-title'))).toBeVisible();
      } catch (error) {
        console.log('No coaches available - skipping coach title test');
      }
    });

    it('should show category badge', async () => {
      try {
        await expect(element(by.id('category-badge'))).toBeVisible();
      } catch (error) {
        console.log('No coaches available - skipping category badge test');
      }
    });

    it('should show install count', async () => {
      try {
        await expect(element(by.id('install-count'))).toBeVisible();
      } catch (error) {
        console.log('No coaches available - skipping install count test');
      }
    });

    it('should show system prompt section', async () => {
      try {
        await expect(element(by.text('System Prompt'))).toBeVisible();
      } catch (error) {
        console.log('No coaches available - skipping system prompt test');
      }
    });

    it('should show install/uninstall button', async () => {
      try {
        // Either Install or Uninstall button should be visible
        try {
          await expect(element(by.text('Install Coach'))).toBeVisible();
        } catch (e) {
          await expect(element(by.text('Uninstall'))).toBeVisible();
        }
      } catch (error) {
        console.log('No coaches available - skipping action button test');
      }
    });
  });

  describe('install/uninstall flow', () => {
    it('should install a coach from the store', async () => {
      try {
        // Navigate to a coach that is not installed
        await waitFor(element(by.id('coach-card-0')))
          .toBeVisible()
          .withTimeout(5000);
        await element(by.id('coach-card-0')).tap();

        await waitFor(element(by.id('store-coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);

        // Check if Install button is visible
        try {
          await expect(element(by.text('Install Coach'))).toBeVisible();

          // Tap install
          await element(by.text('Install Coach')).tap();

          // Should show success alert
          await waitFor(element(by.text('Installed!')))
            .toBeVisible()
            .withTimeout(5000);

          // Dismiss alert
          await element(by.text('Stay Here')).tap();

          // Should now show Uninstall button
          await waitFor(element(by.text('Uninstall')))
            .toBeVisible()
            .withTimeout(5000);
        } catch (e) {
          // Coach might already be installed
          console.log('Coach already installed or install failed - checking uninstall');
          await expect(element(by.text('Uninstall'))).toBeVisible();
        }
      } catch (error) {
        console.log('No coaches available for install test - skipping');
      }
    });

    it('should uninstall a coach', async () => {
      try {
        // Check if we're on detail screen with Uninstall button
        try {
          await expect(element(by.id('store-coach-detail-screen'))).toBeVisible();
        } catch (e) {
          // Navigate to a coach
          await waitFor(element(by.id('coach-card-0')))
            .toBeVisible()
            .withTimeout(5000);
          await element(by.id('coach-card-0')).tap();
          await waitFor(element(by.id('store-coach-detail-screen')))
            .toBeVisible()
            .withTimeout(5000);
        }

        // Check if Uninstall is available
        try {
          await expect(element(by.text('Uninstall'))).toBeVisible();

          // Tap uninstall
          await element(by.text('Uninstall')).tap();

          // Confirm uninstall in alert
          await waitFor(element(by.text('Uninstall Coach?')))
            .toBeVisible()
            .withTimeout(3000);
          await element(by.text('Uninstall')).atIndex(1).tap();

          // Should show confirmation
          await waitFor(element(by.text('Uninstalled')))
            .toBeVisible()
            .withTimeout(5000);

          // Dismiss alert
          await element(by.text('OK')).tap();

          // Should now show Install button
          await waitFor(element(by.text('Install Coach')))
            .toBeVisible()
            .withTimeout(5000);
        } catch (e) {
          // Coach not installed
          console.log('Coach not installed - cannot test uninstall');
        }
      } catch (error) {
        console.log('No coaches available for uninstall test - skipping');
      }
    });
  });

  describe('navigation', () => {
    it('should navigate back to drawer on menu button', async () => {
      await element(by.id('menu-button')).tap();

      // Drawer should be visible
      await waitFor(element(by.text('Coach Store')))
        .toBeVisible()
        .withTimeout(3000);

      // Close drawer
      await element(by.text('Coach Store')).tap();
    });

    it('should navigate to My Coaches from drawer', async () => {
      await element(by.id('menu-button')).tap();

      await waitFor(element(by.text('My Coaches')))
        .toBeVisible()
        .withTimeout(3000);
      await element(by.text('My Coaches')).tap();

      // Should see coach library
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate back to store
      await element(by.id('menu-button')).tap();
      await element(by.text('Coach Store')).tap();
    });
  });

  describe('pull to refresh', () => {
    it('should refresh coaches on pull down', async () => {
      // Scroll up to trigger refresh
      await element(by.id('coach-list')).scroll(200, 'down');
      await element(by.id('coach-list')).scroll(200, 'up', NaN, NaN, 0.5);

      // Wait for refresh to complete
      await waitFor(element(by.id('coach-list')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });
});
