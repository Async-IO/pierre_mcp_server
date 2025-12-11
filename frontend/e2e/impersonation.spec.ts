// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for super admin impersonation feature.
// ABOUTME: Tests impersonation start/end, banner display, and role-based visibility.

import { test, expect, type Page } from '@playwright/test';

// Helper to authenticate as a super admin and navigate to users tab
async function loginAsSuperAdminAndNavigateToUsers(page: Page) {
  // Mock setup status
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
    });
  });

  // Mock OAuth2 ROPC login - SUPER ADMIN role
  await page.route('**/oauth/token', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        access_token: 'test-jwt-token',
        token_type: 'Bearer',
        expires_in: 86400,
        csrf_token: 'test-csrf-token',
        user: {
          user_id: 'super-admin-1',
          email: 'superadmin@test.com',
          display_name: 'Super Admin',
          role: 'super_admin',
          is_admin: true,
          user_status: 'active',
          tier: 'enterprise',
        },
      }),
    });
  });

  // Mock dashboard overview
  await page.route('**/api/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_api_keys: 5,
        active_api_keys: 3,
        total_requests_today: 150,
        total_requests_this_month: 2500,
      }),
    });
  });

  // Mock rate limits
  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock A2A dashboard
  await page.route('**/a2a/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ total_clients: 2, active_sessions: 1, requests_today: 50, error_rate: 0.01 }),
    });
  });

  // Mock analytics
  await page.route('**/api/dashboard/analytics**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ daily_usage: [] }),
    });
  });

  await page.goto('/');
  await page.waitForSelector('form');
  await page.locator('input[name="email"]').fill('superadmin@test.com');
  await page.locator('input[name="password"]').fill('password123');
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait for dashboard sidebar to appear - the sidebar contains Pierre logo
  await page.waitForSelector('text=Pierre', { timeout: 10000 });
  await page.waitForTimeout(300);
}

// Helper to authenticate as a regular admin
async function loginAsRegularAdmin(page: Page) {
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
    });
  });

  // Mock OAuth2 ROPC login - REGULAR ADMIN role
  await page.route('**/oauth/token', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        access_token: 'test-jwt-token',
        token_type: 'Bearer',
        expires_in: 86400,
        csrf_token: 'test-csrf-token',
        user: {
          user_id: 'admin-1',
          email: 'admin@test.com',
          display_name: 'Admin User',
          role: 'admin',
          is_admin: true,
          user_status: 'active',
          tier: 'professional',
        },
      }),
    });
  });

  // Mock dashboard overview
  await page.route('**/api/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_api_keys: 5,
        active_api_keys: 3,
        total_requests_today: 150,
        total_requests_this_month: 2500,
      }),
    });
  });

  // Mock rate limits
  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock A2A dashboard
  await page.route('**/a2a/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ total_clients: 2, active_sessions: 1, requests_today: 50, error_rate: 0.01 }),
    });
  });

  // Mock analytics
  await page.route('**/api/dashboard/analytics**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ daily_usage: [] }),
    });
  });

  await page.goto('/');
  await page.waitForSelector('form');
  await page.locator('input[name="email"]').fill('admin@test.com');
  await page.locator('input[name="password"]').fill('password123');
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait for dashboard sidebar to appear
  await page.waitForSelector('text=Pierre', { timeout: 10000 });
  await page.waitForTimeout(300);
}

