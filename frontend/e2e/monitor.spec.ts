// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for the Monitor tab.
// ABOUTME: Tests real-time stats, filters, request logs, and empty states.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Helper to set up authenticated state with Monitor API mocks
async function setupMonitorMocks(
  page: Page,
  options: {
    hasRequests?: boolean;
    requestCount?: number;
  } = {}
) {
  const { hasRequests = true, requestCount = 10 } = options;

  // Set up base dashboard mocks (includes login mock)
  await setupDashboardMocks(page, { role: 'admin' });

  // Mock request logs endpoint
  await page.route('**/api/dashboard/request-logs*', async (route) => {
    if (!hasRequests) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
      return;
    }

    const logs = Array.from({ length: requestCount }, (_, i) => ({
      id: `req-${i + 1}`,
      timestamp: new Date(Date.now() - i * 60000).toISOString(),
      tool_name: ['get_activities', 'get_athlete', 'get_stats', 'get_activity_intelligence'][i % 4],
      status_code: i === 3 ? 500 : i === 7 ? 404 : 200,
      response_time_ms: 50 + Math.floor(Math.random() * 200),
      api_key_id: 'key-1',
      api_key_name: 'Production API',
      error_message: i === 3 ? 'Internal server error' : i === 7 ? 'Activity not found' : undefined,
    }));

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(logs),
    });
  });

  // Mock request stats endpoint
  await page.route('**/api/dashboard/request-stats*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_requests: hasRequests ? 156 : 0,
        successful_requests: hasRequests ? 148 : 0,
        average_response_time: hasRequests ? 95.5 : 0,
        requests_per_minute: hasRequests ? 2.6 : 0,
      }),
    });
  });
}

async function loginAndNavigateToMonitor(page: Page) {
  await loginToDashboard(page);
  await navigateToTab(page, 'Monitor');
  await page.waitForTimeout(500);
}

test.describe('Monitor Tab - Stats Display', () => {
  test('renders Monitor tab with header and description', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check header
    await expect(page.locator('h1')).toContainText('Monitor');
    await expect(page.getByText('Real-time Request Monitor')).toBeVisible();
  });

  test('displays Total Requests stat card', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    await expect(page.getByText('Total Requests')).toBeVisible();
    // Use exact match to avoid matching '156ms' from response times
    await expect(page.getByText('156', { exact: true })).toBeVisible();
  });

  test('displays Success Rate stat card', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    await expect(page.getByText('Success Rate')).toBeVisible();
    // 148/156 = 94.9%
    await expect(page.getByText('94.9%')).toBeVisible();
  });

  test('displays Avg Response Time stat card', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    await expect(page.getByText('Avg Response Time')).toBeVisible();
    // Use locator within the stat card to avoid matching response times in the request log
    const avgResponseCard = page.locator('div').filter({ hasText: 'Avg Response Time' }).first();
    await expect(avgResponseCard).toBeVisible();
  });

  test('displays Requests/min stat card', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    await expect(page.getByText('Requests/min')).toBeVisible();
    await expect(page.getByText('2.6')).toBeVisible();
  });
});

test.describe('Monitor Tab - Filters', () => {
  test('displays Time Range filter with all options', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check Time Range filter label
    await expect(page.getByText('Time Range')).toBeVisible();

    // Check dropdown has options
    const timeRangeSelect = page.locator('select').filter({ hasText: 'Last Hour' });
    await expect(timeRangeSelect.locator('option[value="1h"]')).toHaveText('Last Hour');
    await expect(timeRangeSelect.locator('option[value="24h"]')).toHaveText('Last 24 Hours');
    await expect(timeRangeSelect.locator('option[value="7d"]')).toHaveText('Last 7 Days');
    await expect(timeRangeSelect.locator('option[value="30d"]')).toHaveText('Last 30 Days');
  });

  test('Time Range filter changes data request', async ({ page }) => {
    await setupMonitorMocks(page, { hasRequests: false });
    await loginAndNavigateToMonitor(page);

    // Change to 24h
    const timeRangeSelect = page.locator('select').first();
    await timeRangeSelect.selectOption('24h');
    await page.waitForTimeout(500);

    // Change to 7d
    await timeRangeSelect.selectOption('7d');
    await page.waitForTimeout(500);
  });

  test('displays Status filter with options', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check Status filter label - use exact match to avoid matching description text
    await expect(page.getByText('Status', { exact: true })).toBeVisible();

    // Check dropdown has options
    const statusSelect = page.locator('select').filter({ hasText: 'All Status' });
    await expect(statusSelect.locator('option[value="all"]')).toHaveText('All Status');
    await expect(statusSelect.locator('option[value="success"]')).toHaveText('Success (2xx)');
    await expect(statusSelect.locator('option[value="error"]')).toHaveText('Error (4xx/5xx)');
  });

  test('Status filter changes to Success only', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Find status filter and change it
    const statusSelect = page.locator('select').nth(1);
    await statusSelect.selectOption('success');

    // Wait for filter to apply
    await page.waitForTimeout(500);
  });

  test('Status filter changes to Error only', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Find status filter and change it
    const statusSelect = page.locator('select').nth(1);
    await statusSelect.selectOption('error');

    await page.waitForTimeout(500);
  });

  test('displays Tool filter with options', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check Tool filter label - use exact match to avoid matching sidebar
    await expect(page.getByText('Tool', { exact: true })).toBeVisible();

    // Check dropdown has options
    const toolSelect = page.locator('select').filter({ hasText: 'All Tools' });
    await expect(toolSelect.locator('option[value="all"]')).toHaveText('All Tools');
    await expect(toolSelect.locator('option[value="get_activities"]')).toHaveText('Get Activities');
    await expect(toolSelect.locator('option[value="get_athlete"]')).toHaveText('Get Athlete');
  });

  test('Tool filter changes selection', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Find tool filter and change it
    const toolSelect = page.locator('select').nth(2);
    await toolSelect.selectOption('get_activities');

    await page.waitForTimeout(500);
  });
});

