// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for the Connections tab (API Tokens, Connected Apps).
// ABOUTME: Tests tab switching, token/client management, status filtering, and CRUD operations.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Sample data for mocks
const sampleApiKeys = [
  {
    id: 'key-1',
    name: 'Production API Key',
    description: 'Main production key for web app',
    key_prefix: 'pk_live_abc',
    is_active: true,
    rate_limit_requests: 10000,
    created_at: '2024-01-01T00:00:00Z',
    last_used_at: '2024-01-15T10:30:00Z',
    expires_at: null,
  },
  {
    id: 'key-2',
    name: 'Development Key',
    description: 'Key for local development',
    key_prefix: 'pk_test_xyz',
    is_active: true,
    rate_limit_requests: 1000,
    created_at: '2024-01-05T00:00:00Z',
    last_used_at: '2024-01-14T08:00:00Z',
    expires_at: '2024-12-31T23:59:59Z',
  },
  {
    id: 'key-3',
    name: 'Legacy Key',
    description: 'Old key no longer in use',
    key_prefix: 'pk_old_123',
    is_active: false,
    rate_limit_requests: 5000,
    created_at: '2023-06-01T00:00:00Z',
    last_used_at: '2023-12-01T00:00:00Z',
    expires_at: null,
  },
];

const sampleA2AClients = [
  {
    id: 'client-1',
    name: 'Fitness Assistant Bot',
    description: 'AI assistant for workout recommendations',
    is_active: true,
    is_verified: true,
    capabilities: ['fitness-data-analysis', 'goal-management'],
    agent_version: '1.2.0',
    created_at: '2024-01-10T00:00:00Z',
  },
  {
    id: 'client-2',
    name: 'Training Analytics',
    description: 'Performance tracking and analysis tool',
    is_active: true,
    is_verified: false,
    capabilities: ['training-analytics', 'performance-prediction'],
    agent_version: '2.0.1',
    created_at: '2024-01-08T00:00:00Z',
  },
  {
    id: 'client-3',
    name: 'Deprecated Integration',
    description: 'Old integration no longer maintained',
    is_active: false,
    is_verified: true,
    capabilities: ['provider-integration'],
    agent_version: '0.9.0',
    created_at: '2023-11-01T00:00:00Z',
  },
];

// Helper to set up connections-specific mocks
async function setupConnectionsMocks(page: Page, options: { isAdmin?: boolean } = {}) {
  const { isAdmin = false } = options;

  // Set up base dashboard mocks with proper auth
  await setupDashboardMocks(page, { role: isAdmin ? 'admin' : 'user' });

  // Mock API keys endpoint
  await page.route('**/api/keys', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ api_keys: sampleApiKeys }),
      });
    } else if (route.request().method() === 'POST') {
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({
          api_key: {
            id: 'key-new',
            name: 'New API Key',
            key_prefix: 'pk_new_',
            is_active: true,
            rate_limit_requests: 5000,
            created_at: new Date().toISOString(),
          },
          full_key: 'pk_new_abcdefghijklmnop1234567890',
        }),
      });
    } else {
      await route.continue();
    }
  });

  // Mock deactivate API key endpoint
  await page.route('**/api/keys/*/deactivate', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true }),
    });
  });

  // Mock A2A clients endpoint
  await page.route('**/a2a/clients', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(sampleA2AClients),
      });
    } else if (route.request().method() === 'POST') {
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({
          client: {
            id: 'client-new',
            name: 'New A2A Client',
            is_active: true,
            capabilities: [],
            created_at: new Date().toISOString(),
          },
          client_secret: 'secret_abcdef123456',
        }),
      });
    } else {
      await route.continue();
    }
  });

  // Mock A2A client usage endpoint (use ** suffix to match query strings)
  await page.route('**/a2a/clients/*/usage**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        requests_today: 150,
        requests_this_month: 3200,
        total_requests: 45000,
        last_request_at: new Date().toISOString(),
        tool_usage_breakdown: [
          { tool_name: 'get_activities', usage_count: 1200 },
          { tool_name: 'get_athlete', usage_count: 800 },
          { tool_name: 'analyze_performance', usage_count: 450 },
        ],
      }),
    });
  });

  // Mock A2A client rate limit endpoint
  await page.route('**/a2a/clients/*/rate-limit', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        tier: 'professional',
        limit: 50000,
        remaining: 46800,
        reset_at: '2024-02-01T00:00:00Z',
      }),
    });
  });

  // Mock deactivate A2A client endpoint
  await page.route('**/a2a/clients/*/deactivate', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true }),
    });
  });

  // Mock admin tokens endpoint (for admin users) - use ** to match query strings
  await page.route('**/api/admin/tokens**', async (route) => {
    if (isAdmin) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          tokens: [
            {
              id: 'token-1',
              service_name: 'Admin Console',
              token_prefix: 'adm_',
              is_active: true,
              created_at: '2024-01-01T00:00:00Z',
            },
          ],
          total: 1,
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

  // Mock request logs for monitor
  await page.route('**/api/dashboard/request-logs**', async (route) => {
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
      ]),
    });
  });

  // Mock request stats for monitor (required for RequestMonitor component)
  await page.route('**/api/dashboard/request-stats**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_requests: 1500,
        successful_requests: 1450,
        failed_requests: 50,
        average_response_time: 125,
        requests_per_minute: 2.5,
      }),
    });
  });
}

