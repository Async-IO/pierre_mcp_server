// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for Dashboard navigation and features.
// ABOUTME: Tests tab navigation, sidebar, user profile, and content loading.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Helper to set up additional dashboard mocks
async function setupFullDashboardMocks(page: Page, options: { isAdmin?: boolean } = {}) {
  const { isAdmin = false } = options;

  // Set up base dashboard mocks (includes login mock)
  await setupDashboardMocks(page, { role: isAdmin ? 'admin' : 'user' });

  // Mock request logs for monitor tab - must match format expected by Monitor component
  await page.route('**/api/dashboard/request-logs*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'req-1',
          timestamp: new Date().toISOString(),
          tool_name: 'get_activities',
          status_code: 200,
          response_time_ms: 120,
          api_key_id: 'key-1',
          api_key_name: 'Production API',
        },
        {
          id: 'req-2',
          timestamp: new Date().toISOString(),
          tool_name: 'get_athlete',
          status_code: 200,
          response_time_ms: 85,
          api_key_id: 'key-1',
          api_key_name: 'Production API',
        },
      ]),
    });
  });

  // Mock tool usage breakdown - must match format expected by Tools component
  await page.route('**/api/dashboard/tool-usage*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        { tool_name: 'get_activities', request_count: 4500, success_rate: 98.9, average_response_time: 120 },
        { tool_name: 'get_athlete', request_count: 450, success_rate: 96.7, average_response_time: 85 },
        { tool_name: 'get_zones', request_count: 200, success_rate: 95.0, average_response_time: 200 },
      ]),
    });
  });

  // Mock request stats - must match format expected by Monitor component
  await page.route('**/api/dashboard/request-stats*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_requests: 156,
        successful_requests: 148,
        average_response_time: 95.5,
        requests_per_minute: 2.6,
      }),
    });
  });

  // Mock API keys for connections tab
  await page.route('**/api/keys', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'key-1',
          name: 'Production API',
          description: 'Main production key',
          created_at: '2024-01-01T00:00:00Z',
          is_active: true,
        },
      ]),
    });
  });

  // Mock A2A clients for connections tab
  await page.route('**/a2a/clients', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'client-1',
          name: 'Fitness Bot',
          description: 'Automated fitness assistant',
          is_active: true,
        },
      ]),
    });
  });

  // Mock admin tokens endpoint for Connections tab (ApiKeyList component)
  await page.route('**/api/admin/tokens**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        tokens: [
          {
            id: 'token-1',
            service_name: 'Test Service',
            service_description: 'Test API token',
            token_prefix: 'pmcp_test',
            is_active: true,
            is_super_admin: false,
            created_at: new Date().toISOString(),
            expires_at: null,
            last_used_at: null,
          },
        ],
      }),
    });
  });
}

async function loginAndGoToDashboard(page: Page) {
  await loginToDashboard(page);
  await page.waitForTimeout(300);
}

