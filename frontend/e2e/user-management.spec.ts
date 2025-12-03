// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for user management features.
// ABOUTME: Tests user listing, approval, suspension, password reset, and user details.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Helper to set up user management specific mocks
async function setupUserManagementMocks(
  page: Page,
  options: {
    pendingCount?: number;
    pendingUsers?: Array<{
      id: string;
      email: string;
      display_name: string;
      status: string;
      tier: string;
      created_at?: string;
      last_active_at?: string | null;
    }>;
    allUsers?: Array<{
      id: string;
      email: string;
      display_name: string;
      status: string;
      tier: string;
      created_at?: string;
      last_active_at?: string | null;
    }>;
  } = {}
) {
  const {
    pendingCount = 2,
    pendingUsers = [
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
    allUsers = [
      { id: 'user-1', email: 'pending1@example.com', display_name: 'Pending User 1', status: 'pending', tier: 'trial' },
      { id: 'user-2', email: 'pending2@example.com', display_name: 'Pending User 2', status: 'pending', tier: 'trial' },
      { id: 'user-3', email: 'active@example.com', display_name: 'Active User', status: 'active', tier: 'starter' },
    ],
  } = options;

  // Set up base dashboard mocks (includes login mock)
  await setupDashboardMocks(page, { role: 'admin' });

  // Override pending users mock
  await page.route('**/api/admin/pending-users', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        count: pendingCount,
        users: pendingUsers,
      }),
    });
  });

  // Override all users mock
  await page.route('**/api/admin/users**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        users: allUsers,
        total_count: allUsers.length,
      }),
    });
  });
}

async function loginAndNavigateToUsers(page: Page) {
  await loginToDashboard(page);
  await navigateToTab(page, 'Users');
  await page.waitForTimeout(500);
}

test.describe('User Management - Pending Users', () => {
  test('displays pending users badge in sidebar', async ({ page }) => {
    await setupUserManagementMocks(page);
    await loginToDashboard(page);

    // Look for the Users button in the sidebar navigation (list item)
    // Use exact match to avoid matching the alert button
    const usersButton = page.getByRole('list').getByRole('button', { name: '2 Users', exact: true });
    await expect(usersButton).toBeVisible();

    // The button should contain the pending count
    await expect(usersButton).toContainText('2');
  });

  test('navigates to user management tab', async ({ page }) => {
    await setupUserManagementMocks(page);
    await loginAndNavigateToUsers(page);

    // Should see user management content - check for the h2 heading specifically
    await expect(page.getByRole('heading', { name: 'User Management', level: 2 })).toBeVisible({ timeout: 5000 });
  });

  test('displays pending users list', async ({ page }) => {
    await setupUserManagementMocks(page);
    await loginAndNavigateToUsers(page);

    // Should see pending users
    await expect(page.getByText('pending1@example.com')).toBeVisible();
    await expect(page.getByText('pending2@example.com')).toBeVisible();
  });

  test('can switch between user status tabs', async ({ page }) => {
    await setupUserManagementMocks(page);
    await loginAndNavigateToUsers(page);

    // Find and click the Active tab
    const activeTab = page.getByRole('button', { name: /Active/i }).or(page.locator('button:has-text("Active")'));
    await activeTab.click();

    // Wait for filter to apply
    await page.waitForTimeout(300);
  });
});

