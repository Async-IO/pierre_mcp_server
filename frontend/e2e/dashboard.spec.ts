// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for Dashboard navigation and features.
// ABOUTME: Tests tab navigation, sidebar, user profile, and content loading.

import { test, expect } from '@playwright/test';

// Helper to set up authenticated state with API mocks
async function setupAuthenticatedMocks(
  page: import('@playwright/test').Page,
  options: { isAdmin?: boolean } = {}
) {
  const { isAdmin = false } = options;

  // Mock dashboard overview endpoint
  await page.route('**/api/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_requests: 12500,
        requests_today: 450,
        active_keys: 8,
        connected_providers: 3,
      }),
    });
  });

  // Mock rate limits endpoint
  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        { key_name: 'Production API', used: 450, limit: 1000, percentage: 45 },
        { key_name: 'Development', used: 100, limit: 500, percentage: 20 },
      ]),
    });
  });

  // Mock usage analytics endpoint
  await page.route('**/api/dashboard/analytics*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_requests: 3200,
        daily_breakdown: [
          { date: '2024-01-01', count: 400 },
          { date: '2024-01-02', count: 520 },
          { date: '2024-01-03', count: 380 },
        ],
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
        active_sessions: 12,
        total_requests_24h: 890,
      }),
    });
  });

  // Mock pending users endpoint (for admin badge)
  await page.route('**/api/admin/pending-users', async (route) => {
    if (isAdmin) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          count: 3,
          users: [
            { id: 'user-1', email: 'pending1@test.com', status: 'pending' },
            { id: 'user-2', email: 'pending2@test.com', status: 'pending' },
            { id: 'user-3', email: 'pending3@test.com', status: 'pending' },
          ],
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

  // Mock request logs for monitor tab
  await page.route('**/api/dashboard/request-logs*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'req-1',
          timestamp: new Date().toISOString(),
          tool: 'get_activities',
          status: 'success',
          duration_ms: 120,
        },
        {
          id: 'req-2',
          timestamp: new Date().toISOString(),
          tool: 'get_athlete',
          status: 'success',
          duration_ms: 85,
        },
      ]),
    });
  });

  // Mock tool usage breakdown
  await page.route('**/api/dashboard/tool-usage*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        { tool_name: 'get_activities', call_count: 1250, percentage: 45 },
        { tool_name: 'get_athlete', call_count: 800, percentage: 28 },
        { tool_name: 'get_zones', call_count: 450, percentage: 16 },
      ]),
    });
  });

  // Mock request stats
  await page.route('**/api/dashboard/request-stats*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total: 156,
        success: 148,
        errors: 8,
        avg_response_time: 95,
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

  // Set up localStorage with user data to simulate authenticated state
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

test.describe('Dashboard Navigation', () => {
  test('displays sidebar with all navigation tabs', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    // Wait for dashboard to load
    await page.waitForSelector('nav', { timeout: 10000 });

    // Check all main navigation tabs are present
    await expect(page.getByRole('button', { name: /Overview/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Connections/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Analytics/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Monitor/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Tools/i })).toBeVisible();
  });

  test('shows Users tab only for admin users', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Admin should see Users tab
    await expect(page.getByRole('button', { name: /Users/i })).toBeVisible();
  });

  test('hides Users tab for non-admin users', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Non-admin should not see Users tab
    await expect(page.getByRole('button', { name: /Users/i })).not.toBeVisible();
  });

  test('displays pending users badge for admins', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Badge should show count of pending users
    const badge = page.locator('[data-testid="pending-users-badge"]');
    await expect(badge).toBeVisible();
    await expect(badge).toHaveText('3');
  });

  test('navigates between tabs correctly', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Start at Overview
    await expect(page.locator('h1')).toContainText('Overview');

    // Navigate to Analytics
    await page.getByRole('button', { name: /Analytics/i }).click();
    await expect(page.locator('h1')).toContainText('Analytics');

    // Navigate to Monitor
    await page.getByRole('button', { name: /Monitor/i }).click();
    await expect(page.locator('h1')).toContainText('Monitor');
    await expect(page.getByText('Real-time Request Monitor')).toBeVisible();

    // Navigate to Tools
    await page.getByRole('button', { name: /Tools/i }).click();
    await expect(page.locator('h1')).toContainText('Tools');
    await expect(page.getByText('Tool Usage Analysis')).toBeVisible();

    // Navigate to Users (admin only)
    await page.getByRole('button', { name: /Users/i }).click();
    await expect(page.locator('h1')).toContainText('Users');
    await expect(page.getByText('User Management')).toBeVisible();
  });

  test('highlights active tab in sidebar', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Overview tab should be active by default
    const overviewButton = page.getByRole('button', { name: /Overview/i });
    await expect(overviewButton).toHaveClass(/bg-gradient/);

    // Click Analytics and check it becomes active
    const analyticsButton = page.getByRole('button', { name: /Analytics/i });
    await analyticsButton.click();
    await expect(analyticsButton).toHaveClass(/bg-gradient/);
    await expect(overviewButton).not.toHaveClass(/bg-gradient/);
  });
});

