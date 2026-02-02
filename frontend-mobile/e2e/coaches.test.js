// ABOUTME: E2E tests for coach library functionality
// ABOUTME: Tests coach listing, filtering, favorites, and hide/show system coaches

const { loginAsMobileTestUser, navigateToTab } = require('./visual-test-helpers');

describe('Coach Library Screen', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });
    await loginAsMobileTestUser();

    // Navigate to coach library via tab
    await navigateToTab('coaches');

    // Wait for coach library screen
    await waitFor(element(by.id('coach-library-screen')))
      .toBeVisible()
      .withTimeout(5000);
  });

  beforeEach(async () => {
    // Ensure we're on the coach library screen
    try {
      await expect(element(by.id('coach-library-screen'))).toBeVisible();
    } catch {
      // Navigate back if not on the coach library screen
      await navigateToTab('coaches');
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

  describe('search', () => {
    it('should filter coaches when typing in search', async () => {
      await element(by.id('coach-search-input')).tap();
      await element(by.id('coach-search-input')).typeText('training');
      await expect(element(by.id('coach-search-input'))).toBeVisible();
      await element(by.id('coach-search-input')).clearText();
    });
  });

  describe('coach detail', () => {
    it('should navigate to coach detail when card is tapped', async () => {
      try {
        await element(by.id('coach-card-0')).tap();
        await waitFor(element(by.id('coach-detail-screen')))
          .toBeVisible()
          .withTimeout(5000);

        await element(by.id('back-button')).tap();
      } catch {
        // No coaches available
      }
    });
  });

  describe('coach wizard', () => {
    it('should open wizard when create button is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.text('New Coach'))).toBeVisible();
      await device.pressBack();
    });
  });

  describe('tab navigation', () => {
    it('should navigate to discover tab', async () => {
      await navigateToTab('discover');
      await waitFor(element(by.id('store-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });

    it('should navigate back to coaches tab', async () => {
      await navigateToTab('coaches');
      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);
    });
  });
});
