// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for the Overview tab.
// ABOUTME: Tests stat cards, 7-day activity, rate limits, tier usage, quick actions, and alerts.

import { test, expect } from '@playwright/test';

// Helper to set up authenticated state with Overview API mocks
async function setupOverviewMocks(
  page: import('@playwright/test').Page,
  options: {
    isAdmin?: boolean;
    pendingUsersCount?: number;
    hasRateLimitWarning?: boolean;
    hasWeeklyData?: boolean;
    hasTierData?: boolean;
  } = {}
) {
  const {
    isAdmin = true,
    pendingUsersCount = 3,
    hasRateLimitWarning = false,
    hasWeeklyData = true,
    hasTierData = true
  } = options;

  // Mock dashboard overview endpoint
  await page.route('**/api/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_api_keys: 10,
        active_api_keys: 8,
        total_requests_today: 450,
        total_requests_this_month: 12500,
        current_month_usage_by_tier: hasTierData ? [
          { tier: 'trial', key_count: 2, total_requests: 500 },
          { tier: 'starter', key_count: 4, total_requests: 3000 },
          { tier: 'professional', key_count: 3, total_requests: 8000 },
          { tier: 'enterprise', key_count: 1, total_requests: 1000 },
        ] : [],
      }),
    });
  });

  // Mock rate limits endpoint
  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          api_key_id: 'key-1',
          api_key_name: 'Production API',
          tier: 'professional',
          current_usage: hasRateLimitWarning ? 950 : 450,
          limit: 1000,
          usage_percentage: hasRateLimitWarning ? 95 : 45
        },
        {
          api_key_id: 'key-2',
          api_key_name: 'Development',
          tier: 'starter',
          current_usage: 100,
          limit: 500,
          usage_percentage: 20
        },
        {
          api_key_id: 'key-3',
          api_key_name: 'Testing',
          tier: 'trial',
          current_usage: 50,
          limit: 100,
          usage_percentage: 50
        },
      ]),
    });
  });

  // Mock usage analytics endpoint for weekly data
  await page.route('**/api/dashboard/analytics*', async (route) => {
    if (!hasWeeklyData) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ time_series: [], top_tools: [] }),
      });
      return;
    }

    const timeSeries = Array.from({ length: 7 }, (_, i) => {
      const date = new Date();
      date.setDate(date.getDate() - (6 - i));
      return {
        date: date.toISOString().split('T')[0],
        request_count: 300 + Math.floor(Math.random() * 200),
        error_count: Math.floor(Math.random() * 10),
      };
    });

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        time_series: timeSeries,
        top_tools: [],
      }),
    });
  });

  // Mock A2A dashboard overview
  await page.route('**/a2a/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_clients: 5,
        active_clients: 3,
        requests_today: 100,
        requests_this_month: 3000,
      }),
    });
  });

  // Mock pending users endpoint
  await page.route('**/api/admin/pending-users', async (route) => {
    if (isAdmin) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          count: pendingUsersCount,
          users: Array.from({ length: pendingUsersCount }, (_, i) => ({
            id: `user-${i + 1}`,
            email: `pending${i + 1}@test.com`,
            status: 'pending',
          })),
        }),
      });
    } else {
      await route.fulfill({
        status: 403,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Admin access required' }),
      });
    }
  });

  // Set up authenticated state
  await page.addInitScript(
    ({ isAdmin }) => {
      localStorage.setItem(
        'user',
        JSON.stringify({
          id: 'user-123',
          email: 'admin@test.com',
          display_name: 'Test Admin',
          is_admin: isAdmin,
        })
      );
    },
    { isAdmin }
  );
}

test.describe('Overview Tab - Stat Cards', () => {
  test('displays Total Connections stat card', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check Total Connections (10 keys + 5 apps = 15)
    await expect(page.getByText('Total Connections')).toBeVisible();
    await expect(page.getByText('15')).toBeVisible();
    await expect(page.getByText('10 Keys + 5 Apps')).toBeVisible();
  });

  test('displays Active stat card', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check Active (8 keys + 3 apps = 11)
    await expect(page.getByText('Active')).toBeVisible();
    await expect(page.getByText('11')).toBeVisible();
    await expect(page.getByText('8 Keys + 3 Apps')).toBeVisible();
  });

  test('displays Today requests stat card', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check Today's requests (450 + 100 = 550)
    await expect(page.getByText('Today')).toBeVisible();
    await expect(page.getByText('550')).toBeVisible();
    await expect(page.getByText('requests').first()).toBeVisible();
  });

  test('displays This Month stat card', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check Monthly requests (12500 + 3000 = 15500)
    await expect(page.getByText('This Month')).toBeVisible();
    await expect(page.getByText('15,500')).toBeVisible();
  });

  test('stat cards have hover effect', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check hover styling
    const statCard = page.locator('.rounded-xl.border').first();
    await expect(statCard).toHaveClass(/hover:shadow-md/);
  });
});

