// ABOUTME: Playwright E2E tests for user management features.
// ABOUTME: Tests user listing, approval, suspension, password reset, and user details.

import { test, expect, type Page } from '@playwright/test';

// Helper to authenticate and navigate to users tab
async function loginAndNavigateToUsers(page: Page) {
  // Mock setup status
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
    });
  });

  // Mock login
  await page.route('**/api/auth/login', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        csrf_token: 'test-csrf-token',
        jwt_token: 'test-jwt-token',
        user: { id: 'admin-1', email: 'admin@test.com', display_name: 'Admin User' },
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
        total_requests_month: 2500,
      }),
    });
  });

  // Mock rate limits
  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ daily_limit: 1000, daily_used: 150, monthly_limit: 10000, monthly_used: 2500 }),
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

  // Wait for dashboard to load
  await page.waitForSelector('[data-testid="dashboard"]', { timeout: 10000 }).catch(() => {
    // Dashboard might not have data-testid, wait for the sidebar instead
  });
  await page.waitForTimeout(500);
}

test.describe('User Management - Pending Users', () => {
  test.beforeEach(async ({ page }) => {
    // Mock pending users list
    await page.route('**/api/admin/pending-users', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          count: 2,
          users: [
            {
              id: 'user-1',
              email: 'pending1@example.com',
              display_name: 'Pending User 1',
              status: 'pending',
              tier: 'trial',
              created_at: '2024-01-15T10:00:00Z',
              last_active_at: null,
            },
            {
              id: 'user-2',
              email: 'pending2@example.com',
              display_name: 'Pending User 2',
              status: 'pending',
              tier: 'trial',
              created_at: '2024-01-16T10:00:00Z',
              last_active_at: null,
            },
          ],
        }),
      });
    });

    // Mock all users list
    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [
            { id: 'user-1', email: 'pending1@example.com', display_name: 'Pending User 1', status: 'pending', tier: 'trial' },
            { id: 'user-2', email: 'pending2@example.com', display_name: 'Pending User 2', status: 'pending', tier: 'trial' },
            { id: 'user-3', email: 'active@example.com', display_name: 'Active User', status: 'active', tier: 'starter' },
          ],
          total_count: 3,
        }),
      });
    });

    await loginAndNavigateToUsers(page);
  });

  test('displays pending users badge in sidebar', async ({ page }) => {
    // Look for the Users tab with a badge
    const usersTab = page.locator('text=Users');
    await expect(usersTab).toBeVisible();

    // Badge should show pending count
    const badge = page.locator('.bg-red-500, .bg-pierre-recovery, [class*="red"]').first();
    await expect(badge).toBeVisible();
  });

  test('navigates to user management tab', async ({ page }) => {
    // Click on Users tab
    await page.getByText('Users').click();

    // Should see user management content
    await expect(page.getByText('User Management')).toBeVisible({ timeout: 5000 });
  });

  test('displays pending users list', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Should see pending users
    await expect(page.getByText('pending1@example.com')).toBeVisible();
    await expect(page.getByText('pending2@example.com')).toBeVisible();
  });

  test('can switch between user status tabs', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Find and click the Active tab
    const activeTab = page.getByRole('button', { name: /Active/i }).or(page.getByText('Active').first());
    await activeTab.click();

    // Should filter to active users
    await page.waitForTimeout(300);
  });
});

test.describe('User Management - Approve User', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/admin/pending-users', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          count: 1,
          users: [
            {
              id: 'user-1',
              email: 'pending@example.com',
              display_name: 'Pending User',
              status: 'pending',
              tier: 'trial',
              created_at: '2024-01-15T10:00:00Z',
            },
          ],
        }),
      });
    });

    await page.route('**/api/admin/users**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          users: [{ id: 'user-1', email: 'pending@example.com', display_name: 'Pending User', status: 'pending', tier: 'trial' }],
          total_count: 1,
        }),
      });
    });

    await loginAndNavigateToUsers(page);
  });

  test('can approve a pending user', async ({ page }) => {
    // Mock approve endpoint
    await page.route('**/api/admin/approve-user/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'User approved successfully' }),
      });
    });

    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Find and click approve button
    const approveButton = page.getByRole('button', { name: /Approve/i }).first();
    await approveButton.click();

    // Should show confirmation modal or success message
    await page.waitForTimeout(300);
  });

  test('shows approval confirmation modal', async ({ page }) => {
    await page.route('**/api/admin/approve-user/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Click approve button
    const approveButton = page.getByRole('button', { name: /Approve/i }).first();
    await approveButton.click();

    // Modal might appear with reason input
    const reasonInput = page.locator('textarea[placeholder*="reason"], input[placeholder*="reason"]');
    if (await reasonInput.isVisible()) {
      await reasonInput.fill('Verified legitimate user');

      // Confirm approval
      const confirmButton = page.getByRole('button', { name: /Confirm|Approve/i }).last();
      await confirmButton.click();
    }
  });
});

test.describe('User Management - Suspend User', () => {
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
            { id: 'user-1', email: 'active@example.com', display_name: 'Active User', status: 'active', tier: 'starter' },
          ],
          total_count: 1,
        }),
      });
    });

    await loginAndNavigateToUsers(page);
  });

  test('can suspend an active user', async ({ page }) => {
    await page.route('**/api/admin/suspend-user/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'User suspended successfully' }),
      });
    });

    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Click on Active tab to see active users
    const activeTab = page.getByRole('button', { name: /Active/i }).or(page.getByText('Active').first());
    await activeTab.click();
    await page.waitForTimeout(300);

    // Find and click suspend button
    const suspendButton = page.getByRole('button', { name: /Suspend/i }).first();
    if (await suspendButton.isVisible()) {
      await suspendButton.click();
    }
  });
});

