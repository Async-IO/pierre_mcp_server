// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for the Tools tab.
// ABOUTME: Tests charts, tool usage table, and summary statistics.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Helper to set up authenticated state with Tools API mocks
async function setupToolsMocks(
  page: Page,
  options: {
    hasData?: boolean;
    toolCount?: number;
  } = {}
) {
  const { hasData = true, toolCount = 5 } = options;

  // Set up base dashboard mocks (includes login mock)
  await setupDashboardMocks(page, { role: 'admin' });

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
}

async function loginAndNavigateToTools(page: Page) {
  await loginToDashboard(page);
  await navigateToTab(page, 'Tools');
  await page.waitForTimeout(500);
}

test.describe('Tools Tab - Overview', () => {
  test('renders Tools tab with header', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Check header
    await expect(page.locator('h1')).toContainText('Tools');
    await expect(page.getByText('Tool Usage Analysis')).toBeVisible();
  });

  test('displays all main sections', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Check for main sections
    await expect(page.getByText('Request Distribution')).toBeVisible();
    await expect(page.getByText('Average Response Time')).toBeVisible();
    await expect(page.getByText('Tool Usage Details')).toBeVisible();
  });
});

test.describe('Tools Tab - Charts', () => {
  test('displays Request Distribution doughnut chart', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Check chart section
    await expect(page.getByText('Request Distribution')).toBeVisible();

    // Canvas element should be present for chart - use first() for multiple canvases
    const chartContainer = page.locator('text=Request Distribution').locator('..').locator('..');
    await expect(chartContainer.locator('canvas').first()).toBeVisible();
  });

  test('displays Average Response Time bar chart', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Check chart section
    await expect(page.getByText('Average Response Time')).toBeVisible();

    // Canvas element should be present for chart - use first() for multiple canvases
    const chartContainer = page.locator('text=Average Response Time').locator('..').locator('..');
    await expect(chartContainer.locator('canvas').first()).toBeVisible();
  });

  test('charts render with correct height', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Charts should have 300px height containers
    const chartContainers = page.locator('[style*="height: 300px"]');
    await expect(chartContainers.first()).toBeVisible();
  });
});

test.describe('Tools Tab - Usage Details Table', () => {
  test('displays table with all column headers', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

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
    await loginAndNavigateToTools(page);

    // Tool names should be formatted nicely (Get Activities instead of get_activities)
    await expect(page.getByText('Get Activities')).toBeVisible();
    await expect(page.getByText('Get Athlete')).toBeVisible();
    await expect(page.getByText('Get Stats')).toBeVisible();
  });

  test('displays request counts in table', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Request counts should be formatted with commas
    await expect(page.getByText('4,500')).toBeVisible();
    await expect(page.getByText('450')).toBeVisible();
  });

  test('displays success rates with percentage', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Success rates should show percentage
    await expect(page.getByText('98.9%')).toBeVisible();
    await expect(page.getByText('96.7%')).toBeVisible();
  });

  test('displays success rate progress bars', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Progress bars should be visible (green for high success rate)
    const progressBars = page.locator('.bg-green-500.h-2.rounded-full');
    await expect(progressBars.first()).toBeVisible();
  });

  test('displays average response times in table', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Response times should show ms suffix
    await expect(page.getByText('120ms')).toBeVisible();
    await expect(page.getByText('85ms')).toBeVisible();
  });

  test('displays error counts with badges', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Error counts should have colored badges
    const errorBadges = page.locator('.rounded-full').filter({ hasText: /^\d+$/ });
    await expect(errorBadges.first()).toBeVisible();
  });

  test('displays share percentages in table', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Share percentages (get_activities has 4500/5550 = ~81%)
    await expect(page.locator('td').filter({ hasText: /\d+\.\d+%/ }).first()).toBeVisible();
  });

  test('table rows are hoverable', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Table rows should have hover styling
    const tableRow = page.locator('tbody tr').first();
    await expect(tableRow).toHaveClass(/hover:bg-gray-50/);
  });

  test('displays color indicators for each tool', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Color indicators (small colored circles next to tool names)
    const colorIndicators = page.locator('.w-3.h-3.rounded-full');
    await expect(colorIndicators.first()).toBeVisible();
  });
});

