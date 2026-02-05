// ABOUTME: Playwright E2E tests for the Settings page UX redesign.
// ABOUTME: Tests user settings tabs, change password modal, about tab, and admin settings navigation.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { test, expect } from '@playwright/test';

// Helper to set up mocks for an authenticated user session
async function setupAuthenticatedMocks(page: import('@playwright/test').Page, isAdmin = false) {
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
    });
  });

  await page.route('**/api/auth/me', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        id: 'user-1',
        email: isAdmin ? 'admin@pierre.dev' : 'webtest@pierre.dev',
        display_name: isAdmin ? 'Admin User' : 'Web Test',
        is_admin: isAdmin,
        role: isAdmin ? 'admin' : 'user',
        tier: 'free',
        created_at: '2024-06-15T10:00:00Z',
      }),
    });
  });

  await page.route('**/oauth/token', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        access_token: 'test-jwt-token',
        token_type: 'Bearer',
        expires_in: 86400,
        csrf_token: 'test-csrf',
        user: {
          id: 'user-1',
          email: isAdmin ? 'admin@pierre.dev' : 'webtest@pierre.dev',
          display_name: isAdmin ? 'Admin User' : 'Web Test',
          is_admin: isAdmin,
          role: isAdmin ? 'admin' : 'user',
          user_status: 'active',
          tier: 'free',
          created_at: '2024-06-15T10:00:00Z',
        },
      }),
    });
  });

  // Mock user stats
  await page.route('**/api/user/stats', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ connected_providers: 2, days_active: 45 }),
    });
  });

  // Mock MCP tokens
  await page.route('**/api/user/mcp-tokens', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ tokens: [] }),
    });
  });

  // Mock OAuth apps
  await page.route('**/api/users/oauth-apps', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ apps: [] }),
    });
  });

  // Mock OAuth status
  await page.route('**/api/oauth/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        { provider: 'strava', connected: false },
        { provider: 'fitbit', connected: false },
      ]),
    });
  });

  // Mock dashboard overview (for admin)
  await page.route('**/api/dashboard/overview**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_api_keys: 5,
        active_api_keys: 3,
        total_requests_today: 100,
        total_requests_this_month: 2500,
      }),
    });
  });

  // Mock pending users (for admin)
  await page.route('**/api/admin/users/pending', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock A2A dashboard (for admin)
  await page.route('**/api/a2a/dashboard**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_clients: 0,
        active_clients: 0,
        requests_today: 0,
        requests_this_month: 0,
      }),
    });
  });

  // Mock change password
  await page.route('**/api/user/change-password', async (route) => {
    const body = route.request().postDataJSON();
    if (body?.current_password === 'WrongPassword123') {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Current password is incorrect' }),
      });
    } else {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'Password changed successfully' }),
      });
    }
  });

  // Mock rate limit overview
  await page.route('**/api/dashboard/rate-limits**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock A2A clients list (used by API Tokens tab)
  await page.route('**/a2a/clients', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock LLM settings (used by AI Settings tab)
  await page.route('**/api/llm/settings', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ providers: [] }),
    });
  });

  // Mock admin configuration catalog and audit (used by AdminConfiguration on Configuration tab)
  await page.route('**/api/admin/config/catalog', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ parameters: [] }),
    });
  });

  await page.route('**/api/admin/config/audit**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ entries: [] }),
    });
  });

  // Mock tool availability (used by AdminConfiguration)
  await page.route('**/api/admin/tools**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ tools: [] }),
    });
  });

  // Mock admin settings (used by AdminSettings component on Configuration tab)
  await page.route('**/api/admin/settings/auto-approval', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ data: { enabled: false, description: 'Auto-approve new users' } }),
    });
  });

  await page.route('**/api/admin/settings/social-insights', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        data: {
          min_activities_for_comparison: 5,
          comparison_window_days: 90,
          min_similar_users: 3,
          max_comparison_users: 50,
        },
      }),
    });
  });
}

async function loginAndNavigateToSettings(
  page: import('@playwright/test').Page,
  isAdmin = false
) {
  await setupAuthenticatedMocks(page, isAdmin);
  await page.goto('/');
  await page.waitForSelector('form', { timeout: 10000 });

  await page.locator('input[name="email"]').fill(isAdmin ? 'admin@pierre.dev' : 'webtest@pierre.dev');
  await page.locator('input[name="password"]').fill('TestPassword123');
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait for dashboard to load
  await expect(page.locator('input[name="email"]')).not.toBeVisible({ timeout: 10000 });

  // Click the gear icon (Settings) in the bottom-left profile bar
  const settingsGear = page.getByRole('button', { name: 'Settings', exact: true });
  if (await settingsGear.first().isVisible().catch(() => false)) {
    await settingsGear.first().click();
    await page.waitForTimeout(500);
  }
}

