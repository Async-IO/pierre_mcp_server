// ABOUTME: E2E tests for Coach Wizard multi-step creation flow
// ABOUTME: Tests step navigation, validation, category picker, tags, and text expansion

const { loginAsMobileTestUser, navigateToTab } = require('./visual-test-helpers');

describe('Coach Wizard', () => {
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

  describe('wizard navigation', () => {
    it('should open wizard when create button is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.text('New Coach'))).toBeVisible();
      await device.pressBack();
    });

    it('should display step indicator at top', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('step-indicator')))
        .toBeVisible()
        .withTimeout(5000);

      // Check that step dots are visible
      await expect(element(by.id('step-dot-0'))).toBeVisible();
      await expect(element(by.id('step-dot-1'))).toBeVisible();
      await expect(element(by.id('step-dot-2'))).toBeVisible();
      await expect(element(by.id('step-dot-3'))).toBeVisible();

      // Check current step label
      await expect(element(by.id('current-step-label'))).toBeVisible();
      await expect(element(by.text('Basic Info'))).toBeVisible();

      await device.pressBack();
    });

    it('should navigate to next step when Next is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Fill required field first
      await element(by.id('coach-title-input')).typeText('Test Wizard Coach');

      // Tap next
      await element(by.id('next-button')).tap();

      // Should advance to step 2 (Personality)
      await waitFor(element(by.text('Personality')))
        .toBeVisible()
        .withTimeout(3000);

      await device.pressBack();
    });

    it('should navigate back when Back is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Fill required field and go to step 2
      await element(by.id('coach-title-input')).typeText('Test Coach');
      await element(by.id('next-button')).tap();

      await waitFor(element(by.text('Personality')))
        .toBeVisible()
        .withTimeout(3000);

      // Go back to step 1
      await element(by.id('back-button')).tap();

      await waitFor(element(by.text('Basic Info')))
        .toBeVisible()
        .withTimeout(3000);

      await device.pressBack();
    });
  });

  describe('step 1 - basic info', () => {
    it('should show title input', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.id('coach-title-input'))).toBeVisible();
      await device.pressBack();
    });

    it('should show category picker', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      await expect(element(by.id('category-picker'))).toBeVisible();
      await device.pressBack();
    });

    it('should validate empty title', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Try to advance without title
      await element(by.id('next-button')).tap();

      // Should show validation error
      await waitFor(element(by.text(/required|title/i)))
        .toBeVisible()
        .withTimeout(3000);

      await device.pressBack();
    });
  });

  describe('step 2 - personality', () => {
    it('should show personality textarea', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Go to step 2
      await element(by.id('coach-title-input')).typeText('Test Coach');
      await element(by.id('next-button')).tap();

      await waitFor(element(by.id('personality-input')))
        .toBeVisible()
        .withTimeout(5000);

      await device.pressBack();
    });
  });

  describe('step 3 - expertise', () => {
    it('should show expertise textarea', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate to step 3
      await element(by.id('coach-title-input')).typeText('Test Coach');
      await element(by.id('next-button')).tap();
      await element(by.id('next-button')).tap();

      await waitFor(element(by.id('expertise-input')))
        .toBeVisible()
        .withTimeout(5000);

      await device.pressBack();
    });
  });

  describe('step 4 - review', () => {
    it('should show review summary', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate to step 4
      await element(by.id('coach-title-input')).typeText('Test Coach');
      await element(by.id('next-button')).tap();
      await element(by.id('next-button')).tap();
      await element(by.id('next-button')).tap();

      await waitFor(element(by.text('Review')))
        .toBeVisible()
        .withTimeout(5000);

      await device.pressBack();
    });
  });
});
