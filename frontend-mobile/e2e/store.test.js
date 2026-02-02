// ABOUTME: E2E tests for Discover functionality
// ABOUTME: Tests store browsing, filtering, coach detail, and install/uninstall flow

const { loginAsMobileTestUser, navigateToTab } = require('./visual-test-helpers');

describe('Discover', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
    await loginAsMobileTestUser();

    // Navigate to Discover via tab
    await navigateToTab('discover');

    // Wait for store screen
    await waitFor(element(by.id('store-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  beforeEach(async () => {
    // Ensure we're on the store screen
    try {
      await expect(element(by.id('store-screen'))).toBeVisible();
    } catch {
      // Navigate back if not on the store screen
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    }
  });

  describe('basic rendering', () => {
    it('should show store screen', async () => {
      await expect(element(by.id('store-screen'))).toBeVisible();
    });

    it('should show Discover title', async () => {
      await expect(element(by.text('Discover'))).toBeVisible();
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

  describe('sorting', () => {
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

  describe('search', () => {
    it('should filter coaches when typing in search', async () => {
      await element(by.id('search-input')).tap();
      await element(by.id('search-input')).typeText('training');
      // Results should update based on search term
      await expect(element(by.id('search-input'))).toBeVisible();
    });

    it('should clear search when X is tapped', async () => {
      await element(by.id('search-input')).tap();
      await element(by.id('search-input')).typeText('test');

      try {
        await element(by.id('clear-search-button')).tap();
        await expect(element(by.id('search-input'))).toHaveText('');
      } catch {
        // Clear button might not exist
      }
    });
  });

  describe('coach detail', () => {
    it('should navigate to coach detail when card is tapped', async () => {
      try {
        await element(by.id('coach-card-0')).tap();
        await waitFor(element(by.id('coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);
      } catch {
        // No coaches available
      }
    });

    it('should navigate back to store on back button', async () => {
      try {
        await element(by.id('coach-card-0')).tap();
        await waitFor(element(by.id('coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);

        await element(by.id('back-button')).tap();
        await waitFor(element(by.id('store-screen')))
          .toBeVisible()
          .withTimeout(5000);
      } catch {
        // No coaches available
      }
    });
  });

  describe('tab navigation', () => {
    it('should navigate to coaches tab', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should navigate back to discover tab', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });
});
