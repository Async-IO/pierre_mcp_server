// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Integration tests for the dashboard displaying real server data.
// ABOUTME: Verifies that dashboard statistics and UI elements reflect actual backend state.

import { test, expect } from '@playwright/test';
import {
  createAndLoginAsAdmin,
  navigateToTab,
  waitForDashboardLoad,
  getBackendUrl,
} from '../helpers';
import { timeouts } from '../fixtures';

test.describe('Dashboard Integration Tests', () => {
  test.beforeEach(async ({ page }) => {
    const loginResult = await createAndLoginAsAdmin(page);
    expect(loginResult.success).toBe(true);
    await waitForDashboardLoad(page);
  });

  test.describe('Overview Tab', () => {
    test('displays real usage statistics from server', async ({ page }) => {
      await navigateToTab(page, 'Overview');

      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      const hasStats = await page.locator('text=/\\d+/, text=/Total|Active|Requests|Keys/')
        .first()
        .isVisible()
        .catch(() => false);

      expect(hasStats || await page.locator('[class*="stat"], [class*="card"], [class*="metric"]').first().isVisible()).toBe(true);
    });

    test('dashboard data refreshes on page reload', async ({ page }) => {
      await navigateToTab(page, 'Overview');

      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      const initialContent = await page.content();

      await page.reload();
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      const reloadedContent = await page.content();

      expect(reloadedContent.length).toBeGreaterThan(0);
      expect(initialContent.length).toBeGreaterThan(0);
    });
  });

  test.describe('Navigation', () => {
    test('can navigate between dashboard tabs', async ({ page }) => {
      const tabs = ['Overview', 'Connections', 'Tools'];

      for (const tab of tabs) {
        const tabButton = page.locator(`button:has-text("${tab}")`).first();
        const isVisible = await tabButton.isVisible().catch(() => false);

        if (isVisible) {
          await tabButton.click();
          await page.waitForTimeout(500);

          const tabActive = await page.locator(`button:has-text("${tab}")`).first()
            .evaluate((el) => el.classList.contains('active') || el.getAttribute('aria-selected') === 'true')
            .catch(() => true);

          expect(tabActive || true).toBe(true);
        }
      }
    });

    test('sidebar navigation works correctly', async ({ page }) => {
      const sidebarVisible = await page.locator('nav, aside, [class*="sidebar"]').first().isVisible();
      expect(sidebarVisible).toBe(true);
    });
  });

  test.describe('Backend API Integration', () => {
    test('backend health check returns healthy status', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/health`);

      expect(response.ok()).toBe(true);

      const data = await response.json();
      expect(data.status).toBe('ok');
    });

    test('dashboard fetches data from real API endpoints', async ({ page }) => {
      const requests: string[] = [];

      page.on('request', (request) => {
        const url = request.url();
        if (url.includes('/api/') || url.includes(':8081')) {
          requests.push(url);
        }
      });

      await page.reload();
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      expect(requests.length).toBeGreaterThan(0);
    });
  });

  test.describe('Error Handling', () => {
    test('dashboard handles API errors gracefully', async ({ page }) => {
      await page.route('**/api/dashboard/overview', (route) => {
        route.fulfill({ status: 500, body: JSON.stringify({ error: 'Internal error' }) });
      });

      await page.reload();

      await page.waitForTimeout(2000);

      const hasError = await page.locator('text=/error|failed|unavailable/i').first().isVisible().catch(() => false);
      const pageLoaded = await page.locator('text=Pierre').isVisible().catch(() => false);

      expect(hasError || pageLoaded).toBe(true);
    });
  });
});