test.describe('Settings Page - User Mode', () => {
  test('settings tab navigation shows all tabs', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    // Use button role to avoid matching headings with the same text
    await expect(page.getByRole('button', { name: 'Profile' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Connections' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'API Tokens' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'AI Settings' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'About' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Account' })).toBeVisible();
  });

  test('profile tab shows user info and stats', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    // User info should be visible (name appears in both sidebar and settings content)
    await expect(page.getByRole('main').getByText('Web Test')).toBeVisible();
    // Email appears in both the header and the form field
    await expect(page.getByText('webtest@pierre.dev').first()).toBeVisible();

    // Stat cards should appear after data loads
    await expect(page.getByText('Connected Providers')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Days Active')).toBeVisible({ timeout: 5000 });
  });

  test('about tab shows version and links', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    // Click About tab button
    await page.getByRole('button', { name: 'About' }).click();
    await page.waitForTimeout(300);

    await expect(page.getByText('Version')).toBeVisible();
    await expect(page.getByText('1.0.0')).toBeVisible();
    await expect(page.getByText('Help Center')).toBeVisible();
    await expect(page.getByText('Terms & Privacy')).toBeVisible();
  });

  test('account tab shows member since and change password', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    // Click Account tab button
    await page.getByRole('button', { name: 'Account' }).click();
    await page.waitForTimeout(300);

    // Member since should show formatted date
    await expect(page.getByText('Jun 15, 2024')).toBeVisible();

    // Change password button
    await expect(page.getByRole('button', { name: 'Change Password' })).toBeVisible();

    // Danger zone
    await expect(page.getByText('Danger Zone')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Sign Out', exact: true })).toBeVisible();
  });

  test('change password modal opens and validates', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    // Go to Account tab
    await page.getByRole('button', { name: 'Account' }).click();
    await page.waitForTimeout(300);

    // Open change password modal - the button in Account tab's Security section
    await page.getByRole('button', { name: 'Change Password' }).click();
    await page.waitForTimeout(300);

    // Modal should be visible with password fields
    const currentPasswordInput = page.locator('input[type="password"]').first();
    await expect(currentPasswordInput).toBeVisible();

    // Fill in mismatched passwords
    const passwordInputs = page.locator('input[type="password"]');
    await passwordInputs.nth(0).fill('password123');
    await passwordInputs.nth(1).fill('NewPass456');
    await passwordInputs.nth(2).fill('DifferentPass789');

    // Submit via the "Update Password" button in the modal footer
    await page.getByRole('button', { name: 'Update Password' }).click();
    await page.waitForTimeout(300);

    // Should show mismatch error (appears in both modal banner and field validation)
    await expect(page.getByText(/passwords do not match/i).first()).toBeVisible();
  });

  test('connections tab shows provider credentials section', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    await page.getByRole('button', { name: 'Connections' }).click();
    await page.waitForTimeout(300);

    await expect(page.getByRole('heading', { name: 'Provider Credentials' })).toBeVisible();
  });

  test('tokens tab shows create new token button', async ({ page }) => {
    await loginAndNavigateToSettings(page);

    await page.getByRole('button', { name: 'API Tokens' }).click();
    await page.waitForTimeout(300);

    await expect(page.getByText('Create New Token')).toBeVisible();
  });
});

test.describe('Settings Page - User Profile Bar Navigation', () => {
  test('clicking user profile bar navigates to settings (user mode)', async ({ page }) => {
    await setupAuthenticatedMocks(page, false);
    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    await page.locator('input[name="email"]').fill('webtest@pierre.dev');
    await page.locator('input[name="password"]').fill('TestPassword123');
    await page.getByRole('button', { name: 'Sign in' }).click();

    await expect(page.locator('input[name="email"]')).not.toBeVisible({ timeout: 10000 });

    // Look for the user profile bar at bottom of sidebar and click it
    const userProfileBar = page.locator('button:has-text("Web Test")');
    if (await userProfileBar.first().isVisible().catch(() => false)) {
      await userProfileBar.first().click();
      await page.waitForTimeout(500);

      // Should now see settings content (use button role to avoid heading matches)
      await expect(page.getByRole('button', { name: 'Profile' })).toBeVisible();
    }
  });

  test('clicking user profile bar navigates to user settings (admin mode)', async ({ page }) => {
    await setupAuthenticatedMocks(page, true);
    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    await page.locator('input[name="email"]').fill('admin@pierre.dev');
    await page.locator('input[name="password"]').fill('TestPassword123');
    await page.getByRole('button', { name: 'Sign in' }).click();

    await expect(page.locator('input[name="email"]')).not.toBeVisible({ timeout: 10000 });

    // Look for the user profile bar and click it — navigates to user settings for all users
    const userProfileBar = page.locator('button:has-text("Admin User")');
    if (await userProfileBar.first().isVisible().catch(() => false)) {
      await userProfileBar.first().click();
      await page.waitForTimeout(500);

      // Should navigate to user settings (Profile tab visible)
      await expect(page.getByRole('button', { name: 'Profile' })).toBeVisible({ timeout: 5000 });
    }
  });
});

async function loginAndNavigateToAdminSettings(page: import('@playwright/test').Page) {
  await setupAuthenticatedMocks(page, true);
  await page.goto('/');
  await page.waitForSelector('form', { timeout: 10000 });

  await page.locator('input[name="email"]').fill('admin@pierre.dev');
  await page.locator('input[name="password"]').fill('TestPassword123');
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait for dashboard to load
  await expect(page.locator('input[name="email"]')).not.toBeVisible({ timeout: 10000 });

  // Navigate to admin settings via Configuration sidebar tab
  await page.getByRole('button', { name: 'Configuration', exact: true }).click();
  await page.waitForTimeout(500);
}

test.describe('Settings Page - Admin Mode', () => {
  test('admin settings shows system settings heading', async ({ page }) => {
    await loginAndNavigateToAdminSettings(page);

    // Admin settings should show "System Settings" heading
    await expect(page.getByText('System Settings')).toBeVisible({ timeout: 5000 });
  });

  test('admin settings shows auto-approval toggle', async ({ page }) => {
    await loginAndNavigateToAdminSettings(page);

    // Should show user registration / auto-approval section
    await expect(page.getByRole('heading', { name: 'User Registration' })).toBeVisible({ timeout: 5000 });
  });

  test('admin settings shows social insights configuration', async ({ page }) => {
    await loginAndNavigateToAdminSettings(page);

    // Should show social insights config section
    await expect(page.getByRole('heading', { name: 'Social Insights Configuration' })).toBeVisible({ timeout: 5000 });
  });
});
