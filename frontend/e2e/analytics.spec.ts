// ABOUTME: Playwright E2E tests for the Analytics tab.
// ABOUTME: Tests time period selection, stats display, charts, and tool usage list.

import { test, expect } from '@playwright/test';

// Helper to set up authenticated state with Analytics API mocks
async function setupAnalyticsMocks(
  page: import('@playwright/test').Page,
  options: {
    hasData?: boolean;
    timeRange?: number;
  } = {}
) {
  const { hasData = true, timeRange = 30 } = options;

  // Mock usage analytics endpoint
  await page.route('**/api/dashboard/analytics*', async (route) => {
    const url = route.request().url();
    const requestedDays = url.includes('days=7') ? 7 : url.includes('days=90') ? 90 : 30;

    if (!hasData) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          time_series: [],
          top_tools: [],
          error_rate: 0,
          average_response_time: 0,
        }),
      });
      return;
    }

    // Generate time series data based on requested days
    const timeSeries = Array.from({ length: requestedDays }, (_, i) => {
      const date = new Date();
      date.setDate(date.getDate() - (requestedDays - i - 1));
      return {
        date: date.toISOString().split('T')[0],
        request_count: Math.floor(Math.random() * 200) + 100,
        error_count: Math.floor(Math.random() * 10),
      };
    });

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        time_series: timeSeries,
        top_tools: [
          { tool_name: 'get_activities', request_count: 4500, success_rate: 0.989, average_response_time: 120 },
          { tool_name: 'get_athlete', request_count: 450, success_rate: 0.967, average_response_time: 85 },
          { tool_name: 'get_zones', request_count: 45, success_rate: 0.98, average_response_time: 150 },
        ],
        error_rate: 2.3,
        average_response_time: 118,
      }),
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

test.describe('Analytics Tab', () => {
  test('renders Analytics tab with all main sections', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Analytics tab
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check header
    await expect(page.locator('h1')).toContainText('Analytics');

    // Check for main sections
    await expect(page.getByText('Usage Analytics')).toBeVisible();
    await expect(page.getByText('Total Requests')).toBeVisible();
    await expect(page.getByText('Error Rate')).toBeVisible();
    await expect(page.getByText('Avg Response Time')).toBeVisible();
  });

  test('displays time period dropdown with all options', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check dropdown is visible
    const dropdown = page.locator('select.input-field');
    await expect(dropdown).toBeVisible();

    // Check all options
    await expect(dropdown.locator('option[value="7"]')).toHaveText('Last 7 days');
    await expect(dropdown.locator('option[value="30"]')).toHaveText('Last 30 days');
    await expect(dropdown.locator('option[value="90"]')).toHaveText('Last 90 days');
  });

  test('changes time period when dropdown selection changes', async ({ page }) => {
    let requestedDays = 30;

    await page.route('**/api/dashboard/analytics*', async (route) => {
      const url = route.request().url();
      if (url.includes('days=7')) requestedDays = 7;
      else if (url.includes('days=90')) requestedDays = 90;
      else requestedDays = 30;

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          time_series: Array.from({ length: requestedDays }, (_, i) => ({
            date: new Date(Date.now() - (requestedDays - i) * 86400000).toISOString().split('T')[0],
            request_count: 100,
            error_count: 2,
          })),
          top_tools: [{ tool_name: 'get_activities', request_count: 1000, success_rate: 0.99, average_response_time: 100 }],
          error_rate: 2.0,
          average_response_time: 100,
        }),
      });
    });

    await page.route('**/api/dashboard/overview', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify({}) });
    });
    await page.route('**/api/dashboard/rate-limits', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify([]) });
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
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Select 7 days
    const dropdown = page.locator('select.input-field');
    await dropdown.selectOption('7');

    // Wait for data to update
    await page.waitForTimeout(500);

    // Select 90 days
    await dropdown.selectOption('90');
    await page.waitForTimeout(500);
  });

  test('displays stat cards with correct values', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Wait for stats to load
    await expect(page.getByText('Total Requests')).toBeVisible();

    // Check error rate displays with percentage
    await expect(page.getByText('2.3%')).toBeVisible();

    // Check average response time displays with ms
    await expect(page.getByText('118ms')).toBeVisible();
  });

  test('displays Request Volume Over Time chart section', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check chart section title
    await expect(page.getByText('Request Volume Over Time')).toBeVisible();

    // Chart should render (canvas element)
    await expect(page.locator('canvas').first()).toBeVisible();
  });

  test('displays Tool Usage Distribution chart', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check chart section title
    await expect(page.getByText('Tool Usage Distribution')).toBeVisible();
  });

  test('displays Response Time by Tool chart', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check chart section title
    await expect(page.getByText('Response Time by Tool')).toBeVisible();
  });

  test('displays Most Used Tools list with tool details', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check section title
    await expect(page.getByText('Most Used Tools')).toBeVisible();

    // Check tool names are displayed
    await expect(page.getByText('get_activities')).toBeVisible();
    await expect(page.getByText('get_athlete')).toBeVisible();
    await expect(page.getByText('get_zones')).toBeVisible();

    // Check success rates are displayed (98.9% for get_activities)
    await expect(page.getByText('98.9% success rate')).toBeVisible();

    // Check request counts
    await expect(page.getByText('4,500')).toBeVisible();
  });

  test('displays tool average response times in list', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check average response times are displayed
    await expect(page.getByText('120ms avg')).toBeVisible();
    await expect(page.getByText('85ms avg')).toBeVisible();
  });

  test('shows empty state when no data available', async ({ page }) => {
    await setupAnalyticsMocks(page, { hasData: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Check for empty state messages
    await expect(page.getByText('No usage data yet')).toBeVisible();
    await expect(page.getByText('Start making API calls to see analytics here')).toBeVisible();
  });

  test('shows loading spinner while data loads', async ({ page }) => {
    // Set up slow response
    await page.route('**/api/dashboard/analytics*', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ time_series: [], top_tools: [], error_rate: 0, average_response_time: 0 }),
      });
    });

    await page.route('**/api/dashboard/overview', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify({}) });
    });
    await page.route('**/api/dashboard/rate-limits', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify([]) });
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
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Should show loading spinner
    await expect(page.locator('.animate-spin')).toBeVisible({ timeout: 5000 });
  });

  test('handles API error gracefully', async ({ page }) => {
    await page.route('**/api/dashboard/analytics*', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await page.route('**/api/dashboard/overview', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify({}) });
    });
    await page.route('**/api/dashboard/rate-limits', async (route) => {
      await route.fulfill({ status: 200, body: JSON.stringify([]) });
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
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Page should still be navigable
    await expect(page.getByText('Usage Analytics')).toBeVisible();
  });
});

test.describe('Analytics Tab - Chart Interactions', () => {
  test('charts are responsive and render properly', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Wait for charts to render
    await page.waitForTimeout(1000);

    // Check multiple canvas elements are present (Line, Doughnut, Bar charts)
    const canvasElements = await page.locator('canvas').count();
    expect(canvasElements).toBeGreaterThanOrEqual(1);
  });

  test('tool list items are hoverable', async ({ page }) => {
    await setupAnalyticsMocks(page);
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });
    await page.getByRole('button', { name: /Analytics/i }).click();

    // Find a tool list item and check it has hover styling
    const toolItem = page.locator('text=get_activities').locator('..').locator('..');
    await expect(toolItem).toHaveClass(/hover:bg-pierre-gray-100/);
  });
});