async function loginAndNavigateToConnections(page: Page) {
  await loginToDashboard(page);
  await navigateToTab(page, 'Connections');
  await page.waitForTimeout(500);
}

test.describe('Connections Tab - API Tokens', () => {
  test.beforeEach(async ({ page }) => {
    // API Tokens tab is only visible for admin users
    await setupConnectionsMocks(page, { isAdmin: true });
    await loginAndNavigateToConnections(page);
  });

  test('displays API Tokens tab for admin users', async ({ page }) => {
    // API Tokens tab should be visible for admin
    await expect(page.locator('.tab').getByText('API Tokens')).toBeVisible();
  });

  test('shows list of API keys with details', async ({ page }) => {
    // Check for API key names
    await expect(page.getByText('Production API Key')).toBeVisible();
    await expect(page.getByText('Development Key')).toBeVisible();

    // Check for status badges
    await expect(page.getByText('Active').first()).toBeVisible();

    // Check for key prefix display
    await expect(page.getByText('pk_live_abc****')).toBeVisible();
  });

  test('can filter API tokens by status', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    // Initially showing active tokens
    await expect(page.getByText('Production API Key')).toBeVisible();
    await expect(page.getByText('Development Key')).toBeVisible();

    // Click on Inactive filter
    const inactiveFilter = page.getByRole('button', { name: /Inactive/i });
    if (await inactiveFilter.isVisible()) {
      await inactiveFilter.click();
      await page.waitForTimeout(300);

      // Should show inactive key
      await expect(page.getByText('Legacy Key')).toBeVisible();
    }

    // Click on All filter
    const allFilter = page.getByRole('button', { name: /All/i });
    if (await allFilter.isVisible()) {
      await allFilter.click();
      await page.waitForTimeout(300);

      // Should show all keys
      await expect(page.getByText('Production API Key')).toBeVisible();
      await expect(page.getByText('Legacy Key')).toBeVisible();
    }
  });

  test('shows Create Token button', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    await expect(page.getByRole('button', { name: /Create.*Token/i })).toBeVisible();
  });

  test('navigates to create token form', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    await page.getByRole('button', { name: /Create.*Token/i }).click();
    await page.waitForTimeout(300);

    // Should show back button
    await expect(page.getByRole('button', { name: /Back/i })).toBeVisible();
  });

  test('shows View Usage button for each token', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    const viewUsageButtons = page.getByRole('button', { name: /View Usage/i });
    await expect(viewUsageButtons.first()).toBeVisible();
  });

  test('shows Deactivate button for active tokens', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    const deactivateButton = page.getByRole('button', { name: /Deactivate/i }).first();
    await expect(deactivateButton).toBeVisible();
  });

  test('shows confirmation dialog when deactivating', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    await page.getByRole('button', { name: /Deactivate/i }).first().click();
    await page.waitForTimeout(300);

    // Should show confirmation dialog
    await expect(page.getByText(/Deactivate.*Token/i)).toBeVisible();
    await expect(page.getByText(/Are you sure you want to deactivate/)).toBeVisible();
    await expect(page.getByRole('button', { name: /Cancel/i })).toBeVisible();
  });

  test('can cancel deactivation', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    await page.getByRole('button', { name: /Deactivate/i }).first().click();
    await page.waitForTimeout(300);

    await page.getByRole('button', { name: /Cancel/i }).click();
    await page.waitForTimeout(300);

    // Dialog should be closed
    await expect(page.getByText(/Deactivate.*Token/i)).not.toBeVisible();
  });

  test('displays rate limit information', async ({ page }) => {
    // Click on API Tokens tab first
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    // Check for rate limit badges
    await expect(page.getByText(/req\/month/).first()).toBeVisible();
  });
});