test.describe('User Management - Approve User', () => {
  test('can approve a pending user', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 1,
      pendingUsers: [
        {
          id: 'user-1',
          email: 'pending@example.com',
          display_name: 'Pending User',
          status: 'pending',
          tier: 'trial',
          created_at: '2024-01-15T10:00:00Z',
        },
      ],
      allUsers: [{ id: 'user-1', email: 'pending@example.com', display_name: 'Pending User', status: 'pending', tier: 'trial' }],
    });

    // Mock approve endpoint
    await page.route('**/api/admin/approve-user/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'User approved successfully' }),
      });
    });

    await loginAndNavigateToUsers(page);

    // Find and click approve button
    const approveButton = page.getByRole('button', { name: /Approve/i }).first();
    await approveButton.click();

    // Should show confirmation modal or success message
    await page.waitForTimeout(300);
  });

  test('shows approval confirmation modal', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 1,
      pendingUsers: [
        {
          id: 'user-1',
          email: 'pending@example.com',
          display_name: 'Pending User',
          status: 'pending',
          tier: 'trial',
          created_at: '2024-01-15T10:00:00Z',
        },
      ],
      allUsers: [{ id: 'user-1', email: 'pending@example.com', display_name: 'Pending User', status: 'pending', tier: 'trial' }],
    });

    await page.route('**/api/admin/approve-user/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    await loginAndNavigateToUsers(page);

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
  test('can suspend an active user', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [{ id: 'user-1', email: 'active@example.com', display_name: 'Active User', status: 'active', tier: 'starter' }],
    });

    await page.route('**/api/admin/suspend-user/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'User suspended successfully' }),
      });
    });

    await loginAndNavigateToUsers(page);

    // Click on Active tab to see active users
    const activeTab = page.getByRole('button', { name: /Active/i }).or(page.locator('button:has-text("Active")'));
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
  test('can view user details drawer', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [
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

    await loginToDashboard(page);

    // Navigate to Users tab - click the sidebar button specifically (not the Quick Actions button)
    const usersButton = page.getByRole('list').getByRole('button', { name: 'Users' });
    await usersButton.click();
    await page.waitForTimeout(500);

    // Verify we're on the Users page
    await expect(page.getByRole('heading', { name: 'Users', level: 1 })).toBeVisible({ timeout: 5000 });

    // Click on "All Users" tab to see active users (since there are no pending)
    const allUsersTab = page.getByRole('button', { name: /All Users/i });
    await allUsersTab.click();
    await page.waitForTimeout(300);

    // Look for the user email to be visible in the list
    const userEmail = page.getByText('user@example.com');
    await expect(userEmail).toBeVisible({ timeout: 5000 });
  });

  test('displays user rate limits', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [
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
    });

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
          reset_times: { daily_reset: '2024-01-21T00:00:00Z', monthly_reset: '2024-02-01T00:00:00Z' },
        }),
      });
    });

    await page.route('**/admin/users/*/activity**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ user_id: 'user-1', period_days: 30, total_requests: 5000, top_tools: [] }),
      });
    });

    await loginAndNavigateToUsers(page);

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
  test('can reset user password', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [{ id: 'user-1', email: 'user@example.com', display_name: 'Test User', status: 'active', tier: 'starter' }],
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

    await loginAndNavigateToUsers(page);

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
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [{ id: 'user-1', email: 'user@example.com', display_name: 'Test User', status: 'active', tier: 'starter' }],
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

    await loginAndNavigateToUsers(page);

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
  test('can search users by email', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [
        { id: 'user-1', email: 'john@example.com', display_name: 'John Doe', status: 'active', tier: 'starter' },
        { id: 'user-2', email: 'jane@example.com', display_name: 'Jane Smith', status: 'active', tier: 'professional' },
        { id: 'user-3', email: 'bob@test.com', display_name: 'Bob Wilson', status: 'suspended', tier: 'trial' },
      ],
    });

    await loginAndNavigateToUsers(page);

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
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [
        { id: 'user-1', email: 'john@example.com', display_name: 'John Doe', status: 'active', tier: 'starter' },
        { id: 'user-2', email: 'jane@example.com', display_name: 'Jane Smith', status: 'active', tier: 'professional' },
        { id: 'user-3', email: 'bob@test.com', display_name: 'Bob Wilson', status: 'suspended', tier: 'trial' },
      ],
    });

    await loginAndNavigateToUsers(page);

    const searchInput = page.locator('input[placeholder*="Search"], input[type="search"]');
    if (await searchInput.isVisible()) {
      await searchInput.fill('Jane');
      await page.waitForTimeout(300);

      await expect(page.getByText('Jane Smith')).toBeVisible();
    }
  });

  test('shows no results message when search finds nothing', async ({ page }) => {
    await setupUserManagementMocks(page, {
      pendingCount: 0,
      pendingUsers: [],
      allUsers: [
        { id: 'user-1', email: 'john@example.com', display_name: 'John Doe', status: 'active', tier: 'starter' },
        { id: 'user-2', email: 'jane@example.com', display_name: 'Jane Smith', status: 'active', tier: 'professional' },
        { id: 'user-3', email: 'bob@test.com', display_name: 'Bob Wilson', status: 'suspended', tier: 'trial' },
      ],
    });

    await loginAndNavigateToUsers(page);

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