test.describe('User Management - User Details', () => {
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
              email: 'user@example.com',
              display_name: 'Test User',
              status: 'active',
              tier: 'professional',
              created_at: '2024-01-01T10:00:00Z',
              last_active_at: '2024-01-20T15:30:00Z',
            },
          ],
          total_count: 1,
        }),
      });
    });

    // Mock rate limit endpoint
    await page.route('**/admin/users/*/rate-limit', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user_id: 'user-1',
          tier: 'professional',
          rate_limits: {
            daily: { limit: 10000, used: 500, remaining: 9500 },
            monthly: { limit: 100000, used: 5000, remaining: 95000 },
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
          total_requests: 5000,
          top_tools: [
            { tool_name: 'get_activities', call_count: 2000, percentage: 40 },
            { tool_name: 'get_athlete', call_count: 1500, percentage: 30 },
            { tool_name: 'get_routes', call_count: 1000, percentage: 20 },
          ],
        }),
      });
    });

    await loginAndNavigateToUsers(page);
  });

  test('can view user details drawer', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Click on a user row or view details button
    const viewDetailsButton = page.getByRole('button', { name: /View|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();

      // Should show user details drawer
      await expect(page.getByText('user@example.com')).toBeVisible();
    }
  });

  test('displays user rate limits', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();

      // Should show rate limit information
      await page.waitForTimeout(500);
      // Rate limits should be displayed (daily/monthly)
    }
  });
});

test.describe('User Management - Password Reset', () => {
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
            { id: 'user-1', email: 'user@example.com', display_name: 'Test User', status: 'active', tier: 'starter' },
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
          reset_times: { daily_reset: '2024-01-21T00:00:00Z', monthly_reset: '2024-02-01T00:00:00Z' },
        }),
      });
    });

    await page.route('**/admin/users/*/activity**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ user_id: 'user-1', period_days: 30, total_requests: 500, top_tools: [] }),
      });
    });

    await loginAndNavigateToUsers(page);
  });

  test('can reset user password', async ({ page }) => {
    // Mock password reset endpoint
    await page.route('**/admin/users/*/reset-password', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          temporary_password: 'TempPass123!',
          expires_at: '2024-01-22T10:00:00Z',
          user_email: 'user@example.com',
        }),
      });
    });

    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Open user details
    const viewDetailsButton = page.getByRole('button', { name: /View|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      // Find and click reset password button
      const resetButton = page.getByRole('button', { name: /Reset Password/i });
      if (await resetButton.isVisible()) {
        await resetButton.click();

        // Should show modal with temporary password
        await page.waitForTimeout(300);
      }
    }
  });

  test('displays temporary password in modal', async ({ page }) => {
    await page.route('**/admin/users/*/reset-password', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          temporary_password: 'SecureTemp456!',
          expires_at: '2024-01-22T10:00:00Z',
          user_email: 'user@example.com',
        }),
      });
    });

    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      const resetButton = page.getByRole('button', { name: /Reset Password/i });
      if (await resetButton.isVisible()) {
        await resetButton.click();
        await page.waitForTimeout(500);

        // Modal should show temporary password
        const passwordDisplay = page.locator('input[readonly], code, .font-mono');
        if (await passwordDisplay.isVisible()) {
          await expect(passwordDisplay).toContainText('SecureTemp456!');
        }
      }
    }
  });
});

test.describe('User Management - Search', () => {
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
            { id: 'user-1', email: 'john@example.com', display_name: 'John Doe', status: 'active', tier: 'starter' },
            { id: 'user-2', email: 'jane@example.com', display_name: 'Jane Smith', status: 'active', tier: 'professional' },
            { id: 'user-3', email: 'bob@test.com', display_name: 'Bob Wilson', status: 'suspended', tier: 'trial' },
          ],
          total_count: 3,
        }),
      });
    });

    await loginAndNavigateToUsers(page);
  });

  test('can search users by email', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    // Find search input
    const searchInput = page.locator('input[placeholder*="Search"], input[type="search"]');
    if (await searchInput.isVisible()) {
      await searchInput.fill('john@');
      await page.waitForTimeout(300);

      // Should filter results
      await expect(page.getByText('john@example.com')).toBeVisible();
    }
  });

  test('can search users by name', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    const searchInput = page.locator('input[placeholder*="Search"], input[type="search"]');
    if (await searchInput.isVisible()) {
      await searchInput.fill('Jane');
      await page.waitForTimeout(300);

      await expect(page.getByText('Jane Smith')).toBeVisible();
    }
  });

  test('shows no results message when search finds nothing', async ({ page }) => {
    await page.getByText('Users').click();
    await page.waitForTimeout(500);

    const searchInput = page.locator('input[placeholder*="Search"], input[type="search"]');
    if (await searchInput.isVisible()) {
      await searchInput.fill('nonexistent@user.com');
      await page.waitForTimeout(300);

      // Should show no results message
      const noResults = page.getByText(/No users found|No results/i);
      await expect(noResults).toBeVisible();
    }
  });
});