test.describe('Overview Tab - 7-Day Activity', () => {
  test('displays 7-Day Activity card', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('7-Day Activity')).toBeVisible();
  });

  test('displays average per day', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show Avg X/day text
    await expect(page.getByText(/Avg \d+.*\/day/)).toBeVisible();
  });

  test('displays peak day indicator', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show Peak [day] text
    await expect(page.getByText(/Peak (Mon|Tue|Wed|Thu|Fri|Sat|Sun)/)).toBeVisible();
  });

  test('displays total requests badge', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show total with badge styling
    await expect(page.locator('.bg-pierre-violet\\/10.text-pierre-violet')).toBeVisible();
  });

  test('displays mini chart', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Chart should render
    await expect(page.locator('canvas').first()).toBeVisible();
  });

  test('hides 7-Day Activity when no data', async ({ page }) => {
    await setupOverviewMocks(page, { hasWeeklyData: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // 7-Day Activity should not be visible
    await expect(page.getByText('7-Day Activity')).not.toBeVisible();
  });
});

test.describe('Overview Tab - Rate Limits', () => {
  test('displays Rate Limits card', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Rate Limits')).toBeVisible();
  });

  test('displays capacity usage percentage', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show percentage of capacity
    await expect(page.getByText(/\d+% of capacity used/)).toBeVisible();
  });

  test('displays rate limit items with tier icons', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show tier icons (P for professional, S for starter, T for trial)
    await expect(page.locator('text=P').first()).toBeVisible();
    await expect(page.locator('text=S').first()).toBeVisible();
    await expect(page.locator('text=T').first()).toBeVisible();
  });

  test('displays API key names in rate limits', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Production API')).toBeVisible();
    await expect(page.getByText('Development')).toBeVisible();
  });

  test('displays progress bars for rate limits', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Progress bars should be visible
    const progressBars = page.locator('.h-1\\.5.bg-pierre-gray-100.rounded-full');
    await expect(progressBars.first()).toBeVisible();
  });

  test('shows warning color for high usage', async ({ page }) => {
    await setupOverviewMocks(page, { hasRateLimitWarning: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // High usage (>90%) should show red progress bar
    await expect(page.locator('.bg-red-500.h-full')).toBeVisible();
  });

  test('displays circular progress indicator', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Circular progress should be visible
    const circularProgress = page.locator('svg').filter({ has: page.locator('circle') });
    await expect(circularProgress.first()).toBeVisible();
  });
});

test.describe('Overview Tab - Usage by Tier', () => {
  test('displays Usage by Tier section', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Usage by Tier')).toBeVisible();
  });

  test('displays all tier cards', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // All four tiers should be displayed
    await expect(page.getByText('trial').first()).toBeVisible();
    await expect(page.getByText('starter').first()).toBeVisible();
    await expect(page.getByText('professional').first()).toBeVisible();
    await expect(page.getByText('enterprise').first()).toBeVisible();
  });

  test('displays key count for each tier', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show Keys label
    const keysLabels = page.getByText('Keys');
    await expect(keysLabels.first()).toBeVisible();
  });

  test('displays requests count for each tier', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show Requests label
    await expect(page.getByText('Requests').first()).toBeVisible();
  });

  test('displays average per key for each tier', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show Avg/Key label
    await expect(page.getByText('Avg/Key').first()).toBeVisible();
  });

  test('tier cards have distinct colors', async ({ page }) => {
    await setupOverviewMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Trial (gray), Starter (activity/green), Professional (violet), Enterprise (cyan)
    await expect(page.locator('.bg-pierre-gray-100')).toBeVisible();
    await expect(page.locator('.bg-pierre-activity\\/10')).toBeVisible();
    await expect(page.locator('.bg-pierre-violet\\/10')).toBeVisible();
    await expect(page.locator('.bg-pierre-cyan\\/10')).toBeVisible();
  });

  test('hides Usage by Tier when no tier data', async ({ page }) => {
    await setupOverviewMocks(page, { hasTierData: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Usage by Tier should not be visible
    await expect(page.getByText('Usage by Tier')).not.toBeVisible();
  });
});