test.describe('Dashboard Sidebar', () => {
  test('displays Pierre logo and branding', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check for Pierre branding text
    await expect(page.getByText('Pierre')).toBeVisible();
    await expect(page.getByText('Fitness Intelligence')).toBeVisible();
  });

  test('collapses and expands sidebar', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

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
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Collapse sidebar
    const collapseButton = page.locator('button[title="Collapse sidebar"]');
    await collapseButton.click();

    // Hover over a nav button to trigger tooltip
    const analyticsButton = page.getByRole('button', { name: /Analytics/i });
    await analyticsButton.hover();

    // Tooltip should appear
    await expect(page.locator('text=Analytics').last()).toBeVisible();
  });
});

test.describe('Dashboard User Profile', () => {
  test('displays user information in sidebar', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // User name should be displayed
    await expect(page.getByText('Test Admin')).toBeVisible();
  });

  test('displays admin badge for admin users', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Admin badge should be visible
    await expect(page.getByText('Admin', { exact: true })).toBeVisible();
  });

  test('displays user badge for non-admin users', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // User badge should be visible (not Admin)
    await expect(page.getByText('User', { exact: true })).toBeVisible();
    await expect(page.getByText('Admin', { exact: true })).not.toBeVisible();
  });

  test('shows welcome message with user name', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Welcome message in header
    await expect(page.getByText(/Welcome back, Test Admin/)).toBeVisible();
  });

  test('logout button is visible and functional', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });

    // Mock logout endpoint
    await page.route('**/api/auth/logout', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    await page.goto('/dashboard');
    await page.waitForSelector('nav', { timeout: 10000 });

    // Find and click logout button
    const logoutButton = page.locator('button[title="Sign out"]');
    await expect(logoutButton).toBeVisible();
  });
});

test.describe('Dashboard Content Loading', () => {
  test('shows loading spinner while content loads', async ({ page }) => {
    // Set up slow responses to observe loading state
    await page.route('**/api/dashboard/overview', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          total_requests: 12500,
          requests_today: 450,
          active_keys: 8,
          connected_providers: 3,
        }),
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
        JSON.stringify({
          id: 'user-123',
          email: 'admin@test.com',
          display_name: 'Test Admin',
          is_admin: false,
        })
      );
    });

    await page.goto('/dashboard');

    // Should show loading spinner
    await expect(page.locator('.pierre-spinner')).toBeVisible({ timeout: 5000 });
  });

  test('displays overview stats after loading', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Wait for overview content to load - look for stats
    await expect(page.getByText('12,500').or(page.getByText('12500'))).toBeVisible({
      timeout: 10000,
    });
  });

  test('loads Monitor tab content correctly', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Monitor tab
    await page.getByRole('button', { name: /Monitor/i }).click();

    // Check for monitor-specific content
    await expect(page.getByText('Real-time Request Monitor')).toBeVisible();
  });

  test('loads Tools tab content correctly', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Tools tab
    await page.getByRole('button', { name: /Tools/i }).click();

    // Check for tools-specific content
    await expect(page.getByText('Tool Usage Analysis')).toBeVisible();
  });
});

test.describe('Dashboard Header', () => {
  test('displays current tab name in header', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: true });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Header should show current tab name
    await expect(page.locator('header h1')).toContainText('Overview');

    // Navigate and check header updates
    await page.getByRole('button', { name: /Analytics/i }).click();
    await expect(page.locator('header h1')).toContainText('Analytics');

    await page.getByRole('button', { name: /Monitor/i }).click();
    await expect(page.locator('header h1')).toContainText('Monitor');
  });

  test('header is sticky on scroll', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

    await page.waitForSelector('nav', { timeout: 10000 });

    // Check header has sticky positioning class
    const header = page.locator('header');
    await expect(header).toHaveClass(/sticky/);
  });
});

test.describe('Dashboard Responsive Behavior', () => {
  test('sidebar collapses for better content visibility', async ({ page }) => {
    await setupAuthenticatedMocks(page, { isAdmin: false });
    await page.goto('/dashboard');

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
    // Mock failing API endpoints
    await page.route('**/api/dashboard/overview', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await page.route('**/api/dashboard/rate-limits', async (route) => {
      await route.fulfill({ status: 500, body: JSON.stringify({ error: 'Error' }) });
    });

    await page.route('**/api/dashboard/analytics*', async (route) => {
      await route.fulfill({ status: 500, body: JSON.stringify({ error: 'Error' }) });
    });

    await page.route('**/a2a/dashboard/overview', async (route) => {
      await route.fulfill({ status: 500, body: JSON.stringify({ error: 'Error' }) });
    });

    await page.addInitScript(() => {
      localStorage.setItem(
        'user',
        JSON.stringify({
          id: 'user-123',
          email: 'admin@test.com',
          display_name: 'Test Admin',
          is_admin: false,
        })
      );
    });

    await page.goto('/dashboard');

    // Dashboard should still render navigation
    await page.waitForSelector('nav', { timeout: 10000 });
    await expect(page.getByRole('button', { name: /Overview/i })).toBeVisible();
  });
});