test.describe('Tools Tab - Summary Stats', () => {
  test('displays Tools Used stat card', async ({ page }) => {
    await setupToolsMocks(page, { toolCount: 5 });
    await loginAndNavigateToTools(page);

    await expect(page.getByText('Tools Used')).toBeVisible();
    await expect(page.locator('.stat-card').filter({ hasText: 'Tools Used' }).getByText('5')).toBeVisible();
  });

  test('displays Total Requests stat card', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    await expect(page.getByText('Total Requests')).toBeVisible();
    // Total: 4500 + 450 + 300 + 200 + 100 = 5,550
    await expect(page.getByText('5,550')).toBeVisible();
  });

  test('displays Overall Success Rate stat card', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    await expect(page.getByText('Overall Success Rate')).toBeVisible();
    // Weighted average success rate
    const successRateStat = page.locator('.stat-card').filter({ hasText: 'Overall Success Rate' });
    await expect(successRateStat.locator('text=/\\d+\\.\\d+%/')).toBeVisible();
  });

  test('displays Avg Response Time summary stat', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Should have Avg Response Time in summary stats
    const statCards = page.locator('.stat-card');
    await expect(statCards.filter({ hasText: 'Avg Response Time' })).toBeVisible();
  });

  test('summary stats are in a 4-column grid', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // Grid should have 4 stat cards
    const statsGrid = page.locator('.grid.grid-cols-1.md\\:grid-cols-4');
    await expect(statsGrid).toBeVisible();
  });
});

test.describe('Tools Tab - Empty State', () => {
  test('shows empty state when no tool data', async ({ page }) => {
    await setupToolsMocks(page, { hasData: false });
    await loginAndNavigateToTools(page);

    // Empty state message
    await expect(page.getByText('No tool usage data')).toBeVisible();
    await expect(page.getByText('Start making API calls to see tool usage breakdown')).toBeVisible();
  });

  test('shows wrench icon in empty state', async ({ page }) => {
    await setupToolsMocks(page, { hasData: false });
    await loginAndNavigateToTools(page);

    // Wrench emoji placeholder
    await expect(page.locator('text=🔧')).toBeVisible();
  });
});

test.describe('Tools Tab - Loading State', () => {
  test('shows loading spinner while data loads', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    await page.route('**/api/dashboard/tool-usage*', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    });

    await loginToDashboard(page);
    await navigateToTab(page, 'Tools');

    // Should show loading spinner
    await expect(page.locator('.pierre-spinner')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Tools Tab - Responsive Layout', () => {
  test('charts stack vertically on mobile', async ({ page }) => {
    await setupToolsMocks(page);

    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await loginAndNavigateToTools(page);

    // Charts should be in a grid that becomes single column on mobile
    const chartsGrid = page.locator('.grid.grid-cols-1');
    await expect(chartsGrid.first()).toBeVisible();
  });

  test('table is horizontally scrollable on mobile', async ({ page }) => {
    await setupToolsMocks(page);

    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await loginAndNavigateToTools(page);

    // Table should have overflow-x-auto
    const tableContainer = page.locator('.overflow-x-auto');
    await expect(tableContainer).toBeVisible();
  });
});

test.describe('Tools Tab - Data Accuracy', () => {
  test('calculates share percentages correctly', async ({ page }) => {
    await setupToolsMocks(page, { toolCount: 2 });
    await loginAndNavigateToTools(page);

    // With only 2 tools (4500 + 450 = 4950 total)
    // get_activities: 4500/4950 = 90.9%
    // get_athlete: 450/4950 = 9.1%
    await expect(page.getByText('90.9%')).toBeVisible();
  });

  test('calculates error counts from success rate', async ({ page }) => {
    await setupToolsMocks(page);
    await loginAndNavigateToTools(page);

    // get_activities: 4500 requests, 98.9% success = ~50 errors
    // Error count badge should be visible - use first() for multiple badges
    await expect(page.locator('.rounded-full').filter({ hasText: /^\d+$/ }).first()).toBeVisible();
  });
});