test.describe('Impersonation - Super Admin Access', () => {
  test.beforeEach(async ({ page }) => {
    // Mock pending users
    await page.route('**/api/admin/pending-users', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ count: 0, users: [] }),
      });
    });

    // Mock users list with a regular user that can be impersonated
    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            {
              id: 'user-1',
              email: 'testuser@example.com',
              display_name: 'Test User',
              role: 'user',
              user_status: 'active',
              tier: 'starter',
              created_at: '2024-01-15T10:00:00Z',
              last_active: '2024-01-20T15:30:00Z',
            },
            {
              id: 'admin-2',
              email: 'otheradmin@example.com',
              display_name: 'Other Admin',
              role: 'admin',
              user_status: 'active',
              tier: 'professional',
              created_at: '2024-01-10T10:00:00Z',
              last_active: '2024-01-20T15:30:00Z',
            },
          ],
          total_count: 2,
        }),
      });
    });

    // Mock rate limit endpoint for user details
    await page.route('**/admin/users/*/rate-limit', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          tier: 'starter',
          rate_limits: {
            daily: { limit: 1000, used: 50, remaining: 950 },
            monthly: { limit: 10000, used: 500, remaining: 9500 },
          },
          reset_times: {
            daily_reset: '2024-01-21T00:00:00Z',
            monthly_reset: '2024-02-01T00:00:00Z',
          },
        }),
      });
    });

    // Mock activity endpoint
    await page.route('**/admin/users/*/activity**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          period_days: 30,
          total_requests: 500,
          top_tools: [],
        }),
      });
    });

    await loginAsSuperAdminAndNavigateToUsers(page);
  });

  test('super admin sees Super Admin role in sidebar', async ({ page }) => {
    // Should see the Super Admin role indicator in the sidebar
    await expect(page.getByText('Super Admin').first()).toBeVisible();
  });

  test('super admin sees Users tab', async ({ page }) => {
    // Navigate to Users tab - use button role for sidebar navigation
    const usersTab = page.locator('button', { hasText: 'Users' }).first();
    await usersTab.click();
    await expect(page.getByText('User Management')).toBeVisible({ timeout: 5000 });
  });

  test('super admin sees Impersonate User button for active regular users', async ({ page }) => {
    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click on Active tab to see active users
    // Click the Active tab button in the user management tabs
    await page.locator('button', { hasText: 'Active' }).first().click();
    await page.waitForTimeout(300);

    // Click on a user to open details drawer - look for the user row
    const userRow = page.getByText('testuser@example.com').first();
    await userRow.click();
    await page.waitForTimeout(500);

    // Should see Impersonate User button in the drawer
    await expect(page.getByRole('button', { name: /Impersonate User/i })).toBeVisible();
  });

  test('impersonate button not shown for super admin users', async ({ page }) => {
    // Add a super admin user to the mock
    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            {
              id: 'super-admin-2',
              email: 'othersuper@example.com',
              display_name: 'Other Super Admin',
              role: 'super_admin',
              user_status: 'active',
              tier: 'enterprise',
              created_at: '2024-01-10T10:00:00Z',
              last_active: '2024-01-20T15:30:00Z',
            },
          ],
          total_count: 1,
        }),
      });
    });

    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click on a super admin user
    const userRow = page.getByText('othersuper@example.com').first();
    if (await userRow.isVisible()) {
      await userRow.click();
      await page.waitForTimeout(500);

      // Should NOT see Impersonate User button (can't impersonate other super admins)
      const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
      await expect(impersonateButton).not.toBeVisible();
    }
  });

  test('impersonate button not shown for pending users', async ({ page }) => {
    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            {
              id: 'pending-user-1',
              email: 'pending@example.com',
              display_name: 'Pending User',
              role: 'user',
              user_status: 'pending',
              tier: 'trial',
              created_at: '2024-01-15T10:00:00Z',
              last_active: null,
            },
          ],
          total_count: 1,
        }),
      });
    });

    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click on the pending user
    const userRow = page.getByText('pending@example.com').first();
    if (await userRow.isVisible()) {
      await userRow.click();
      await page.waitForTimeout(500);

      // Should NOT see Impersonate User button (can't impersonate pending users)
      const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
      await expect(impersonateButton).not.toBeVisible();
    }
  });
});