test.describe('Overview Tab - Quick Actions (Admin Only)', () => {
  test('displays Quick Actions section for admin', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Quick Actions')).toBeVisible();
  });

  test('displays API Keys quick action button', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    const apiKeysButton = page.locator('button').filter({ hasText: 'API Keys' });
    await expect(apiKeysButton).toBeVisible();
  });

  test('displays Analytics quick action button', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    const analyticsButton = page.locator('button').filter({ hasText: 'Analytics' });
    await expect(analyticsButton).toBeVisible();
  });

  test('displays Monitor quick action button', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    const monitorButton = page.locator('button').filter({ hasText: 'Monitor' });
    await expect(monitorButton).toBeVisible();
  });

  test('displays Users quick action button', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    const usersButton = page.locator('button').filter({ hasText: 'Users' });
    await expect(usersButton).toBeVisible();
  });

  test('Quick Actions buttons have hover effect', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    const apiKeysButton = page.locator('button').filter({ hasText: 'API Keys' });
    await expect(apiKeysButton).toHaveClass(/hover:bg-pierre-violet\/10/);
  });

  test('hides Quick Actions for non-admin users', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Quick Actions')).not.toBeVisible();
  });
});

test.describe('Overview Tab - Alerts (Admin Only)', () => {
  test('displays Alerts section for admin', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Alerts')).toBeVisible();
  });

  test('displays pending users alert when users are pending', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, pendingUsersCount: 3 });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('3 users awaiting approval')).toBeVisible();
  });

  test('displays singular form for 1 pending user', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, pendingUsersCount: 1 });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('1 user awaiting approval')).toBeVisible();
  });

  test('pending users alert has pulsing indicator', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, pendingUsersCount: 3 });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Pulsing dot indicator
    await expect(page.locator('.animate-pulse.bg-pierre-nutrition')).toBeVisible();
  });

  test('displays rate limit warning alert', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, hasRateLimitWarning: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText(/\d+ key.* near limit/)).toBeVisible();
  });

  test('rate limit alert has red pulsing indicator', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, hasRateLimitWarning: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Red pulsing dot for rate limit warning
    await expect(page.locator('.animate-pulse.bg-red-500')).toBeVisible();
  });

  test('displays all systems normal when no alerts', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, pendingUsersCount: 0, hasRateLimitWarning: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('All systems normal')).toBeVisible();
  });

  test('all systems normal has green checkmark', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, pendingUsersCount: 0, hasRateLimitWarning: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Green background on all clear state
    await expect(page.locator('.bg-pierre-activity\\/10.border-pierre-activity\\/30')).toBeVisible();
  });

  test('hides Alerts section for non-admin users', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    await expect(page.getByText('Alerts')).not.toBeVisible();
  });

  test('pending users alert is clickable', async ({ page }) => {
    await setupOverviewMocks(page, { isAdmin: true, pendingUsersCount: 3 });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    const alertButton = page.locator('button').filter({ hasText: 'users awaiting approval' });
    await expect(alertButton).toBeVisible();

    // Has arrow icon indicating it's clickable
    await expect(alertButton.locator('svg')).toBeVisible();
  });
});

test.describe('Overview Tab - Loading State', () => {
  test('shows loading spinner while data loads', async ({ page }) => {
    await page.route('**/api/dashboard/overview', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({}),
      });
    });

    await page.route('**/api/dashboard/rate-limits', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify([]) });
    });
    await page.route('**/api/dashboard/analytics*', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify({}) });
    });
    await page.route('**/a2a/dashboard/overview', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify({}) });
    });

    await page.addInitScript(() => {
      localStorage.setItem(
        'user',
        JSON.stringify({ id: 'user-123', email: 'admin@test.com', display_name: 'Test Admin', is_admin: true })
      );
    });

    await page.goto('/dashboard');
    await page.waitForSelector('nav', { timeout: 10000 });

    // Should show loading spinner
    await expect(page.locator('.pierre-spinner')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Overview Tab - Responsive Layout', () => {
  test('stat cards stack on mobile', async ({ page }) => {
    await setupOverviewMocks(page);

    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/dashboard');
    await page.waitForSelector('nav', { timeout: 10000 });

    // Grid should become single column
    const statsGrid = page.locator('.grid.grid-cols-1.md\\:grid-cols-2');
    await expect(statsGrid.first()).toBeVisible();
  });

  test('tier cards stack on mobile', async ({ page }) => {
    await setupOverviewMocks(page);

    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/dashboard');
    await page.waitForSelector('nav', { timeout: 10000 });

    // Tier grid should become single column
    const tierGrid = page.locator('.grid.grid-cols-1.sm\\:grid-cols-2');
    await expect(tierGrid).toBeVisible();
  });
});
