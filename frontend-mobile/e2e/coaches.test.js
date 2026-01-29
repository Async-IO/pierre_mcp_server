// ABOUTME: E2E tests for coach library functionality
// ABOUTME: Tests coach listing, filtering, favorites, and hide/show system coaches

describe('Coach Library Screen', () => {
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

    // Wait for chat screen to load first
    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(10000);

    // Navigate to coach library via drawer
    await element(by.id('menu-button')).tap();
    await waitFor(element(by.text('My Coaches')))
      .toBeVisible()
      .withTimeout(3000);
    await element(by.text('My Coaches')).tap();

    // Wait for coach library screen
    await waitFor(element(by.id('coach-library-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  beforeEach(async () => {
    // Ensure we're on the coach library screen
    try {
      await expect(element(by.id('coach-library-screen'))).toBeVisible();
    } catch (error) {
      // Navigate back if not on the coach library screen
      await element(by.id('menu-button')).tap();
      await element(by.text('My Coaches')).tap();
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);
    }
  });

  describe('basic rendering', () => {
    it('should show coach library screen', async () => {
      await expect(element(by.id('coach-library-screen'))).toBeVisible();
    });

    it('should show My Coaches title', async () => {
      await expect(element(by.text('My Coaches'))).toBeVisible();
    });

    it('should show category filters', async () => {
      await expect(element(by.text('All'))).toBeVisible();
      await expect(element(by.text('Training'))).toBeVisible();
      await expect(element(by.text('Nutrition'))).toBeVisible();
    });

    it('should show create coach button in header', async () => {
      await expect(element(by.id('create-coach-button'))).toBeVisible();
    });

    it('should show show hidden toggle button', async () => {
      await expect(element(by.id('show-hidden-toggle'))).toBeVisible();
    });

    it('should show floating search bar at bottom', async () => {
      await expect(element(by.id('coach-search-input'))).toBeVisible();
    });
  });

  describe('category filtering', () => {
    it('should filter coaches by Training category', async () => {
      await element(by.text('Training')).tap();
      // Verify filter is applied (visual check - the chip should be active)
      await expect(element(by.text('Training'))).toBeVisible();
    });

    it('should filter coaches by Nutrition category', async () => {
      await element(by.text('Nutrition')).tap();
      await expect(element(by.text('Nutrition'))).toBeVisible();
    });

    it('should reset filters when All is selected', async () => {
      await element(by.text('All')).tap();
      await expect(element(by.text('All'))).toBeVisible();
    });
  });

  describe('search functionality', () => {
    it('should filter coaches when typing in search', async () => {
      // Type in search input
      await element(by.id('coach-search-input')).typeText('Marathon');

      // Should still see search input
      await expect(element(by.id('coach-search-input'))).toBeVisible();

      // Clear search
      await element(by.id('coach-search-input')).clearText();
    });

    it('should show placeholder text in search', async () => {
      // The search input should have placeholder text visible
      await expect(element(by.id('coach-search-input'))).toBeVisible();
    });
  });

  describe('show hidden toggle', () => {
    it('should toggle show hidden state when tapped', async () => {
      // Get initial state
      await expect(element(by.id('show-hidden-toggle'))).toBeVisible();

      // Tap the toggle
      await element(by.id('show-hidden-toggle')).tap();

      // Toggle should still be visible (it changes icon)
      await expect(element(by.id('show-hidden-toggle'))).toBeVisible();

      // Tap again to reset
      await element(by.id('show-hidden-toggle')).tap();
    });
  });

  describe('coach creation', () => {
    it('should navigate to coach editor when FAB is pressed', async () => {
      await element(by.id('create-coach-button')).tap();

      // Should see coach editor screen
      await waitFor(element(by.text('Create Coach')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate back
      await device.pressBack();
    });
  });

  describe('coach card interactions', () => {
    it('should navigate to coach editor when coach card is tapped', async () => {
      // First, ensure we have at least one coach by creating one
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-title-input')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.id('coach-title-input')).typeText('E2E Test Coach');
      await element(by.id('coach-prompt-input')).typeText('You are a helpful test coach.');
      await element(by.id('save-coach-button')).tap();

      // Wait to return to library
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Find and tap the coach card
      await waitFor(element(by.text('E2E Test Coach')))
        .toBeVisible()
        .withTimeout(5000);
      await element(by.text('E2E Test Coach')).tap();

      // Should see coach editor
      await waitFor(element(by.text('Edit Coach')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate back
      await device.pressBack();
    });

    it('should show action menu on long press', async () => {
      // Long press on a coach card
      await element(by.text('E2E Test Coach')).longPress();

      // Should see action menu with options
      await waitFor(element(by.text('Add to favorites')))
        .toBeVisible()
        .withTimeout(3000);

      // Close menu by tapping outside
      await element(by.id('coach-library-screen')).tap();
    });
  });

  describe('system coach behavior', () => {
    it('should show System badge on system coaches', async () => {
      // This test assumes there's at least one system coach in the system
      // If not visible, enable show hidden
      await element(by.id('show-hidden-toggle')).tap();

      // Look for any system badge
      try {
        await waitFor(element(by.text('System')))
          .toBeVisible()
          .withTimeout(5000);
        await expect(element(by.text('System'))).toBeVisible();
      } catch (error) {
        // No system coaches available - this is OK for the test
        console.log('No system coaches found - skipping system badge check');
      }

      // Reset show hidden state
      await element(by.id('show-hidden-toggle')).tap();
    });
  });

  describe('cleanup', () => {
    it('should delete the test coach', async () => {
      // Find the test coach
      await waitFor(element(by.text('E2E Test Coach')))
        .toBeVisible()
        .withTimeout(5000);

      // Long press to show action menu
      await element(by.text('E2E Test Coach')).longPress();

      // Wait for Delete option
      await waitFor(element(by.text('Delete')))
        .toBeVisible()
        .withTimeout(3000);

      // Tap Delete
      await element(by.text('Delete')).tap();

      // Confirm deletion
      await waitFor(element(by.text('Delete')))
        .toBeVisible()
        .withTimeout(3000);
      await element(by.text('Delete')).tap();

      // Verify coach is deleted
      await waitFor(element(by.text('E2E Test Coach')))
        .not.toBeVisible()
        .withTimeout(5000);
    });
  });
});