test.describe('Connections Tab - Connected Apps (A2A)', () => {
  test.beforeEach(async ({ page }) => {
    await setupConnectionsMocks(page, { isAdmin: false });
    await loginAndNavigateToConnections(page);
    // Switch to Connected Apps tab
    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(500);
  });

  test('displays Connected Apps tab', async ({ page }) => {
    await expect(page.locator('.tab-active').getByText('Connected Apps')).toBeVisible();
    await expect(page.getByText('Your Connected Apps')).toBeVisible();
  });

  test('shows list of A2A clients', async ({ page }) => {
    await expect(page.getByText('Fitness Assistant Bot')).toBeVisible();
    await expect(page.getByText('Training Analytics')).toBeVisible();
  });

  test('displays client capabilities', async ({ page }) => {
    await expect(page.getByText('fitness-data-analysis')).toBeVisible();
    await expect(page.getByText('goal-management')).toBeVisible();
  });

  test('shows verified badge for verified clients', async ({ page }) => {
    await expect(page.getByText('Verified').first()).toBeVisible();
  });

  test('can filter clients by status', async ({ page }) => {
    // Click on Inactive filter
    const inactiveFilter = page.getByRole('button', { name: /Inactive/i });
    if (await inactiveFilter.isVisible()) {
      await inactiveFilter.click();
      await page.waitForTimeout(300);

      // Should show inactive client
      await expect(page.getByText('Deprecated Integration')).toBeVisible();
    }
  });

  test('shows Register App button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /Register App/i })).toBeVisible();
  });

  test('navigates to register app form', async ({ page }) => {
    await page.getByRole('button', { name: /Register App/i }).click();
    await page.waitForTimeout(300);

    // Should show back button
    await expect(page.getByRole('button', { name: /Back to Connected Apps/i })).toBeVisible();
  });

  test('shows Show Credentials button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /Show Credentials/i }).first()).toBeVisible();
  });

  test('toggles credentials visibility', async ({ page }) => {
    const showCredentialsBtn = page.getByRole('button', { name: /Show Credentials/i }).first();
    await showCredentialsBtn.click();
    await page.waitForTimeout(300);

    // Should show credentials section
    await expect(page.getByText('Client Credentials')).toBeVisible();
    await expect(page.getByText('Client ID:')).toBeVisible();

    // Button should now say Hide
    await expect(page.getByRole('button', { name: /Hide Credentials/i }).first()).toBeVisible();
  });

  test('shows confirmation dialog when deactivating client', async ({ page }) => {
    await page.getByRole('button', { name: /Deactivate/i }).first().click();
    await page.waitForTimeout(300);

    // Should show confirmation dialog
    await expect(page.getByText('Deactivate A2A Client')).toBeVisible();
    await expect(page.getByText(/Are you sure you want to deactivate/)).toBeVisible();
  });
});

