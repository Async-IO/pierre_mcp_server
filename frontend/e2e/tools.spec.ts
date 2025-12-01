// ABOUTME: Playwright E2E tests for the Tools tab.
// ABOUTME: Tests charts, tool usage table, and summary statistics.

import { test, expect } from '@playwright/test';

// Helper to set up authenticated state with Tools API mocks
async function setupToolsMocks(
  page: import('@playwright/test').Page,
  options: {
    hasData?: boolean;
    toolCount?: number;
  } = {}
) {
  const { hasData = true, toolCount = 5 } = options;

  // Mock tool usage breakdown endpoint
  await page.route('**/api/dashboard/tool-usage*', async (route) => {
    if (!hasData) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
      return;
    }

    const tools = [
      { tool_name: 'get_activities', request_count: 4500, success_rate: 98.9, average_response_time: 120 },
      { tool_name: 'get_athlete', request_count: 450, success_rate: 96.7, average_response_time: 85 },
      { tool_name: 'get_stats', request_count: 300, success_rate: 99.3, average_response_time: 150 },
      { tool_name: 'get_zones', request_count: 200, success_rate: 95.0, average_response_time: 200 },
      { tool_name: 'get_activity_intelligence', request_count: 100, success_rate: 92.0, average_response_time: 350 },
    ].slice(0, toolCount);

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(tools),
    });
  });

  // Mock other required dashboard endpoints
  await page.route('**/api/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_api_keys: 10,
        active_api_keys: 8,
        total_requests_today: 450,
        total_requests_this_month: 12500,
      }),
    });
  });

  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  await page.route('**/api/dashboard/analytics*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ time_series: [], top_tools: [] }),
    });
  });

  await page.route('**/a2a/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ total_clients: 5, active_clients: 3, requests_today: 100, requests_this_month: 3000 }),
    });
  });

  // Set up authenticated state
  await page.addInitScript(() => {
    localStorage.setItem(
      'user',
      JSON.stringify({
        id: 'user-123',
        email: 'admin@test.com',
        display_name: 'Test Admin',
        is_admin: true,
      })
    );
  });
}

test.describe('Tools Tab - Overview', () => {
  test('renders Tools tab with header', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Check header
    await expect(page.locator('h1')).toContainText('Tools');
    await expect(page.getByText('Tool Usage Analysis')).toBeVisible();
  });

  test('displays all main sections', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Check for main sections
    await expect(page.getByText('Request Distribution')).toBeVisible();
    await expect(page.getByText('Average Response Time')).toBeVisible();
    await expect(page.getByText('Tool Usage Details')).toBeVisible();
  });
});

test.describe('Tools Tab - Charts', () => {
  test('displays Request Distribution doughnut chart', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Check chart section
    await expect(page.getByText('Request Distribution')).toBeVisible();

    // Canvas element should be present for chart
    const chartContainer = page.locator('text=Request Distribution').locator('..').locator('..');
    await expect(chartContainer.locator('canvas')).toBeVisible();
  });

  test('displays Average Response Time bar chart', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Check chart section
    await expect(page.getByText('Average Response Time')).toBeVisible();

    // Canvas element should be present for chart
    const chartContainer = page.locator('text=Average Response Time').locator('..').locator('..');
    await expect(chartContainer.locator('canvas')).toBeVisible();
  });

  test('charts render with correct height', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Charts should have 300px height containers
    const chartContainers = page.locator('[style*="height: 300px"]');
    await expect(chartContainers.first()).toBeVisible();
  });
});

test.describe('Tools Tab - Usage Details Table', () => {
  test('displays table with all column headers', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Check table headers
    await expect(page.getByRole('columnheader', { name: 'Tool Name' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Requests' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Success Rate' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Avg Response Time' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Errors' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Share' })).toBeVisible();
  });

  test('displays tool names in table', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Tool names should be formatted nicely (Get Activities instead of get_activities)
    await expect(page.getByText('Get Activities')).toBeVisible();
    await expect(page.getByText('Get Athlete')).toBeVisible();
    await expect(page.getByText('Get Stats')).toBeVisible();
  });

  test('displays request counts in table', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Request counts should be formatted with commas
    await expect(page.getByText('4,500')).toBeVisible();
    await expect(page.getByText('450')).toBeVisible();
  });

  test('displays success rates with percentage', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Success rates should show percentage
    await expect(page.getByText('98.9%')).toBeVisible();
    await expect(page.getByText('96.7%')).toBeVisible();
  });

  test('displays success rate progress bars', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Progress bars should be visible (green for high success rate)
    const progressBars = page.locator('.bg-green-500.h-2.rounded-full');
    await expect(progressBars.first()).toBeVisible();
  });

  test('displays average response times in table', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Response times should show ms suffix
    await expect(page.getByText('120ms')).toBeVisible();
    await expect(page.getByText('85ms')).toBeVisible();
  });

  test('displays error counts with badges', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Error counts should have colored badges
    const errorBadges = page.locator('.rounded-full').filter({ hasText: /^\d+$/ });
    await expect(errorBadges.first()).toBeVisible();
  });

  test('displays share percentages in table', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Share percentages (get_activities has 4500/5550 = ~81%)
    await expect(page.locator('td').filter({ hasText: /\d+\.\d+%/ }).first()).toBeVisible();
  });

  test('table rows are hoverable', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Table rows should have hover styling
    const tableRow = page.locator('tbody tr').first();
    await expect(tableRow).toHaveClass(/hover:bg-gray-50/);
  });

  test('displays color indicators for each tool', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Color indicators (small colored circles next to tool names)
    const colorIndicators = page.locator('.w-3.h-3.rounded-full');
    await expect(colorIndicators.first()).toBeVisible();
  });
});