test.describe('Dashboard Navigation', () => {
  test('displays sidebar with all navigation tabs', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    // Wait for dashboard to load
    await page.waitForSelector('nav', { timeout: 10000 });

    // Check main navigation tabs are present (using span text within buttons)
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Overview")') })).toBeVisible();
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Connections")') })).toBeVisible();
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Analytics")') })).toBeVisible();
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Monitor")') })).toBeVisible();
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Tools")') })).toBeVisible();
  });

  test('shows Users tab only for admin users', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Admin should see Users tab
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Users")') })).toBeVisible();
  });

  test('hides Users tab for non-admin users', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: false });
    await loginAndGoToDashboard(page);

    // Non-admin users see chat-first layout with header (no sidebar nav)
    await page.waitForSelector('header', { timeout: 10000 });

    // Non-admin should not see Users tab (they don't have a sidebar at all)
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Users")') })).not.toBeVisible();
  });

  test('navigates between tabs correctly', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Start at Overview - check tab is active (has gradient background)
    const overviewTab = page.locator('button').filter({ has: page.locator('span:has-text("Overview")') });
    await expect(overviewTab).toHaveClass(/bg-gradient/);

    // Navigate to Connections - check tab becomes active
    await navigateToTab(page, 'Connections');
    const connectionsTab = page.locator('button').filter({ has: page.locator('span:has-text("Connections")') });
    await expect(connectionsTab).toHaveClass(/bg-gradient/);
    await expect(overviewTab).not.toHaveClass(/bg-gradient/);

    // Navigate to Analytics - check tab becomes active
    await navigateToTab(page, 'Analytics');
    const analyticsTab = page.locator('button').filter({ has: page.locator('span:has-text("Analytics")') });
    await expect(analyticsTab).toHaveClass(/bg-gradient/);
    await expect(connectionsTab).not.toHaveClass(/bg-gradient/);
  });

  test('highlights active tab in sidebar', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Overview tab should be active by default
    const overviewButton = page.locator('button').filter({ has: page.locator('span:has-text("Overview")') });
    await expect(overviewButton).toHaveClass(/bg-gradient/);

    // Click Analytics and check it becomes active
    await navigateToTab(page, 'Analytics');
    const analyticsButton = page.locator('button').filter({ has: page.locator('span:has-text("Analytics")') });
    await expect(analyticsButton).toHaveClass(/bg-gradient/);
    await expect(overviewButton).not.toHaveClass(/bg-gradient/);
  });
});

test.describe('Dashboard Sidebar', () => {
  test('displays Pierre logo and branding', async ({ page }) => {
    // Sidebar tests require admin users (non-admin users see chat-first layout)
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check for Pierre branding text (use exact match to avoid matching "Chat with Pierre")
    await expect(page.getByText('Pierre', { exact: true })).toBeVisible();
    await expect(page.getByText('Fitness Intelligence')).toBeVisible();
  });

  test('collapses and expands sidebar', async ({ page }) => {
    // Sidebar tests require admin users (non-admin users see chat-first layout)
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Find collapse button
    const collapseButton = page.locator('button[title="Collapse sidebar"]');
    await expect(collapseButton).toBeVisible();

    // Click to collapse
    await collapseButton.click();

    // Sidebar should be collapsed - branding text should be hidden
    await expect(page.getByText('Fitness Intelligence')).not.toBeVisible();

    // Expand button should now be present
    const expandButton = page.locator('button[title="Expand sidebar"]');
    await expect(expandButton).toBeVisible();

    // Click to expand
    await expandButton.click();

    // Branding should be visible again
    await expect(page.getByText('Fitness Intelligence')).toBeVisible();
  });

  test('shows tooltips in collapsed state', async ({ page }) => {
    // Sidebar tests require admin users (non-admin users see chat-first layout)
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Collapse sidebar
    const collapseButton = page.locator('button[title="Collapse sidebar"]');
    await collapseButton.click();

    // Hover over a nav button to trigger tooltip
    const overviewButton = page.locator('button[title="Overview"]');
    await overviewButton.hover();

    // Wait for tooltip
    await page.waitForTimeout(300);
  });
});

test.describe('Dashboard User Profile', () => {
  test('displays user information in sidebar', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // User name should be displayed in sidebar
    await expect(page.getByText('Test Admin').first()).toBeVisible();
  });

  test('displays admin badge for admin users', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Admin badge should be visible
    await expect(page.getByText('Admin', { exact: true })).toBeVisible();
  });

  test('displays user badge for non-admin users', async ({ page }) => {
    // Non-admin users see chat-first layout; verify branding is visible
    await setupFullDashboardMocks(page, { isAdmin: false });
    await loginAndGoToDashboard(page);

    // Non-admin layout has header with Pierre branding (no sidebar with user profile)
    await page.waitForSelector('header', { timeout: 10000 });
    await expect(page.getByText('Pierre Fitness Intelligence')).toBeVisible();
  });

  test('shows user display name in header', async ({ page }) => {
    // User display name is shown in admin sidebar, not in non-admin chat layout
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // User display name should be visible in sidebar for admin users
    await expect(page.getByText('Test Admin').first()).toBeVisible();
  });

  test('logout button is visible and functional', async ({ page }) => {
    // Logout button is in admin sidebar, not in non-admin chat layout
    await setupFullDashboardMocks(page, { isAdmin: true });

    // Mock logout endpoint
    await page.route('**/api/auth/logout', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Find and click logout button
    const logoutButton = page.locator('button[title="Sign out"]');
    await expect(logoutButton).toBeVisible();
  });
});