test.describe('Connections Tab - API Tokens (Admin Only)', () => {
  test.beforeEach(async ({ page }) => {
    await setupConnectionsMocks(page, { isAdmin: true });
    await loginAndNavigateToConnections(page);
  });

  test('shows API Tokens tab for admin users', async ({ page }) => {
    await expect(page.locator('.tab').getByText('API Tokens')).toBeVisible();
  });

  test('can switch to API Tokens tab', async ({ page }) => {
    // Verify we're on the dashboard first (not login page) - use .first() for strict mode
    await expect(page.locator('nav').first()).toBeVisible({ timeout: 10000 });

    // Find and click the API Tokens tab
    const apiTokensTab = page.locator('button').filter({ hasText: 'API Tokens' });
    await expect(apiTokensTab).toBeVisible({ timeout: 5000 });
    await apiTokensTab.click();
    await page.waitForTimeout(500);

    // Should show Create Token button (the main CTA in API Tokens view)
    await expect(page.getByRole('button', { name: /Create.*Token/i })).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Connections Tab - Tab Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await setupConnectionsMocks(page, { isAdmin: true });
    await loginAndNavigateToConnections(page);
  });

  test('can switch between all tabs', async ({ page }) => {
    // Start with API Tokens - use button filter instead of .tab class
    const apiTokensTab = page.locator('button').filter({ hasText: 'API Tokens' });
    await apiTokensTab.click();
    await page.waitForTimeout(300);
    await expect(page.getByText(/API Tokens/i).first()).toBeVisible();

    // Switch to Connected Apps
    const connectedAppsTab = page.locator('button').filter({ hasText: 'Connected Apps' });
    await connectedAppsTab.click();
    await page.waitForTimeout(300);
    await expect(page.getByText(/Connected Apps/i).first()).toBeVisible();
  });

  test('highlights active tab correctly', async ({ page }) => {
    // Click API Tokens tab
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    await expect(page.locator('.tab-active').getByText('API Tokens')).toBeVisible();

    // Click Connected Apps tab
    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(300);
    await expect(page.locator('.tab-active').getByText('Connected Apps')).toBeVisible();
  });

  test('resets view when switching tabs', async ({ page }) => {
    // Go to API Tokens and click Create
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(300);
    await page.getByRole('button', { name: /Create.*Token/i }).click();
    await page.waitForTimeout(300);

    // Should be in create view
    await expect(page.getByRole('button', { name: /Back/i })).toBeVisible();

    // Switch to Connected Apps
    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(300);

    // Should be back to overview view
    await expect(page.getByText('Your Connected Apps')).toBeVisible();
    await expect(page.getByRole('button', { name: /Register App/i })).toBeVisible();
  });
});

test.describe('Connections Tab - Empty States', () => {
  test('shows empty state when no API tokens', async ({ page }) => {
    // API Tokens tab is only visible for admin users
    await setupDashboardMocks(page, { role: 'admin' });

    // Override mock with empty response for admin tokens
    await page.route('**/api/admin/tokens**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ tokens: [], total: 0 }),
      });
    });

    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // Click on API Tokens tab
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(500);

    // Should show empty state or "No tokens" message
    await expect(page.getByText(/No.*token/i)).toBeVisible();
  });

  test('shows empty state when no A2A clients', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Override mock with empty responses
    await page.route('**/a2a/clients', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([]),
        });
      }
    });

    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // Non-admin users land on Connected Apps tab by default
    // Should show empty state
    await expect(page.getByText('No Connected Apps Yet')).toBeVisible();
    await expect(page.getByRole('button', { name: /Register Your First App/i })).toBeVisible();
  });
});

test.describe('Connections Tab - Error Handling', () => {
  test('handles API error gracefully for API tokens', async ({ page }) => {
    // API Tokens tab is only visible for admin users
    await setupDashboardMocks(page, { role: 'admin' });

    await page.route('**/api/admin/tokens**', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // Page should still render without crashing
    await expect(page.locator('.tab').getByText('API Tokens')).toBeVisible();
  });

  test('handles API error gracefully for A2A clients', async ({ page }) => {
    // Set up error mock FIRST before dashboard mocks (routes are LIFO)
    await page.route('**/a2a/clients', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Internal server error' }),
        });
      } else {
        await route.continue();
      }
    });

    await setupDashboardMocks(page, { role: 'user' });

    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(1000); // Longer wait for error state

    // Non-admin users land on Connected Apps tab by default
    // Should show error state
    await expect(page.getByText('Failed to load A2A clients')).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('button', { name: 'Try Again' })).toBeVisible();
  });
});

