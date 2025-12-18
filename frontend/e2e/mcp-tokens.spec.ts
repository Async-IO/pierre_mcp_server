// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for MCP Tokens tab functionality.
// ABOUTME: Tests token creation, listing, revocation, and setup instructions display.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Mock tokens data
const mockTokens = [
  {
    id: 'token-1',
    name: 'Claude Desktop',
    token_prefix: 'pmcp_abc123',
    expires_at: '2025-12-31T23:59:59Z',
    last_used_at: '2025-12-03T10:30:00Z',
    usage_count: 42,
    is_revoked: false,
    created_at: '2025-01-15T09:00:00Z',
  },
  {
    id: 'token-2',
    name: 'Cursor IDE',
    token_prefix: 'pmcp_xyz789',
    expires_at: null,
    last_used_at: null,
    usage_count: 0,
    is_revoked: false,
    created_at: '2025-11-01T14:30:00Z',
  },
  {
    id: 'token-3',
    name: 'Revoked Token',
    token_prefix: 'pmcp_revoked',
    expires_at: '2025-06-01T00:00:00Z',
    last_used_at: '2025-05-15T08:00:00Z',
    usage_count: 100,
    is_revoked: true,
    created_at: '2025-02-01T12:00:00Z',
  },
];

// Helper to set up MCP tokens mocks
async function setupMcpTokenMocks(page: Page, options: { tokens?: typeof mockTokens } = {}) {
  const { tokens = mockTokens } = options;

  // Set up base dashboard mocks for regular user
  await setupDashboardMocks(page, { role: 'user' });

  // Mock list tokens endpoint
  await page.route('**/api/user/mcp-tokens', async (route, request) => {
    if (request.method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ tokens }),
      });
    } else if (request.method() === 'POST') {
      // Handle create token
      const body = request.postDataJSON();
      const newToken = {
        id: 'new-token-123',
        name: body.name,
        token_prefix: 'pmcp_new123',
        token_value: 'pmcp_new123456789abcdef',
        expires_at: body.expires_in_days
          ? new Date(Date.now() + body.expires_in_days * 24 * 60 * 60 * 1000).toISOString()
          : null,
        created_at: new Date().toISOString(),
      };
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify(newToken),
      });
    } else {
      await route.fallback();
    }
  });

  // Mock revoke token endpoint
  await page.route('**/api/user/mcp-tokens/*', async (route, request) => {
    if (request.method() === 'DELETE') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    } else {
      await route.fallback();
    }
  });
}

async function loginAndGoToMcpTokens(page: Page) {
  await loginToDashboard(page);
  await page.waitForSelector('nav', { timeout: 10000 });
  await navigateToTab(page, 'MCP Tokens');
  // Wait for lazy-loaded MCP Tokens tab content to appear
  await page.waitForSelector('text=MCP Tokens', { timeout: 10000 });
  await page.waitForTimeout(500);
}

test.describe('MCP Tokens Tab Navigation', () => {
  test('MCP Tokens tab is visible for regular users', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // MCP Tokens tab should be visible in sidebar
    await expect(page.locator('nav button').filter({ hasText: 'MCP Tokens' })).toBeVisible();
  });

  test('navigates to MCP Tokens tab', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should see MCP Tokens header (CardHeader component)
    await expect(page.locator('h3:has-text("MCP Tokens")')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('MCP Tokens List', () => {
  test('displays empty state when no tokens exist', async ({ page }) => {
    await setupMcpTokenMocks(page, { tokens: [] });
    await loginAndGoToMcpTokens(page);

    // Should show empty state message
    await expect(page.getByText('No MCP tokens yet')).toBeVisible();
    await expect(page.getByText('Create a token to connect AI clients')).toBeVisible();
  });

  test('displays list of existing tokens', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show token names (using h3 heading selector to avoid matching setup instructions)
    await expect(page.locator('h3:has-text("Claude Desktop")')).toBeVisible();
    await expect(page.locator('h3:has-text("Cursor IDE")')).toBeVisible();
  });

  test('shows token prefix for identification', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show token prefixes
    await expect(page.getByText('pmcp_abc123...')).toBeVisible();
    await expect(page.getByText('pmcp_xyz789...')).toBeVisible();
  });

  test('shows active badge for active tokens', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show Active badges
    const activeBadges = page.locator('text=Active');
    await expect(activeBadges.first()).toBeVisible();
  });

  test('shows revoked badge for revoked tokens', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show Revoked badge (use exact match to avoid matching "Revoked Token" heading)
    await expect(page.getByText('Revoked', { exact: true })).toBeVisible();
  });

  test('displays usage count for tokens', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show usage counts (use exact match to avoid "100 requests" matching "0 requests")
    await expect(page.getByText('42 requests')).toBeVisible();
    await expect(page.getByText('0 requests', { exact: true })).toBeVisible();
  });

  test('shows expiration status', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show "Never" for non-expiring tokens
    await expect(page.getByText('Never').first()).toBeVisible();
  });
});

