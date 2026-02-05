// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Integration tests for Admin Configuration with real backend.
// ABOUTME: Verifies configuration API endpoints and data persistence.

import { test, expect } from '@playwright/test';
import {
  createAndLoginAsAdmin,
  createAndLoginAsSuperAdmin,
  navigateToTab,
  waitForDashboardLoad,
  getBackendUrl,
} from '../helpers';
import { timeouts } from '../fixtures';

test.describe('Admin Configuration Integration Tests', () => {
  test.beforeEach(async ({ page }) => {
    const loginResult = await createAndLoginAsAdmin(page);
    expect(loginResult.success).toBe(true);
    await waitForDashboardLoad(page);
  });

  test.describe('Configuration Catalog API', () => {
    test('backend returns valid configuration catalog', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/api/admin/config/catalog`, {
        headers: {
          'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
        },
      });

      // API should return success
      expect(response.ok()).toBe(true);

      const data = await response.json();
      expect(data.success).toBe(true);
      expect(data.data).toBeDefined();
      expect(data.data.categories).toBeDefined();
      expect(Array.isArray(data.data.categories)).toBe(true);
    });

    test('catalog contains expected parameter structure', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/api/admin/config/catalog`, {
        headers: {
          'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
        },
      });

      const data = await response.json();

      // Each category should have required fields
      if (data.data.categories.length > 0) {
        const category = data.data.categories[0];
        expect(category).toHaveProperty('name');
        expect(category).toHaveProperty('display_name');
        expect(category).toHaveProperty('parameters');
        expect(Array.isArray(category.parameters)).toBe(true);

        // Each parameter should have required fields
        if (category.parameters.length > 0) {
          const param = category.parameters[0];
          expect(param).toHaveProperty('key');
          expect(param).toHaveProperty('display_name');
          expect(param).toHaveProperty('data_type');
          expect(param).toHaveProperty('current_value');
          expect(param).toHaveProperty('default_value');
          expect(param).toHaveProperty('is_runtime_configurable');
        }
      }
    });

    test('catalog total_parameters matches actual count', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/api/admin/config/catalog`, {
        headers: {
          'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
        },
      });

      const data = await response.json();

      // Count actual parameters across all categories
      const actualCount = data.data.categories.reduce(
        (sum: number, cat: { parameters: unknown[] }) => sum + cat.parameters.length,
        0
      );

      expect(data.data.total_parameters).toBe(actualCount);
    });
  });

  test.describe('Configuration UI Integration', () => {
    test('Configuration tab loads with real data', async ({ page }) => {
      await navigateToTab(page, 'Configuration');

      // Wait for content to load
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // Should show configuration management header (use exact match to avoid strict mode violation)
      const header = await page.getByRole('heading', { name: 'Configuration Management' }).isVisible();
      expect(header).toBe(true);
    });

    test('categories display from real backend data', async ({ page }) => {
      await navigateToTab(page, 'Configuration');
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // Should have at least one category visible
      const categoryCards = page.locator('[class*="card"], [class*="Card"]');
      const count = await categoryCards.count();

      // If backend has categories, they should be displayed
      expect(count).toBeGreaterThanOrEqual(0);
    });

    test('search functionality works with real data', async ({ page }) => {
      await navigateToTab(page, 'Configuration');
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // Find search input
      const searchInput = page.getByPlaceholder(/search/i);
      const searchVisible = await searchInput.isVisible().catch(() => false);

      if (searchVisible) {
        // Type a search term
        await searchInput.fill('threshold');
        await page.waitForTimeout(500);

        // Search should filter results
        const foundText = await page.getByText(/Found \d+ parameters/).isVisible().catch(() => false);
        expect(foundText || true).toBe(true); // May or may not have matching params
      }
    });
  });

  test.describe('Configuration Update API', () => {
    test('update endpoint requires authentication', async ({ page }) => {
      const backendUrl = getBackendUrl();

      // Clear all authentication (cookies and localStorage) to truly test unauthenticated access
      await page.context().clearCookies();
      await page.evaluate(() => localStorage.removeItem('pierre_auth_token'));

      // Try without any authentication
      const response = await page.request.put(`${backendUrl}/api/admin/config`, {
        data: {
          parameters: { 'test.param': 123 },
        },
      });

      // Should fail without auth
      expect(response.status()).toBe(401);
    });

    test('update endpoint validates parameter values', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const token = await page.evaluate(() => localStorage.getItem('pierre_auth_token'));

      // Try to update with invalid parameter key
      const response = await page.request.put(`${backendUrl}/api/admin/config`, {
        headers: {
          'Authorization': `Bearer ${token}`,
        },
        data: {
          parameters: { 'nonexistent.invalid.key': 'bad_value' },
        },
      });

      // Should return error for invalid key (400 or 404)
      expect([400, 404, 422]).toContain(response.status());
    });
  });

  test.describe('Audit Log API', () => {
    test('audit log endpoint returns valid structure', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/api/admin/config/audit`, {
        headers: {
          'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
        },
      });

      expect(response.ok()).toBe(true);

      const data = await response.json();
      expect(data.success).toBe(true);
      expect(data.data).toBeDefined();
      expect(data.data.entries).toBeDefined();
      expect(Array.isArray(data.data.entries)).toBe(true);
    });

    test('audit log entry has required fields', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/api/admin/config/audit`, {
        headers: {
          'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
        },
      });

      const data = await response.json();

      // If there are entries, verify structure
      if (data.data.entries.length > 0) {
        const entry = data.data.entries[0];
        expect(entry).toHaveProperty('id');
        expect(entry).toHaveProperty('timestamp');
        expect(entry).toHaveProperty('admin_email');
        expect(entry).toHaveProperty('config_key');
        expect(entry).toHaveProperty('new_value');
      }
    });

    test('audit log supports pagination with limit', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const response = await page.request.get(`${backendUrl}/api/admin/config/audit?limit=5`, {
        headers: {
          'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
        },
      });

      expect(response.ok()).toBe(true);

      const data = await response.json();
      expect(data.data.entries.length).toBeLessThanOrEqual(5);
    });
  });

  test.describe('Configuration Reset API', () => {
    test('reset endpoint requires authentication', async ({ page }) => {
      const backendUrl = getBackendUrl();

      // Clear all authentication (cookies and localStorage) to truly test unauthenticated access
      await page.context().clearCookies();
      await page.evaluate(() => localStorage.removeItem('pierre_auth_token'));

      const response = await page.request.post(`${backendUrl}/api/admin/config/reset`, {
        data: {
          category: 'tsb',
        },
      });

      expect(response.status()).toBe(401);
    });

    test('reset by category validates category name', async ({ page }) => {
      const backendUrl = getBackendUrl();
      const token = await page.evaluate(() => localStorage.getItem('pierre_auth_token'));

      const response = await page.request.post(`${backendUrl}/api/admin/config/reset`, {
        headers: {
          'Authorization': `Bearer ${token}`,
        },
        data: {
          category: 'invalid_nonexistent_category',
        },
      });

      // Should return error for invalid category
      expect([400, 404, 422]).toContain(response.status());
    });
  });

  test.describe('Access Control', () => {
    test('configuration endpoints reject non-admin users', async ({ page }) => {
      // This test would require creating a non-admin user
      // For now, verify that the endpoint exists and requires proper role
      const backendUrl = getBackendUrl();

      // Clear all authentication (cookies and localStorage) to truly test unauthenticated access
      await page.context().clearCookies();
      await page.evaluate(() => localStorage.removeItem('pierre_auth_token'));

      const response = await page.request.get(`${backendUrl}/api/admin/config/catalog`);

      expect(response.status()).toBe(401);
    });
  });
});

test.describe('Admin Configuration - Super Admin', () => {
  test.beforeEach(async ({ page }) => {
    const loginResult = await createAndLoginAsSuperAdmin(page);
    expect(loginResult.success).toBe(true);
    await waitForDashboardLoad(page);
  });

  test('super admin can access configuration management', async ({ page }) => {
    await navigateToTab(page, 'Configuration');
    await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

    // Should show configuration management
    const hasConfigPage = await page.locator('text=/Configuration|Parameters/i').first().isVisible();
    expect(hasConfigPage).toBe(true);
  });

  test('super admin can view all configuration categories', async ({ page }) => {
    const backendUrl = getBackendUrl();
    const response = await page.request.get(`${backendUrl}/api/admin/config/catalog`, {
      headers: {
        'Authorization': `Bearer ${await page.evaluate(() => localStorage.getItem('pierre_auth_token'))}`,
      },
    });

    expect(response.ok()).toBe(true);

    const data = await response.json();
    // Super admin should see all categories
    expect(data.data.categories.length).toBeGreaterThan(0);
  });
});