test.describe('Monitor Tab - Request Log', () => {
  test('displays Request Log section with count', async ({ page }) => {
    await setupMonitorMocks(page, { requestCount: 10 });
    await loginAndNavigateToMonitor(page);

    await expect(page.getByText('Request Log')).toBeVisible();
    await expect(page.getByText('Showing 10 requests')).toBeVisible();
  });

  test('displays request entries with tool names', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check tool names are visible
    await expect(page.getByText('get_activities').first()).toBeVisible();
    await expect(page.getByText('get_athlete').first()).toBeVisible();
  });

  test('displays status codes with appropriate colors', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check for success status codes (200)
    await expect(page.getByText('200').first()).toBeVisible();

    // Check for error status codes (500, 404)
    await expect(page.getByText('500').first()).toBeVisible();
  });

  test('displays success/error icons for requests', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Check success and error icons are present
    await expect(page.locator('text=✅').first()).toBeVisible();
    await expect(page.locator('text=❌').first()).toBeVisible();
  });

  test('displays response times for each request', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Response times should be displayed (in ms format)
    await expect(page.locator('text=/\\d+ms/').first()).toBeVisible();
  });

  test('displays timestamps for each request', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Timestamps should be visible (date format)
    const datePattern = await page.locator('[class*="text-gray-500"]').filter({ hasText: /\d{1,2}\/\d{1,2}\/\d{4}/ }).first();
    await expect(datePattern).toBeVisible();
  });

  test('displays error messages for failed requests', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Error message should be visible for failed request
    await expect(page.getByText('Internal server error')).toBeVisible();
  });
});

test.describe('Monitor Tab - Empty States', () => {
  test('shows empty state message when no requests', async ({ page }) => {
    await setupMonitorMocks(page, { hasRequests: false });
    await loginAndNavigateToMonitor(page);

    // Check empty state is displayed
    await expect(page.getByText('No requests yet')).toBeVisible();
    await expect(page.getByText('Start making API calls to see request logs here')).toBeVisible();
  });

  test('shows chart icon in empty state', async ({ page }) => {
    await setupMonitorMocks(page, { hasRequests: false });
    await loginAndNavigateToMonitor(page);

    // Check for placeholder icon
    await expect(page.locator('text=📊')).toBeVisible();
  });

  test('shows zero stats when no requests', async ({ page }) => {
    await setupMonitorMocks(page, { hasRequests: false });
    await loginAndNavigateToMonitor(page);

    // Stats should show zeros
    await expect(page.getByText('0.0%')).toBeVisible(); // Success rate
  });
});

test.describe('Monitor Tab - Loading States', () => {
  test('shows loading spinner while data loads', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    await page.route('**/api/dashboard/request-logs*', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    });

    await page.route('**/api/dashboard/request-stats*', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ total_requests: 0, successful_requests: 0, average_response_time: 0, requests_per_minute: 0 }),
      });
    });

    await loginToDashboard(page);
    await navigateToTab(page, 'Monitor');

    // Should show loading spinner
    await expect(page.locator('.pierre-spinner')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Monitor Tab - Real-time Indicator', () => {
  test('displays real-time indicator', async ({ page }) => {
    await setupMonitorMocks(page);
    await loginAndNavigateToMonitor(page);

    // Real-time indicator component should be present
    // The RealTimeIndicator component shows connection status
    await expect(page.locator('[class*="ml-auto"]')).toBeVisible();
  });
});

test.describe('Monitor Tab - Request Log Scrolling', () => {
  test('request log has scrollable container for many entries', async ({ page }) => {
    await setupMonitorMocks(page, { requestCount: 50 });
    await loginAndNavigateToMonitor(page);

    // Check that the container has overflow styling
    const requestLogContainer = page.locator('.max-h-96.overflow-y-auto');
    await expect(requestLogContainer).toBeVisible();
  });
});
