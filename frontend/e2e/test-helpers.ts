// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Shared test helper functions for Playwright E2E tests.
// ABOUTME: Provides reusable authentication mocks and login helpers.

import type { Page } from '@playwright/test';

interface UserOptions {
  role?: 'user' | 'admin' | 'super_admin';
  email?: string;
  displayName?: string;
  status?: 'active' | 'pending' | 'suspended';
}

/**
 * Sets up common API mocks for authenticated dashboard access.
 * This must be called BEFORE navigating to any page.
 */
export async function setupDashboardMocks(page: Page, userOptions: UserOptions = {}) {
  const {
    role = 'admin',
    email = 'admin@test.com',
    displayName = 'Test Admin',
    status = 'active',
  } = userOptions;

  // Mock setup status
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
    });
  });

  // Mock OAuth2 ROPC login endpoint
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
          user_id: 'user-123',
          email,
          display_name: displayName,
          role,
          is_admin: role === 'admin' || role === 'super_admin',
          user_status: status,
          tier: role === 'super_admin' ? 'enterprise' : 'professional',
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
        total_api_keys: 10,
        active_api_keys: 8,
        total_requests_today: 450,
        total_requests_this_month: 12500,
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
      body: JSON.stringify({
        total_clients: 5,
        active_clients: 3,
        requests_today: 100,
        requests_this_month: 3000,
      }),
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

  // Mock pending users
  await page.route('**/api/admin/pending-users', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ count: 0, users: [] }),
    });
  });

  // Mock admin users list
  await page.route('**/api/admin/users**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ users: [], total_count: 0 }),
    });
  });

  // Mock user stats endpoint (used by UserHome component for non-admin users)
  await page.route('**/api/user/stats', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        connected_providers: 1,
        activities_synced: 42,
        days_active: 7,
      }),
    });
  });

  // Mock OAuth status endpoint (used by Connections tab)
  // Note: Backend returns array directly, getOAuthStatus() wraps it in { providers: ... }
  await page.route('**/api/oauth/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock MCP tokens endpoint (used by MCPTokensTab for non-admin users)
  await page.route('**/api/user/mcp-tokens', async (route, request) => {
    if (request.method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ tokens: [] }),
      });
    } else {
      await route.fallback();
    }
  });

  // Mock chat conversations endpoint
  await page.route('**/api/chat/conversations**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ conversations: [], total: 0, limit: 50, offset: 0 }),
    });
  });

  // Mock user OAuth apps endpoint
  await page.route('**/api/users/oauth-apps', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ apps: [] }),
    });
  });

  // Mock A2A clients endpoint (used by A2AClientList in MCPTokensTab)
  await page.route('**/a2a/clients', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([]),
    });
  });

  // Mock A2A client individual endpoints
  await page.route('**/a2a/clients/*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({}),
    });
  });

  // Mock prompts suggestions endpoint (public API for chat prompts)
  await page.route('**/api/prompts/suggestions', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        categories: [
          {
            category_key: 'training',
            category_title: 'Training',
            category_icon: '🏃',
            pillar: 'activity',
            prompts: ['Am I ready for a hard workout today?', 'What should my training focus be this week?'],
          },
          {
            category_key: 'nutrition',
            category_title: 'Nutrition',
            category_icon: '🥗',
            pillar: 'nutrition',
            prompts: ['What should I eat before my long run?', 'How can I improve my recovery nutrition?'],
          },
          {
            category_key: 'recovery',
            category_title: 'Recovery',
            category_icon: '😴',
            pillar: 'recovery',
            prompts: ['Am I getting enough rest?', 'How is my sleep affecting my training?'],
          },
          {
            category_key: 'recipes',
            category_title: 'Recipes',
            category_icon: '🍳',
            pillar: 'nutrition',
            prompts: ['Give me a high-protein post-workout meal idea', 'What are some easy pre-race breakfast options?'],
          },
        ],
        welcome_prompt: 'List my last 20 activities with key insights about my training patterns.',
        metadata: {
          timestamp: new Date().toISOString(),
          api_version: '1.0',
        },
      }),
    });
  });
}

/**
 * Performs login through the login form.
 * Requires setupDashboardMocks() to be called first.
 */
export async function loginToDashboard(page: Page, credentials?: { email?: string; password?: string }) {
  const { email = 'admin@test.com', password = 'password123' } = credentials || {};

  await page.goto('/');
  await page.waitForSelector('form', { timeout: 10000 });
  await page.locator('input[name="email"]').fill(email);
  await page.locator('input[name="password"]').fill(password);
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait for dashboard to load
  await page.waitForSelector('text=Pierre', { timeout: 10000 });
  await page.waitForTimeout(300);
}

/**
 * Navigates to a specific dashboard tab by clicking the sidebar button.
 */
export async function navigateToTab(page: Page, tabName: string) {
  // Try multiple selectors in order of preference:
  // 1. Button with span containing tab name (some UI versions)
  // 2. Button with generic/div containing tab name (current UI)
  // 3. Button containing the text anywhere (handles badges like "2 Users")
  // 4. Button with title attribute (collapsed sidebar)

  const selectors = [
    page.locator('button').filter({ has: page.locator(`span:has-text("${tabName}")`) }),
    page.locator('button').filter({ has: page.locator(`div:has-text("${tabName}")`) }),
    page.locator(`button:has-text("${tabName}")`),
    page.locator(`button[title="${tabName}"]`),
  ];

  for (const selector of selectors) {
    const isVisible = await selector.first().isVisible().catch(() => false);
    if (isVisible) {
      await selector.first().click();
      await page.waitForTimeout(300);
      return;
    }
  }

  // If none of the selectors worked, try clicking by accessible name (handles "2 Users" case)
  const buttonByName = page.getByRole('button', { name: new RegExp(`.*${tabName}.*`, 'i') });
  await buttonByName.click();
  await page.waitForTimeout(300);
}

/**
 * Shorthand for setting up mocks and logging in as an admin.
 */
export async function setupAndLoginAsAdmin(page: Page) {
  await setupDashboardMocks(page, { role: 'admin' });
  await loginToDashboard(page);
}

/**
 * Shorthand for setting up mocks and logging in as a super admin.
 */
export async function setupAndLoginAsSuperAdmin(page: Page) {
  await setupDashboardMocks(page, { role: 'super_admin' });
  await loginToDashboard(page);
}

/**
 * Shorthand for setting up mocks and logging in as a regular user.
 */
export async function setupAndLoginAsUser(page: Page) {
  await setupDashboardMocks(page, { role: 'user' });
  await loginToDashboard(page);
}