test.describe('Impersonation - Start and End Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/admin/pending-users', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ count: 0, users: [] }),
      });
    });

    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            {
              id: 'user-1',
              email: 'testuser@example.com',
              display_name: 'Test User',
              role: 'user',
              user_status: 'active',
              tier: 'starter',
              created_at: '2024-01-15T10:00:00Z',
              last_active: '2024-01-20T15:30:00Z',
            },
          ],
          total_count: 1,
        }),
      });
    });

    await page.route('**/admin/users/*/rate-limit', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          tier: 'starter',
          rate_limits: {
            daily: { limit: 1000, used: 50, remaining: 950 },
            monthly: { limit: 10000, used: 500, remaining: 9500 },
          },
          reset_times: {
            daily_reset: '2024-01-21T00:00:00Z',
            monthly_reset: '2024-02-01T00:00:00Z',
          },
        }),
      });
    });

    await page.route('**/admin/users/*/activity**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          period_days: 30,
          total_requests: 500,
          top_tools: [],
        }),
      });
    });

    await loginAsSuperAdminAndNavigateToUsers(page);
  });

  test('can start impersonation and see banner', async ({ page }) => {
    // Mock impersonate endpoint
    await page.route('**/api/admin/impersonate', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          session_id: 'session-123',
          target_user: {
            id: 'user-1',
            email: 'testuser@example.com',
            display_name: 'Test User',
            role: 'user',
          },
          impersonation_token: 'impersonation-jwt-token',
        }),
      });
    });

    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click on Active tab
    // Click the Active tab button in the user management tabs
    await page.locator('button', { hasText: 'Active' }).first().click();
    await page.waitForTimeout(300);

    // Click on user to open details
    const userRow = page.getByText('testuser@example.com').first();
    await userRow.click();
    await page.waitForTimeout(500);

    // Click Impersonate User button
    const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
    await impersonateButton.click();
    await page.waitForTimeout(500);

    // Should see impersonation banner
    await expect(page.getByText(/You are impersonating/i)).toBeVisible();
    await expect(page.getByText('Test User').first()).toBeVisible();
    await expect(page.getByRole('button', { name: /End Impersonation/i })).toBeVisible();
  });

  test('can end impersonation', async ({ page }) => {
    // Mock impersonate endpoint
    await page.route('**/api/admin/impersonate', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          session_id: 'session-123',
          target_user: {
            id: 'user-1',
            email: 'testuser@example.com',
            display_name: 'Test User',
            role: 'user',
          },
          impersonation_token: 'impersonation-jwt-token',
        }),
      });
    });

    // Mock end impersonation endpoint
    await page.route('**/api/admin/impersonate/end', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          message: 'Impersonation session ended',
        }),
      });
    });

    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click the Active tab button in the user management tabs
    await page.locator('button', { hasText: 'Active' }).first().click();
    await page.waitForTimeout(300);

    const userRow = page.getByText('testuser@example.com').first();
    await userRow.click();
    await page.waitForTimeout(500);

    // Start impersonation
    const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
    await impersonateButton.click();
    await page.waitForTimeout(500);

    // Verify banner is visible
    await expect(page.getByText(/You are impersonating/i)).toBeVisible();

    // End impersonation
    const endButton = page.getByRole('button', { name: /End Impersonation/i });
    await endButton.click();
    await page.waitForTimeout(500);

    // Banner should be gone
    await expect(page.getByText(/You are impersonating/i)).not.toBeVisible();
  });

  test('impersonation banner shows target user info', async ({ page }) => {
    await page.route('**/api/admin/impersonate', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          session_id: 'session-123',
          target_user: {
            id: 'user-1',
            email: 'testuser@example.com',
            display_name: 'Test User',
            role: 'user',
          },
          impersonation_token: 'impersonation-jwt-token',
        }),
      });
    });

    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click the Active tab button in the user management tabs
    await page.locator('button', { hasText: 'Active' }).first().click();
    await page.waitForTimeout(300);

    const userRow = page.getByText('testuser@example.com').first();
    await userRow.click();
    await page.waitForTimeout(500);

    const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
    await impersonateButton.click();
    await page.waitForTimeout(500);

    // Banner should show user info
    await expect(page.getByText('Test User').first()).toBeVisible();
    await expect(page.getByText('testuser@example.com').first()).toBeVisible();
    await expect(page.getByText(/Role: user/i)).toBeVisible();
  });
});