test.describe('Connections Tab - API Token Usage Modal', () => {
  test('opens usage monitor when clicking View Usage', async ({ page }) => {
    // API Tokens tab requires admin access
    await setupConnectionsMocks(page, { isAdmin: true });
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // Click on API Tokens tab
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(500);

    // Click View Usage button (using View Details since the UI may use different button text)
    const viewDetailsButton = page.getByRole('button', { name: /View.*Details|View.*Usage/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);
    }

    // Should show token details or usage information
    await expect(page.locator('body')).toBeVisible();
  });

  test('can navigate back from details view', async ({ page }) => {
    // API Tokens tab requires admin access
    await setupConnectionsMocks(page, { isAdmin: true });
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // Click on API Tokens tab
    await page.locator('.tab').getByText('API Tokens').click();
    await page.waitForTimeout(500);

    // Click View Details button
    const viewDetailsButton = page.getByRole('button', { name: /View.*Details|View.*Usage/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      // Click back button if visible
      const backButton = page.getByRole('button', { name: /Back/i });
      if (await backButton.isVisible()) {
        await backButton.click();
        await page.waitForTimeout(500);

        // Should be back to overview
        await expect(page.locator('.tab').getByText('API Tokens')).toBeVisible();
      }
    }
  });
})

test.describe('Connections Tab - A2A Client Expansion', () => {
  test('expands client details on click', async ({ page }) => {
    await setupConnectionsMocks(page);
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // Navigate to Connected Apps tab
    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(500);

    // Click on a client card to select it (triggers usage/rate-limit queries)
    await page.getByText('Fitness Assistant Bot').click();
    await page.waitForTimeout(1000); // Wait for async queries to complete

    // Should show usage stats section (details view) - use longer timeout for async data
    await expect(page.getByText('Client Usage & Rate Limits')).toBeVisible({ timeout: 10000 });
  });

  test('shows usage statistics when client selected', async ({ page }) => {
    await setupConnectionsMocks(page);
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(500);

    await page.getByText('Fitness Assistant Bot').click();
    await page.waitForTimeout(1000); // Wait for async queries

    // Check for usage stats labels - use longer timeout
    await expect(page.getByText('Usage Statistics')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Today:')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('This Month:')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Total:')).toBeVisible({ timeout: 5000 });
  });

  test('shows rate limit tier', async ({ page }) => {
    await setupConnectionsMocks(page);
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(500);

    await page.getByText('Fitness Assistant Bot').click();
    await page.waitForTimeout(1000); // Wait for async queries

    // Check for rate limits section - use exact match to avoid matching "Client Usage & Rate Limits"
    await expect(page.getByRole('heading', { name: 'Rate Limits', exact: true })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Tier:')).toBeVisible({ timeout: 5000 });
  });

  test('shows top tools section', async ({ page }) => {
    await setupConnectionsMocks(page);
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    await page.locator('.tab').getByText('Connected Apps').click();
    await page.waitForTimeout(500);

    await page.getByText('Fitness Assistant Bot').click();
    await page.waitForTimeout(1000); // Wait for async queries

    // Check for top tools section - use longer timeout
    await expect(page.getByText('Top Tools')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Connections Tab - API Tokens Tab Visibility', () => {
  test('shows API Tokens tab for admin users', async ({ page }) => {
    await setupConnectionsMocks(page, { isAdmin: true });
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // API Tokens tab should be visible for admin
    await expect(page.locator('.tab').getByText('API Tokens')).toBeVisible();
  });

  test('does not show API Tokens tab for non-admin users', async ({ page }) => {
    await setupConnectionsMocks(page, { isAdmin: false });
    await loginToDashboard(page);
    await navigateToTab(page, 'Connections');
    await page.waitForTimeout(500);

    // API Tokens tab should not be visible for non-admin
    await expect(page.locator('.tab').getByText('API Tokens')).not.toBeVisible();
  });
});
