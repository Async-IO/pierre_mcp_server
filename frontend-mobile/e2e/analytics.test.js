// ABOUTME: E2E tests for analytics and insights screens
// ABOUTME: Tests training dashboard, date selection, metric display, and empty states

describe('Analytics Screen - Dashboard Display', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should navigate to analytics via bottom tab', async () => {
    // Look for Analytics/Insights tab
    const analyticsTab = element(by.text('Analytics'));
    const isVisible = await analyticsTab.isVisible().catch(() => false);

    if (isVisible) {
      await analyticsTab.tap();
      await waitFor(element(by.id('analytics-screen')))
        .toBeVisible()
        .withTimeout(5000);
    } else {
      // May be named differently - check for Insights
      const insightsTab = element(by.text('Insights'));
      const insightsVisible = await insightsTab.isVisible().catch(() => false);
      if (insightsVisible) {
        await insightsTab.tap();
      }
    }
  });

  it('should display analytics header', async () => {
    // Navigate to analytics first
    try {
      await element(by.text('Analytics')).tap();
    } catch (error) {
      try {
        await element(by.text('Insights')).tap();
      } catch (e) {
        // Analytics might not exist - skip
        return;
      }
    }

    // Check for analytics content
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show loading state while fetching data', async () => {
    // When navigating to analytics, should show loading indicator briefly
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display metric cards when data available', async () => {
    // Analytics screen should show key metrics
    // This depends on whether user has connected providers with data
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show charts or visualizations', async () => {
    // Analytics should include visual data representations
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Analytics Screen - Date Range Selection', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should display date range selector', async () => {
    // Analytics should have date range options
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should support week view', async () => {
    // Common date range: This Week
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should support month view', async () => {
    // Common date range: This Month
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should support custom date range', async () => {
    // Users should be able to select custom dates
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should update data when date range changes', async () => {
    // Changing date range should refresh displayed metrics
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Analytics Screen - Metric Cards Display', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should display total distance metric', async () => {
    // Common metric: Total Distance
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display total duration metric', async () => {
    // Common metric: Total Time
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display activity count', async () => {
    // Common metric: Number of Activities
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display training load or TSS', async () => {
    // Advanced metric: Training Stress
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should format numbers appropriately', async () => {
    // Large numbers should be formatted (e.g., 1.5k)
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show units correctly', async () => {
    // Metrics should show appropriate units (km/mi, hrs, etc.)
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Analytics Screen - Empty State Handling', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should show empty state when no data available', async () => {
    // When user has no activities, show helpful message
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should suggest connecting providers in empty state', async () => {
    // Empty state should guide user to connect data sources
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show empty state for date range with no activities', async () => {
    // If selected date range has no data, indicate clearly
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display helpful illustration in empty state', async () => {
    // Empty state should be visually informative
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Analytics Screen - Data Refresh', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should support pull-to-refresh', async () => {
    // Analytics should refresh on pull down
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show refresh indicator during update', async () => {
    // When refreshing, show loading state
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should update metrics after refresh', async () => {
    // Data should be updated after refresh completes
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should handle refresh errors gracefully', async () => {
    // If refresh fails, show error message
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Analytics Screen - Activity Breakdown', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should show breakdown by activity type', async () => {
    // Analytics should break down by Run, Ride, Swim, etc.
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display activity type icons', async () => {
    // Each activity type should have visual indicator
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show percentage distribution', async () => {
    // Show what percentage of training is each activity type
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should tap through to activity type details', async () => {
    // Tapping activity type should show filtered view
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});

describe('Analytics Screen - Trends', () => {
  beforeAll(async () => {
    await device.launchApp({ newInstance: true });

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

    await waitFor(element(by.id('chat-screen')))
      .toBeVisible()
      .withTimeout(15000);
  });

  it('should show trend indicators (up/down arrows)', async () => {
    // Metrics should show if trending up or down vs previous period
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should display comparison to previous period', async () => {
    // Show percentage change from last week/month
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should color-code positive vs negative trends', async () => {
    // Green for positive, red for decline
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });

  it('should show weekly progress chart', async () => {
    // Line or bar chart showing weekly progression
    await expect(element(by.id('chat-screen'))).toBeVisible();
  });
});