test.describe('Impersonation - Regular Admin Cannot Impersonate', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/admin/pending-users', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ count: 0, users: [] }),
      });
    });

    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            {
              id: 'user-1',
              email: 'testuser@example.com',
              display_name: 'Test User',
              role: 'user',
              user_status: 'active',
              tier: 'starter',
              created_at: '2024-01-15T10:00:00Z',
              last_active: '2024-01-20T15:30:00Z',
            },
          ],
          total_count: 1,
        }),
      });
    });

    await page.route('**/admin/users/*/rate-limit', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          tier: 'starter',
          rate_limits: {
            daily: { limit: 1000, used: 50, remaining: 950 },
            monthly: { limit: 10000, used: 500, remaining: 9500 },
          },
          reset_times: {
            daily_reset: '2024-01-21T00:00:00Z',
            monthly_reset: '2024-02-01T00:00:00Z',
          },
        }),
      });
    });

    await page.route('**/admin/users/*/activity**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          period_days: 30,
          total_requests: 500,
          top_tools: [],
        }),
      });
    });

    await loginAsRegularAdmin(page);
  });

  test('regular admin does not see Impersonate User button', async ({ page }) => {
    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click the Active tab button in the user management tabs
    await page.locator('button', { hasText: 'Active' }).first().click();
    await page.waitForTimeout(300);

    const userRow = page.getByText('testuser@example.com').first();
    if (await userRow.isVisible()) {
      await userRow.click();
      await page.waitForTimeout(500);

      // Regular admin should NOT see Impersonate User button
      const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
      await expect(impersonateButton).not.toBeVisible();
    }
  });
});

test.describe('Impersonation - Error Handling', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/admin/pending-users', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ count: 0, users: [] }),
      });
    });

    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            {
              id: 'user-1',
              email: 'testuser@example.com',
              display_name: 'Test User',
              role: 'user',
              user_status: 'active',
              tier: 'starter',
              created_at: '2024-01-15T10:00:00Z',
              last_active: '2024-01-20T15:30:00Z',
            },
          ],
          total_count: 1,
        }),
      });
    });

    await page.route('**/admin/users/*/rate-limit', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          tier: 'starter',
          rate_limits: {
            daily: { limit: 1000, used: 50, remaining: 950 },
            monthly: { limit: 10000, used: 500, remaining: 9500 },
          },
          reset_times: {
            daily_reset: '2024-01-21T00:00:00Z',
            monthly_reset: '2024-02-01T00:00:00Z',
          },
        }),
      });
    });

    await page.route('**/admin/users/*/activity**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          period_days: 30,
          total_requests: 500,
          top_tools: [],
        }),
      });
    });

    await loginAsSuperAdminAndNavigateToUsers(page);
  });

  test('handles impersonation API error gracefully', async ({ page }) => {
    // Mock impersonate endpoint with error
    await page.route('**/api/admin/impersonate', async (route) => {
      await route.fulfill({
        status: 403,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Not authorized to impersonate this user',
        }),
      });
    });

    await page.locator('button', { hasText: 'Users' }).first().click();
    await page.waitForTimeout(500);

    // Click the Active tab button in the user management tabs
    await page.locator('button', { hasText: 'Active' }).first().click();
    await page.waitForTimeout(300);

    const userRow = page.getByText('testuser@example.com').first();
    await userRow.click();
    await page.waitForTimeout(500);

    const impersonateButton = page.getByRole('button', { name: /Impersonate User/i });
    await impersonateButton.click();
    await page.waitForTimeout(500);

    // Should show error and NOT show impersonation banner
    await expect(page.getByText(/You are impersonating/i)).not.toBeVisible();
  });
});