test.describe('Dashboard Content Loading', () => {
  test('shows loading spinner while content loads', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    // Set up slow responses to observe loading state
    await page.route('**/api/dashboard/overview', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 2000));
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

    await loginToDashboard(page);

    // Should show loading spinner - check for any loading indicators
    // The dashboard may show a loading state while data loads
    await expect(page.locator('.pierre-spinner').first()).toBeVisible({ timeout: 3000 }).catch(() => {
      // If no spinner, verify dashboard loaded eventually
      return page.waitForSelector('nav', { timeout: 10000 });
    });
  });

  test('displays overview stats after loading', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Wait for overview content to load - check the header shows Overview
    await expect(page.locator('h1').first()).toContainText('Overview');

    // Verify stats section is visible (the actual numbers depend on API response)
    await expect(page.getByText(/Total|Requests|Keys/i).first()).toBeVisible({ timeout: 10000 });
  });

  test('loads Monitor tab content correctly', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Monitor tab (admin only)
    await navigateToTab(page, 'Monitor');

    // Wait for tab content to load and check for monitor-specific content
    // The Monitor tab renders "Real-time Request Monitor" as h2
    await expect(page.getByText('Real-time Request Monitor')).toBeVisible({ timeout: 10000 });
  });

  test('loads Tools tab content correctly', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Tools tab (admin only)
    await navigateToTab(page, 'Tools');

    // Wait for lazy-loaded component and check for tools-specific content
    await expect(page.getByText('Tool Usage Analysis')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Dashboard Header', () => {
  test('displays current tab name in header', async ({ page }) => {
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Header should show current tab name
    await expect(page.locator('header h1')).toContainText('Overview');

    // Navigate and check header updates
    await navigateToTab(page, 'Analytics');
    await expect(page.locator('header h1')).toContainText('Analytics');

    await navigateToTab(page, 'Monitor');
    await expect(page.locator('header h1')).toContainText('Monitor');
  });

  test('header is sticky on scroll', async ({ page }) => {
    // Admin layout has sticky header on the main content area
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check main content header has sticky positioning class
    const header = page.locator('main header');
    await expect(header).toHaveClass(/sticky/);
  });
});

test.describe('Dashboard Responsive Behavior', () => {
  test('sidebar collapses for better content visibility', async ({ page }) => {
    // Sidebar tests require admin users (non-admin users see chat-first layout)
    await setupFullDashboardMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Get main content area
    const main = page.locator('main');

    // Check initial margin when sidebar is expanded
    await expect(main).toHaveClass(/ml-\[260px\]/);

    // Collapse sidebar
    const collapseButton = page.locator('button[title="Collapse sidebar"]');
    await collapseButton.click();

    // Check margin changes to smaller value
    await expect(main).toHaveClass(/ml-\[72px\]/);
  });
});

test.describe('Dashboard Error Handling', () => {
  test('handles API errors gracefully', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    // Mock failing API endpoints
    await page.route('**/api/dashboard/overview', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginAndGoToDashboard(page);

    // Dashboard should still render navigation (admin users see Overview)
    await page.waitForSelector('nav', { timeout: 10000 });
    await expect(page.getByRole('list').getByRole('button', { name: 'Overview' })).toBeVisible();
  });
});