test.describe('Tools Tab - Summary Stats', () => {
  test('displays Tools Used stat card', async ({ page }) => {
    await setupToolsMocks(page, { toolCount: 5 });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    await expect(page.getByText('Tools Used')).toBeVisible();
    await expect(page.locator('.stat-card').filter({ hasText: 'Tools Used' }).getByText('5')).toBeVisible();
  });

  test('displays Total Requests stat card', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    await expect(page.getByText('Total Requests')).toBeVisible();
    // Total: 4500 + 450 + 300 + 200 + 100 = 5,550
    await expect(page.getByText('5,550')).toBeVisible();
  });

  test('displays Overall Success Rate stat card', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    await expect(page.getByText('Overall Success Rate')).toBeVisible();
    // Weighted average success rate
    const successRateStat = page.locator('.stat-card').filter({ hasText: 'Overall Success Rate' });
    await expect(successRateStat.locator('text=/\\d+\\.\\d+%/')).toBeVisible();
  });

  test('displays Avg Response Time summary stat', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Should have Avg Response Time in summary stats
    const statCards = page.locator('.stat-card');
    await expect(statCards.filter({ hasText: 'Avg Response Time' })).toBeVisible();
  });

  test('summary stats are in a 4-column grid', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Grid should have 4 stat cards
    const statsGrid = page.locator('.grid.grid-cols-1.md\\:grid-cols-4');
    await expect(statsGrid).toBeVisible();
  });
});

test.describe('Tools Tab - Empty State', () => {
  test('shows empty state when no tool data', async ({ page }) => {
    await setupToolsMocks(page, { hasData: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Empty state message
    await expect(page.getByText('No tool usage data')).toBeVisible();
    await expect(page.getByText('Start making API calls to see tool usage breakdown')).toBeVisible();
  });

  test('shows wrench icon in empty state', async ({ page }) => {
    await setupToolsMocks(page, { hasData: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Wrench emoji placeholder
    await expect(page.locator('text=🔧')).toBeVisible();
  });
});

test.describe('Tools Tab - Loading State', () => {
  test('shows loading spinner while data loads', async ({ page }) => {
    await page.route('**/api/dashboard/tool-usage*', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    });

    await page.route('**/api/dashboard/overview', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify({}) });
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
    await page.getByRole('button', { name: /Tools/i }).click();

    // Should show loading spinner
    await expect(page.locator('.animate-spin')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Tools Tab - Responsive Layout', () => {
  test('charts stack vertically on mobile', async ({ page }) => {
    await setupToolsMocks(page);

    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/dashboard');
    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Charts should be in a grid that becomes single column on mobile
    const chartsGrid = page.locator('.grid.grid-cols-1');
    await expect(chartsGrid.first()).toBeVisible();
  });

  test('table is horizontally scrollable on mobile', async ({ page }) => {
    await setupToolsMocks(page);

    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/dashboard');
    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // Table should have overflow-x-auto
    const tableContainer = page.locator('.overflow-x-auto');
    await expect(tableContainer).toBeVisible();
  });
});

test.describe('Tools Tab - Data Accuracy', () => {
  test('calculates share percentages correctly', async ({ page }) => {
    await setupToolsMocks(page, { toolCount: 2 });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // With only 2 tools (4500 + 450 = 4950 total)
    // get_activities: 4500/4950 = 90.9%
    // get_athlete: 450/4950 = 9.1%
    await expect(page.getByText('90.9%')).toBeVisible();
  });

  test('calculates error counts from success rate', async ({ page }) => {
    await setupToolsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Tools/i }).click();

    // get_activities: 4500 requests, 98.9% success = ~50 errors
    // Error count badge should be visible
    await expect(page.locator('.rounded-full').filter({ hasText: /^\d+$/ })).toBeVisible();
  });
});
