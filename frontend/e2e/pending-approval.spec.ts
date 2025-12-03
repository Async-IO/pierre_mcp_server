// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for the pending approval page.
// ABOUTME: Tests display of pending status, user info, and logout functionality.

import { test, expect } from '@playwright/test';

// Helper to mock a pending user login that results in pending approval page
async function loginAsPendingUser(page: import('@playwright/test').Page) {
  // Mock setup status
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        needs_setup: false,
        admin_user_exists: true,
      }),
    });
  });

  // Mock login to return a pending user
  await page.route('**/api/auth/login', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        csrf_token: 'test-csrf-token',
        jwt_token: 'test-jwt-token',
        user: {
          id: 'pending-user-123',
          email: 'pending@test.com',
          display_name: 'Pending User',
          role: 'user',
          user_status: 'pending',
          tier: 'starter',
          created_at: new Date().toISOString(),
          last_active: new Date().toISOString(),
        },
      }),
    });
  });

  // Mock logout endpoint
  await page.route('**/api/auth/logout', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true }),
    });
  });

  await page.goto('/');
  await page.waitForSelector('form', { timeout: 10000 });

  // Login
  await page.locator('input[name="email"]').fill('pending@test.com');
  await page.locator('input[name="password"]').fill('TestPassword123');
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait for pending approval page to appear
  await page.waitForSelector('text=Account Pending Approval', { timeout: 5000 });
}

test.describe('Pending Approval Page - Display', () => {
  test('renders pending approval page with correct elements', async ({ page }) => {
    await loginAsPendingUser(page);

    // Check for main heading
    await expect(page.locator('h1')).toContainText('Account Pending Approval');

    // Check for explanation text
    await expect(page.getByText('Your account has been created successfully')).toBeVisible();
    await expect(page.getByText('awaiting approval by an administrator')).toBeVisible();

    // Check for status badge - use exact match to target the badge specifically
    await expect(page.getByText('Pending', { exact: true })).toBeVisible();

    // Check for sign out button
    await expect(page.getByRole('button', { name: 'Sign Out' })).toBeVisible();
  });

  test('displays user email on pending page', async ({ page }) => {
    await loginAsPendingUser(page);

    // Check that user email is displayed
    await expect(page.getByText('pending@test.com')).toBeVisible();
  });

  test('displays user display name on pending page', async ({ page }) => {
    await loginAsPendingUser(page);

    // Check that user display name is displayed
    await expect(page.getByText('Pending User')).toBeVisible();
  });

  test('shows what happens next section', async ({ page }) => {
    await loginAsPendingUser(page);

    // Check for "What happens next" section
    await expect(page.getByText('What happens next?')).toBeVisible();
    await expect(page.getByText('An administrator will review your registration')).toBeVisible();
    await expect(page.getByText("You'll receive an email when approved")).toBeVisible();
  });
});

test.describe('Pending Approval Page - Logout', () => {
  test('clicking sign out returns to login page', async ({ page }) => {
    await loginAsPendingUser(page);

    // Click sign out
    await page.getByRole('button', { name: 'Sign Out' }).click();

    // Should return to login page
    await expect(page.locator('h1')).toContainText('Pierre Fitness Platform', { timeout: 5000 });
    await expect(page.locator('input[name="email"]')).toBeVisible();
  });
});

test.describe('Pending Approval Page - Status Badge', () => {
  test('shows pending badge with correct styling', async ({ page }) => {
    await loginAsPendingUser(page);

    // The Badge component should show "Pending" with warning variant
    const badge = page.locator('text=Pending').first();
    await expect(badge).toBeVisible();
  });
});

test.describe('Pending Approval Page - Without Display Name', () => {
  test('renders correctly when user has no display name', async ({ page }) => {
    // Mock setup status
    await page.route('**/admin/setup/status', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          needs_setup: false,
          admin_user_exists: true,
        }),
      });
    });

    // Mock login with user without display name
    await page.route('**/api/auth/login', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          csrf_token: 'test-csrf-token',
          jwt_token: 'test-jwt-token',
          user: {
            id: 'pending-user-456',
            email: 'noname@test.com',
            role: 'user',
            user_status: 'pending',
            tier: 'starter',
            created_at: new Date().toISOString(),
            last_active: new Date().toISOString(),
          },
        }),
      });
    });

    await page.route('**/api/auth/logout', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    await page.locator('input[name="email"]').fill('noname@test.com');
    await page.locator('input[name="password"]').fill('TestPassword123');
    await page.getByRole('button', { name: 'Sign in' }).click();

    await page.waitForSelector('text=Account Pending Approval', { timeout: 5000 });

    // Should still show email
    await expect(page.getByText('noname@test.com')).toBeVisible();

    // Page should render without display name field
    await expect(page.locator('h1')).toContainText('Account Pending Approval');
  });
});

test.describe('Pending Approval Page - Active User Redirect', () => {
  test('active user does not see pending approval page', async ({ page }) => {
    // Mock setup status
    await page.route('**/admin/setup/status', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          needs_setup: false,
          admin_user_exists: true,
        }),
      });
    });

    // Mock login with an active user
    await page.route('**/api/auth/login', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          csrf_token: 'test-csrf-token',
          jwt_token: 'test-jwt-token',
          user: {
            id: 'active-user-123',
            email: 'active@test.com',
            display_name: 'Active User',
            role: 'user',
            user_status: 'active',
            tier: 'starter',
            created_at: new Date().toISOString(),
            last_active: new Date().toISOString(),
          },
        }),
      });
    });

    // Mock dashboard overview for active user view
    await page.route('**/admin/dashboard/overview', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          total_requests_today: 100,
          total_requests_this_month: 1000,
          active_api_keys: 5,
          total_api_keys: 10,
          error_rate_today: 1.5,
        }),
      });
    });

    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    await page.locator('input[name="email"]').fill('active@test.com');
    await page.locator('input[name="password"]').fill('TestPassword123');
    await page.getByRole('button', { name: 'Sign in' }).click();

    // Should NOT see pending approval page
    await expect(page.locator('text=Account Pending Approval')).not.toBeVisible({ timeout: 3000 });

    // Should be logged in (login form should not be visible)
    await expect(page.locator('input[name="email"]')).not.toBeVisible({ timeout: 5000 });
  });
});

test.describe('Pending Approval Page - Pierre Branding', () => {
  test('shows Pierre logo', async ({ page }) => {
    await loginAsPendingUser(page);

    // Check for SVG logo (Pierre holistic node logo)
    const logo = page.locator('svg').first();
    await expect(logo).toBeVisible();
  });

  test('shows clock icon indicating waiting status', async ({ page }) => {
    await loginAsPendingUser(page);

    // The clock icon should be visible
    const clockIcon = page.locator('svg[viewBox="0 0 24 24"]').first();
    await expect(clockIcon).toBeVisible();
  });
});
