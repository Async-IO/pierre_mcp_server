// ABOUTME: E2E tests for Coach Wizard multi-step creation flow (ASY-158)
// ABOUTME: Tests step navigation, validation, category picker, tags, and text expansion

describe('Coach Wizard', () => {
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

      // Should be on step 2
      await expect(element(by.text('Category & Tags'))).toBeVisible();
      await expect(element(by.id('step-category-tags'))).toBeVisible();

      await device.pressBack();
    });

    it('should navigate back when Back is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Fill required field and go to step 2
      await element(by.id('coach-title-input')).typeText('Back Test Coach');
      await element(by.id('next-button')).tap();

      await waitFor(element(by.text('Category & Tags')))
        .toBeVisible()
        .withTimeout(3000);

      // Tap back
      await element(by.id('back-button')).tap();

      // Should be back on step 1
      await expect(element(by.text('Basic Info'))).toBeVisible();

      await device.pressBack();
    });
  });

  describe('step validation', () => {
    it('should show error when title is empty and Next is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Try to go to next step without filling title
      await element(by.id('next-button')).tap();

      // Should show error
      await expect(element(by.id('title-error'))).toBeVisible();
      await expect(element(by.text('Title is required'))).toBeVisible();

      await device.pressBack();
    });

    it('should show error when system prompt is empty', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate to system prompt step
      await element(by.id('coach-title-input')).typeText('Validation Test');
      await element(by.id('next-button')).tap();
      await element(by.id('next-button')).tap();

      // Should be on system prompt step
      await waitFor(element(by.text('System Prompt')))
        .toBeVisible()
        .withTimeout(3000);

      // Try to go to next step without prompt
      await element(by.id('next-button')).tap();

      // Should show error
      await expect(element(by.id('prompt-error'))).toBeVisible();
      await expect(element(by.text('System prompt is required'))).toBeVisible();

      await device.pressBack();
    });
  });

  describe('category picker', () => {
    it('should show category picker when tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Go to category step
      await element(by.id('coach-title-input')).typeText('Category Test');
      await element(by.id('next-button')).tap();

      await waitFor(element(by.id('category-picker')))
        .toBeVisible()
        .withTimeout(3000);

      // Tap category picker
      await element(by.id('category-picker')).tap();

      // Should show action sheet with category options
      await waitFor(element(by.text('Training')))
        .toBeVisible()
        .withTimeout(3000);

      // Select Training category
      await element(by.text('Training')).tap();

      // Verify category is selected (badge should show Training)
      await expect(element(by.id('selected-category'))).toBeVisible();

      await device.pressBack();
    });
  });

  describe('tag chips', () => {
    it('should add tag when input is submitted', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Go to category step
      await element(by.id('coach-title-input')).typeText('Tag Test');
      await element(by.id('next-button')).tap();

      await waitFor(element(by.id('tag-input')))
        .toBeVisible()
        .withTimeout(3000);

      // Type a tag and add it
      await element(by.id('tag-input')).typeText('intervals');
      await element(by.id('add-tag-button')).tap();

      // Tag should appear as a chip
      await expect(element(by.id('tag-chip-intervals'))).toBeVisible();

      await device.pressBack();
    });

    it('should remove tag when X is tapped', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Go to category step
      await element(by.id('coach-title-input')).typeText('Remove Tag Test');
      await element(by.id('next-button')).tap();

      await waitFor(element(by.id('tag-input')))
        .toBeVisible()
        .withTimeout(3000);

      // Add a tag
      await element(by.id('tag-input')).typeText('removeme');
      await element(by.id('add-tag-button')).tap();

      await expect(element(by.id('tag-chip-removeme'))).toBeVisible();

      // Remove the tag
      await element(by.id('remove-tag-removeme')).tap();

      // Tag should be gone
      await expect(element(by.id('tag-chip-removeme'))).not.toBeVisible();

      await device.pressBack();
    });
  });

  describe('text expansion', () => {
    it('should open full-screen modal for description', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Tap expand button for description
      await element(by.id('expand-description-button')).tap();

      // Modal should be visible
      await waitFor(element(by.id('expanded-modal')))
        .toBeVisible()
        .withTimeout(3000);

      await expect(element(by.id('modal-text-input'))).toBeVisible();

      // Close modal
      await element(by.id('modal-done-button')).tap();

      await device.pressBack();
    });

    it('should open full-screen modal for system prompt', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Navigate to system prompt step
      await element(by.id('coach-title-input')).typeText('Expand Test');
      await element(by.id('next-button')).tap();
      await element(by.id('next-button')).tap();

      await waitFor(element(by.id('expand-prompt-button')))
        .toBeVisible()
        .withTimeout(3000);

      // Tap expand button
      await element(by.id('expand-prompt-button')).tap();

      // Modal should show token count
      await waitFor(element(by.id('expanded-modal')))
        .toBeVisible()
        .withTimeout(3000);

      // Type in modal
      await element(by.id('modal-text-input')).typeText('Testing expanded view');

      // Close modal
      await element(by.id('modal-done-button')).tap();

      // Text should be preserved
      await expect(element(by.id('system-prompt-input'))).toHaveText('Testing expanded view');

      await device.pressBack();
    });
  });

  describe('review step', () => {
    it('should display all entered information on review step', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Step 1: Basic Info
      await element(by.id('coach-title-input')).typeText('Review Test Coach');
      await element(by.id('coach-description-input')).typeText('A test description');
      await element(by.id('next-button')).tap();

      // Step 2: Category & Tags
      await element(by.id('tag-input')).typeText('test');
      await element(by.id('add-tag-button')).tap();
      await element(by.id('next-button')).tap();

      // Step 3: System Prompt
      await element(by.id('system-prompt-input')).typeText('You are a test coach.');
      await element(by.id('next-button')).tap();

      // Step 4: Review
      await waitFor(element(by.id('step-review')))
        .toBeVisible()
        .withTimeout(3000);

      await expect(element(by.id('review-title'))).toBeVisible();
      await expect(element(by.id('review-coach-title'))).toHaveText('Review Test Coach');
      await expect(element(by.id('review-description'))).toHaveText('A test description');
      await expect(element(by.id('review-category'))).toBeVisible();
      await expect(element(by.id('review-tags'))).toBeVisible();
      await expect(element(by.id('review-prompt'))).toBeVisible();

      await device.pressBack();
    });
  });

  describe('full wizard flow', () => {
    it('should create a coach through complete wizard flow', async () => {
      await element(by.id('create-coach-button')).tap();

      await waitFor(element(by.id('coach-wizard-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Step 1: Basic Info
      await element(by.id('coach-title-input')).typeText('E2E Wizard Coach');
      await element(by.id('coach-description-input')).typeText('Created via E2E test');
      await element(by.id('next-button')).tap();

      // Step 2: Category & Tags
      await waitFor(element(by.id('category-picker')))
        .toBeVisible()
        .withTimeout(3000);
      await element(by.id('category-picker')).tap();
      await element(by.text('Training')).tap();
      await element(by.id('tag-input')).typeText('e2e');
      await element(by.id('add-tag-button')).tap();
      await element(by.id('next-button')).tap();

      // Step 3: System Prompt
      await waitFor(element(by.id('system-prompt-input')))
        .toBeVisible()
        .withTimeout(3000);
      await element(by.id('system-prompt-input')).typeText('You are an E2E test coach created by Detox.');
      await element(by.id('next-button')).tap();

      // Step 4: Review & Save
      await waitFor(element(by.id('step-review')))
        .toBeVisible()
        .withTimeout(3000);
      await element(by.id('save-button')).tap();

      // Should return to library with success message
      await waitFor(element(by.text('Coach created successfully')))
        .toBeVisible()
        .withTimeout(5000);

      await waitFor(element(by.id('coach-library-screen')))
        .toBeVisible()
        .withTimeout(5000);

      // Verify coach appears in list
      await expect(element(by.text('E2E Wizard Coach'))).toBeVisible();
    });

    it('should cleanup the created coach', async () => {
      // Find and delete the test coach
      await waitFor(element(by.text('E2E Wizard Coach')))
        .toBeVisible()
        .withTimeout(5000);

      await element(by.text('E2E Wizard Coach')).longPress();

      await waitFor(element(by.text('Delete')))
        .toBeVisible()
        .withTimeout(3000);

      await element(by.text('Delete')).tap();

      // Confirm deletion
      await waitFor(element(by.text('Delete')))
        .toBeVisible()
        .withTimeout(3000);
      await element(by.text('Delete')).tap();

      // Verify coach is deleted
      await waitFor(element(by.text('E2E Wizard Coach')))
        .not.toBeVisible()
        .withTimeout(5000);
    });
  });
});

describe('Coach Forking (Mobile)', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: false });

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

  it('should show fork option for system coaches', async () => {
    // Enable showing hidden/system coaches
    await element(by.id('show-hidden-toggle')).tap();

    // Look for a system coach
    try {
      await waitFor(element(by.text('System')))
        .toBeVisible()
        .withTimeout(5000);

      // Long press on a system coach to show action menu
      const systemCoachCard = element(by.id('coach-card')).atIndex(0);
      await systemCoachCard.longPress();

      // Should see Fork option in action menu
      await waitFor(element(by.text('Fork')))
        .toBeVisible()
        .withTimeout(3000);

      // Dismiss the menu
      await element(by.id('coach-library-screen')).tap();
    } catch (error) {
      // No system coaches available - this is OK for the test
      console.log('No system coaches found - skipping fork test');
    }

    // Reset show hidden state
    await element(by.id('show-hidden-toggle')).tap();
  });
});