test.describe('MCP Token Creation', () => {
  test('shows Create New Token button', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await expect(page.getByRole('button', { name: 'Create New Token' })).toBeVisible();
  });

  test('opens create form when clicking Create New Token', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Create New Token' }).click();

    // Should show form elements
    await expect(page.getByText('Token Name')).toBeVisible();
    await expect(page.getByText('Expires In (days)')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create Token' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Cancel' })).toBeVisible();
  });

  test('can cancel token creation', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Create New Token' }).click();
    await expect(page.getByText('Token Name')).toBeVisible();

    await page.getByRole('button', { name: 'Cancel' }).click();

    // Form should be hidden
    await expect(page.getByText('Token Name')).not.toBeVisible();
  });

  test('creates token and shows token value', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Create New Token' }).click();

    // Fill in token name
    await page.getByPlaceholder('e.g., Claude Desktop').fill('Test Token');

    // Select expiration
    await page.locator('select').selectOption('30');

    // Click create
    await page.getByRole('button', { name: 'Create Token' }).click();

    // Should show success message with token value
    await expect(page.getByText('Token Created')).toBeVisible();
    await expect(page.getByText("Copy this token now")).toBeVisible();
    await expect(page.getByText('pmcp_new123456789abcdef')).toBeVisible();
  });

  test('shows copy button for new token', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Create New Token' }).click();
    await page.getByPlaceholder('e.g., Claude Desktop').fill('Test Token');
    await page.getByRole('button', { name: 'Create Token' }).click();

    // Should show copy button
    await expect(page.getByRole('button', { name: 'Copy' })).toBeVisible();
  });

  test('can dismiss token created message', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Create New Token' }).click();
    await page.getByPlaceholder('e.g., Claude Desktop').fill('Test Token');
    await page.getByRole('button', { name: 'Create Token' }).click();

    await expect(page.getByText('Token Created')).toBeVisible();

    // Dismiss the message
    await page.getByRole('button', { name: 'Dismiss' }).click();

    await expect(page.getByText('Token Created')).not.toBeVisible();
  });

  test('disables create button when name is empty', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Create New Token' }).click();

    // Create button should be disabled when name is empty
    const createButton = page.getByRole('button', { name: 'Create Token' });
    await expect(createButton).toBeDisabled();
  });
});

test.describe('MCP Token Revocation', () => {
  test('shows revoke button for active tokens', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Revoke button should be visible for active tokens
    const revokeButtons = page.getByRole('button', { name: 'Revoke' });
    await expect(revokeButtons.first()).toBeVisible();
  });

  test('hides revoke button for already revoked tokens', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Count revoke buttons - should be 2 (for Claude Desktop and Cursor IDE, not for Revoked Token)
    const revokeButtons = page.getByRole('button', { name: 'Revoke' });
    await expect(revokeButtons).toHaveCount(2);
  });

  test('shows confirmation dialog when clicking revoke', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Click first revoke button
    await page.getByRole('button', { name: 'Revoke' }).first().click();

    // Should show confirmation dialog (look for heading or specific dialog text)
    await expect(page.getByRole('heading', { name: 'Revoke Token' })).toBeVisible();
    await expect(page.getByText('Are you sure you want to revoke')).toBeVisible();
  });

  test('can cancel token revocation', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Revoke' }).first().click();
    await expect(page.getByText('Are you sure you want to revoke')).toBeVisible();

    // Cancel
    await page.getByRole('button', { name: 'Cancel' }).click();

    // Dialog should be closed
    await expect(page.getByText('Are you sure you want to revoke')).not.toBeVisible();
  });

  test('revokes token when confirmed', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    await page.getByRole('button', { name: 'Revoke' }).first().click();

    // Confirm revocation
    await page.getByRole('button', { name: 'Revoke Token' }).click();

    // Dialog should close (API call made)
    await page.waitForTimeout(500);
    await expect(page.getByText('Are you sure you want to revoke')).not.toBeVisible();
  });
});

test.describe('MCP Setup Instructions', () => {
  test('displays Claude Desktop setup instructions', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Click to expand setup instructions (collapsible section)
    await page.locator('button:has-text("Setup Instructions")').click();

    // Should show Claude Desktop instructions section
    await expect(page.locator('h4:has-text("Claude Desktop")')).toBeVisible();
    await expect(page.locator('pre:has-text("mcpServers")')).toBeVisible();
  });

  test('displays ChatGPT setup instructions', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Click to expand setup instructions (collapsible section)
    await page.locator('button:has-text("Setup Instructions")').click();

    // Should show ChatGPT instructions section
    await expect(page.locator('h4:has-text("ChatGPT")')).toBeVisible();
    await expect(page.locator('pre:has-text("Server URL")')).toBeVisible();
  });

  test('shows MCP endpoint in instructions', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Click to expand setup instructions (collapsible section)
    await page.locator('button:has-text("Setup Instructions")').click();

    // Instructions should contain the MCP server endpoint (origin + /mcp)
    await expect(page.locator('pre').filter({ hasText: '/mcp' }).first()).toBeVisible();
  });
});

test.describe('MCP Tokens Error Handling', () => {
  test('shows error state when API fails', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Mock failing API
    await page.route('**/api/user/mcp-tokens', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'MCP Tokens');
    await page.waitForTimeout(1000);

    // Should show error message - the component shows "Failed to load MCP tokens"
    await expect(page.getByText('Failed to load MCP tokens')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('MCP Tokens Active Count', () => {
  test('shows active token count in header', async ({ page }) => {
    await setupMcpTokenMocks(page);
    await loginAndGoToMcpTokens(page);

    // Should show "2 active tokens" (mockTokens has 2 non-revoked tokens)
    await expect(page.getByText('2 active tokens')).toBeVisible();
  });

  test('shows 0 active tokens when all revoked', async ({ page }) => {
    const allRevokedTokens = [
      { ...mockTokens[2], id: 'token-only', name: 'Only Token' },
    ];
    await setupMcpTokenMocks(page, { tokens: allRevokedTokens });
    await loginAndGoToMcpTokens(page);

    await expect(page.getByText('0 active tokens')).toBeVisible();
  });
});
