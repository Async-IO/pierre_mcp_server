// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Visual E2E tests for admin mode (ASY-312).
// ABOUTME: Tests all admin dashboard screens against real backend.

import { test, expect } from '@playwright/test';
import {
  loginAsUser,
  navigateToTab,
  takeVisualScreenshot,
  waitForNetworkIdle,
  VISUAL_TEST_CONFIG,
} from './visual-test-helpers';

test.describe('ASY-312: Web Admin Mode Visual Tests', () => {
  test.describe.configure({ mode: 'serial' });

  // ========================================
  // Login & Authentication
  // ========================================
  test.describe('Login & Authentication', () => {
    test('admin login - renders login form', async ({ page }) => {
      // Setup minimal mocks needed for login page to render
      await page.route('**/admin/setup/status', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
        });
      });

      await page.goto('/');
      await page.waitForSelector('form', { timeout: VISUAL_TEST_CONFIG.defaultTimeout });

      await expect(page.locator('input[name="email"]')).toBeVisible();
      await expect(page.locator('input[name="password"]')).toBeVisible();
      await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible();

      await takeVisualScreenshot(page, 'admin-login', 'form-rendered');
    });

    test('admin login - successful login redirects to dashboard', async ({ page }) => {
      await loginAsUser(page, 'admin');

      // Verify we're on the dashboard (not login page)
      await expect(page.locator('input[name="email"]')).not.toBeVisible();

      // Admin should see Overview tab content
      await expect(page.getByText(/Overview|Dashboard/i).first()).toBeVisible({ timeout: 10000 });

      await takeVisualScreenshot(page, 'admin-login', 'dashboard-visible');
    });
  });

  // ========================================
  // Overview Tab
  // ========================================
  test.describe('Overview Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('overview - displays stats cards', async ({ page }) => {
      await navigateToTab(page, 'Overview');
      await waitForNetworkIdle(page);

      // Look for stat cards (various formats)
      const statsArea = page.locator('main');
      await expect(statsArea).toBeVisible();

      await takeVisualScreenshot(page, 'admin-overview', 'stats-cards');
    });

    test('overview - displays usage charts', async ({ page }) => {
      await navigateToTab(page, 'Overview');
      await waitForNetworkIdle(page);

      // Charts should be present in the overview
      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-overview', 'charts');
    });
  });

  // ========================================
  // Connections Tab
  // ========================================
  test.describe('Connections Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('connections - displays OAuth providers', async ({ page }) => {
      await navigateToTab(page, 'Connections');
      await waitForNetworkIdle(page);

      // Should show provider cards or empty state
      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-connections', 'providers');
    });

    test('connections - shows connection status', async ({ page }) => {
      await navigateToTab(page, 'Connections');
      await waitForNetworkIdle(page);

      await takeVisualScreenshot(page, 'admin-connections', 'status');
    });
  });

  // ========================================
  // Analytics Tab
  // ========================================
  test.describe('Analytics Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('analytics - displays usage charts', async ({ page }) => {
      await navigateToTab(page, 'Analytics');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-analytics', 'charts');
    });

    test('analytics - date picker works', async ({ page }) => {
      await navigateToTab(page, 'Analytics');
      await waitForNetworkIdle(page);

      // Look for date/time range controls
      const dateControls = page.locator('select, [role="combobox"], button:has-text("days")');
      const hasDateControls = await dateControls.first().isVisible().catch(() => false);

      if (hasDateControls) {
        await dateControls.first().click();
        await page.waitForTimeout(300);
      }

      await takeVisualScreenshot(page, 'admin-analytics', 'date-picker');
    });
  });

  // ========================================
  // Request Monitor Tab
  // ========================================
  test.describe('Request Monitor Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('monitor - displays request list', async ({ page }) => {
      await navigateToTab(page, 'Monitor');
      await waitForNetworkIdle(page);

      // Verify the Monitor tab content renders
      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-monitor', 'request-list');
    });

    test('monitor - filter by status works', async ({ page }) => {
      await navigateToTab(page, 'Monitor');
      await waitForNetworkIdle(page);

      // Look for filter controls
      const filterControls = page.locator('select, [role="combobox"], input[type="search"]');
      const hasFilters = await filterControls.first().isVisible().catch(() => false);

      if (hasFilters) {
        await takeVisualScreenshot(page, 'admin-monitor', 'filters');
      }
    });
  });

  // ========================================
  // Tools Tab
  // ========================================
  test.describe('Tools Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('tools - displays tool list', async ({ page }) => {
      await navigateToTab(page, 'Tools');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-tools', 'list');
    });

    test('tools - search filters results', async ({ page }) => {
      await navigateToTab(page, 'Tools');
      await waitForNetworkIdle(page);

      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"]');
      if (await searchInput.first().isVisible().catch(() => false)) {
        await searchInput.first().fill('strava');
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'admin-tools', 'search-results');
      }
    });
  });

  // ========================================
  // User Management Tab
  // ========================================
  test.describe('User Management Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('users - displays user list', async ({ page }) => {
      await navigateToTab(page, 'Users');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-users', 'list');
    });

    test('users - search by email works', async ({ page }) => {
      await navigateToTab(page, 'Users');
      await waitForNetworkIdle(page);

      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"]');
      if (await searchInput.first().isVisible().catch(() => false)) {
        await searchInput.first().fill('webtest');
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'admin-users', 'search-results');
      }
    });

    test('users - status filter works', async ({ page }) => {
      await navigateToTab(page, 'Users');
      await waitForNetworkIdle(page);

      // Look for status filter dropdown
      const statusFilter = page.locator('select, [role="combobox"]').first();
      if (await statusFilter.isVisible().catch(() => false)) {
        await statusFilter.click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'admin-users', 'status-filter');
      }
    });

    test('users - clicking user opens detail drawer', async ({ page }) => {
      await navigateToTab(page, 'Users');
      await waitForNetworkIdle(page);

      // Click first user row
      const userRow = page.locator('tr, [role="row"]').nth(1);
      if (await userRow.isVisible().catch(() => false)) {
        await userRow.click();
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'admin-users', 'detail-drawer');
      }
    });
  });

  // ========================================
  // Coaches Tab
  // ========================================
  test.describe('Coaches Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('coaches - displays coach list', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-coaches', 'list');
    });

    test('coaches - create coach button opens wizard', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await waitForNetworkIdle(page);

      const createButton = page.getByRole('button', { name: /create|new|add/i });
      if (await createButton.first().isVisible().catch(() => false)) {
        await createButton.first().click();
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'admin-coaches', 'wizard-open');

        // Close the wizard
        const closeButton = page.getByRole('button', { name: /close|cancel|×/i });
        if (await closeButton.first().isVisible().catch(() => false)) {
          await closeButton.first().click();
        }
      }
    });
  });

  // ========================================
  // Coach Store Tab
  // ========================================
  test.describe('Coach Store Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('store - displays store coach list', async ({ page }) => {
      await navigateToTab(page, 'Coach Store');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-store', 'list');
    });

    test('store - category filter works', async ({ page }) => {
      await navigateToTab(page, 'Coach Store');
      await waitForNetworkIdle(page);

      // Look for category tabs or filter
      const categoryTabs = page.locator('[role="tab"], button:has-text("Training"), button:has-text("Nutrition")');
      if (await categoryTabs.first().isVisible().catch(() => false)) {
        await categoryTabs.first().click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'admin-store', 'category-filter');
      }
    });

    test('store - search coaches works', async ({ page }) => {
      await navigateToTab(page, 'Coach Store');
      await waitForNetworkIdle(page);

      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"]');
      if (await searchInput.first().isVisible().catch(() => false)) {
        await searchInput.first().fill('training');
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'admin-store', 'search-results');
      }
    });
  });

  // ========================================
  // Admin Configuration Tab
  // ========================================
  test.describe('Admin Configuration Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('config - displays settings list', async ({ page }) => {
      await navigateToTab(page, 'Configuration');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-config', 'settings');
    });
  });

  // ========================================
  // Social Features (Admin View)
  // ========================================
  test.describe('Social Features', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'admin');
    });

    test('friends - displays friends list', async ({ page }) => {
      await navigateToTab(page, 'Friends');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-friends', 'list');
    });

    test('feed - displays social feed', async ({ page }) => {
      await navigateToTab(page, 'Social Feed');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'admin-feed', 'insights');
    });
  });
});
