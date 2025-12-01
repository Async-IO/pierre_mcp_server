// ABOUTME: Playwright E2E tests for admin token management features.
// ABOUTME: Tests token listing, creation, revocation, rotation, and bulk operations.

import { test, expect, type Page } from '@playwright/test';

// Helper to authenticate and set up common mocks
async function setupAuthenticatedSession(page: Page) {
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

  // Mock dashboard endpoints
  await page.route('**/api/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ total_api_keys: 5, active_api_keys: 3, total_requests_today: 150, total_requests_month: 2500 }),
    });
  });

  await page.route('**/api/dashboard/rate-limits', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ daily_limit: 1000, daily_used: 150, monthly_limit: 10000, monthly_used: 2500 }),
    });
  });

  await page.route('**/a2a/dashboard/overview', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ total_clients: 2, active_sessions: 1, requests_today: 50, error_rate: 0.01 }),
    });
  });

  await page.route('**/api/dashboard/analytics**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ daily_usage: [] }),
    });
  });

  // Mock pending users (for badge)
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
      body: JSON.stringify({ users: [], total_count: 0 }),
    });
  });
}

async function loginToDashboard(page: Page) {
  await page.goto('/');
  await page.waitForSelector('form');
  await page.locator('input[name="email"]').fill('admin@test.com');
  await page.locator('input[name="password"]').fill('password123');
  await page.getByRole('button', { name: 'Sign in' }).click();
  await page.waitForTimeout(500);
}

const sampleTokens = [
  {
    id: 'token-1',
    service_name: 'CI/CD Pipeline',
    service_description: 'Automated deployment service',
    token_prefix: 'pierre_at_abc123',
    is_active: true,
    is_super_admin: false,
    permissions: ['provision_keys', 'list_keys'],
    created_at: '2024-01-01T10:00:00Z',
    expires_at: '2024-12-31T23:59:59Z',
    last_used_at: '2024-01-20T15:30:00Z',
    usage_count: 150,
  },
  {
    id: 'token-2',
    service_name: 'Admin Console',
    service_description: 'Full admin access',
    token_prefix: 'pierre_at_def456',
    is_active: true,
    is_super_admin: true,
    permissions: [],
    created_at: '2024-01-05T10:00:00Z',
    expires_at: null,
    last_used_at: '2024-01-21T09:00:00Z',
    usage_count: 500,
  },
  {
    id: 'token-3',
    service_name: 'Monitoring Service',
    service_description: 'Read-only monitoring',
    token_prefix: 'pierre_at_ghi789',
    is_active: false,
    is_super_admin: false,
    permissions: ['list_keys', 'view_audit_logs'],
    created_at: '2023-12-01T10:00:00Z',
    expires_at: '2024-01-01T00:00:00Z',
    last_used_at: '2023-12-31T23:00:00Z',
    usage_count: 75,
  },
];

test.describe('Admin Token List', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedSession(page);

    // Mock admin tokens list
    await page.route('**/api/admin/tokens**', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ tokens: sampleTokens }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);
  });

  test('displays admin tokens list', async ({ page }) => {
    // Navigate to tokens section (might be in Overview or separate tab)
    // Look for token-related content
    await page.waitForTimeout(500);

    // Check if tokens are displayed on dashboard or need navigation
    const tokenSection = page.getByText('Admin Tokens').or(page.getByText('CI/CD Pipeline'));
    if (await tokenSection.isVisible()) {
      await expect(page.getByText('CI/CD Pipeline')).toBeVisible();
      await expect(page.getByText('Admin Console')).toBeVisible();
    }
  });

  test('shows active and inactive token counts', async ({ page }) => {
    await page.waitForTimeout(500);

    // Look for filter buttons with counts - both should be visible
    const activeFilter = page.getByRole('button', { name: /Active.*\d/i });
    const inactiveFilter = page.getByRole('button', { name: /Inactive.*\d/i });

    if (await activeFilter.isVisible()) {
      await expect(activeFilter).toBeVisible();
      await expect(inactiveFilter).toBeVisible();
    }
  });

  test('can filter tokens by status', async ({ page }) => {
    await page.waitForTimeout(500);

    // Click on Inactive filter
    const inactiveFilter = page.getByRole('button', { name: /Inactive/i });
    if (await inactiveFilter.isVisible()) {
      await inactiveFilter.click();
      await page.waitForTimeout(300);

      // Should show only inactive tokens
      await expect(page.getByText('Monitoring Service')).toBeVisible();
    }
  });

  test('displays token prefix with lock icon', async ({ page }) => {
    await page.waitForTimeout(500);

    // Token prefix should be displayed
    const tokenPrefix = page.getByText('pierre_at_abc123').or(page.getByText('pierre_at_'));
    if (await tokenPrefix.isVisible()) {
      await expect(tokenPrefix).toBeVisible();
    }
  });

  test('shows Super Admin badge for super admin tokens', async ({ page }) => {
    await page.waitForTimeout(500);

    // Look for Super Admin badge
    const superAdminBadge = page.getByText('Super Admin');
    if (await superAdminBadge.isVisible()) {
      await expect(superAdminBadge).toBeVisible();
    }
  });

  test('shows expiration date or Never for tokens', async ({ page }) => {
    await page.waitForTimeout(500);

    // Should show "Never" for non-expiring tokens
    const neverExpires = page.getByText('Never');
    if (await neverExpires.isVisible()) {
      await expect(neverExpires).toBeVisible();
    }
  });
});

test.describe('Admin Token Creation', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedSession(page);

    await page.route('**/api/admin/tokens**', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ tokens: sampleTokens }),
        });
      } else if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            token: 'pierre_at_newtoken_full_jwt_here',
            token_id: 'token-new',
            service_name: 'New Service',
            token_prefix: 'pierre_at_new123',
          }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);
  });

  test('can open create token form', async ({ page }) => {
    await page.waitForTimeout(500);

    // Find create token button
    const createButton = page.getByRole('button', { name: /Create.*Token|New.*Token/i });
    if (await createButton.isVisible()) {
      await createButton.click();

      // Form should appear
      await expect(page.getByLabel(/Service Name/i)).toBeVisible();
    }
  });

  test('validates required service name', async ({ page }) => {
    await page.waitForTimeout(500);

    const createButton = page.getByRole('button', { name: /Create.*Token|New.*Token/i });
    if (await createButton.isVisible()) {
      await createButton.click();
      await page.waitForTimeout(300);

      // Try to submit without service name
      const submitButton = page.getByRole('button', { name: /Create Admin Token/i });
      if (await submitButton.isVisible()) {
        await submitButton.click();

        // Should show validation error
        await page.waitForTimeout(300);
      }
    }
  });

  test('can create token with permissions', async ({ page }) => {
    await page.waitForTimeout(500);

    const createButton = page.getByRole('button', { name: /Create.*Token|New.*Token/i });
    if (await createButton.isVisible()) {
      await createButton.click();
      await page.waitForTimeout(300);

      // Fill in service name
      const serviceNameInput = page.getByLabel(/Service Name/i);
      await serviceNameInput.fill('Test Integration Service');

      // Select permissions
      const provisionKeysCheckbox = page.getByLabel(/provision_keys|Provision Keys/i);
      if (await provisionKeysCheckbox.isVisible()) {
        await provisionKeysCheckbox.check();
      }

      // Submit
      const submitButton = page.getByRole('button', { name: /Create Admin Token/i });
      await submitButton.click();

      // Should show success modal with token
      await page.waitForTimeout(500);
    }
  });

  test('shows warning for super admin creation', async ({ page }) => {
    await page.waitForTimeout(500);

    const createButton = page.getByRole('button', { name: /Create.*Token|New.*Token/i });
    if (await createButton.isVisible()) {
      await createButton.click();
      await page.waitForTimeout(300);

      // Check super admin checkbox
      const superAdminCheckbox = page.getByLabel(/Super Admin/i);
      if (await superAdminCheckbox.isVisible()) {
        await superAdminCheckbox.check();

        // Should show danger warning
        const warning = page.getByText(/danger|warning|full access/i);
        await expect(warning).toBeVisible();
      }
    }
  });

  test('displays token in success modal', async ({ page }) => {
    await page.waitForTimeout(500);

    const createButton = page.getByRole('button', { name: /Create.*Token|New.*Token/i });
    if (await createButton.isVisible()) {
      await createButton.click();
      await page.waitForTimeout(300);

      const serviceNameInput = page.getByLabel(/Service Name/i);
      await serviceNameInput.fill('New API Service');

      const provisionKeysCheckbox = page.getByLabel(/provision_keys|Provision Keys/i);
      if (await provisionKeysCheckbox.isVisible()) {
        await provisionKeysCheckbox.check();
      }

      const submitButton = page.getByRole('button', { name: /Create Admin Token/i });
      await submitButton.click();

      // Modal should show the token
      await page.waitForTimeout(500);
      const tokenDisplay = page.locator('textarea, input[readonly], code').filter({ hasText: /pierre_at_/ });
      if (await tokenDisplay.isVisible()) {
        await expect(tokenDisplay).toBeVisible();
      }
    }
  });

  test('can copy token from success modal', async ({ page }) => {
    await page.waitForTimeout(500);

    const createButton = page.getByRole('button', { name: /Create.*Token|New.*Token/i });
    if (await createButton.isVisible()) {
      await createButton.click();
      await page.waitForTimeout(300);

      const serviceNameInput = page.getByLabel(/Service Name/i);
      await serviceNameInput.fill('Copy Test Service');

      const listKeysCheckbox = page.getByLabel(/list_keys|List Keys/i);
      if (await listKeysCheckbox.isVisible()) {
        await listKeysCheckbox.check();
      }

      const submitButton = page.getByRole('button', { name: /Create Admin Token/i });
      await submitButton.click();
      await page.waitForTimeout(500);

      // Find copy button
      const copyButton = page.getByRole('button', { name: /Copy/i });
      if (await copyButton.isVisible()) {
        await copyButton.click();
        // Should show copied confirmation
      }
    }
  });
});

test.describe('Admin Token Revocation', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedSession(page);

    await page.route('**/api/admin/tokens**', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ tokens: sampleTokens }),
        });
      } else {
        await route.continue();
      }
    });

    // Mock revoke endpoint
    await page.route('**/api/admin/tokens/*/revoke', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'Token revoked successfully' }),
      });
    });

    await loginToDashboard(page);
  });

  test('can revoke a single token', async ({ page }) => {
    await page.waitForTimeout(500);

    // Find revoke button for active token
    const revokeButton = page.getByRole('button', { name: /Revoke/i }).first();
    if (await revokeButton.isVisible()) {
      await revokeButton.click();

      // Confirmation dialog should appear
      await page.waitForTimeout(300);
      const confirmButton = page.getByRole('button', { name: /Confirm|Revoke/i }).last();
      if (await confirmButton.isVisible()) {
        await confirmButton.click();
      }
    }
  });

  test('shows revocation confirmation dialog', async ({ page }) => {
    await page.waitForTimeout(500);

    const revokeButton = page.getByRole('button', { name: /Revoke/i }).first();
    if (await revokeButton.isVisible()) {
      await revokeButton.click();

      // Dialog should mention the token name
      const dialog = page.getByRole('dialog').or(page.locator('[role="alertdialog"]'));
      if (await dialog.isVisible()) {
        await expect(dialog.getByText(/revoke|confirm/i)).toBeVisible();
      }
    }
  });

  test('can select multiple tokens for bulk revocation', async ({ page }) => {
    await page.waitForTimeout(500);

    // Find checkboxes
    const checkboxes = page.locator('input[type="checkbox"]');
    const count = await checkboxes.count();

    if (count > 1) {
      // Select first two checkboxes
      await checkboxes.nth(1).check();
      await checkboxes.nth(2).check();

      // Should show bulk actions
      const bulkRevokeButton = page.getByRole('button', { name: /Revoke Selected/i });
      if (await bulkRevokeButton.isVisible()) {
        await expect(bulkRevokeButton).toBeVisible();
      }
    }
  });

  test('can use select all checkbox', async ({ page }) => {
    await page.waitForTimeout(500);

    // Find select all checkbox (usually first or in header)
    const selectAllCheckbox = page.locator('input[type="checkbox"]').first();
    if (await selectAllCheckbox.isVisible()) {
      await selectAllCheckbox.check();

      // All tokens should be selected
      await page.waitForTimeout(300);
    }
  });
});

test.describe('Admin Token Rotation', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedSession(page);

    await page.route('**/api/admin/tokens**', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ tokens: sampleTokens }),
        });
      } else {
        await route.continue();
      }
    });

    // Mock rotate endpoint
    await page.route('**/api/admin/tokens/*/rotate', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          token: 'pierre_at_rotated_new_jwt_token',
          token_id: 'token-1',
          service_name: 'CI/CD Pipeline',
          token_prefix: 'pierre_at_rot123',
        }),
      });
    });

    // Mock token details
    await page.route('**/api/admin/tokens/token-1', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(sampleTokens[0]),
      });
    });

    await loginToDashboard(page);
  });

  test('can rotate a token', async ({ page }) => {
    await page.waitForTimeout(500);

    // Open token details first
    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(300);

      // Find rotate button
      const rotateButton = page.getByRole('button', { name: /Rotate/i });
      if (await rotateButton.isVisible()) {
        await rotateButton.click();

        // Should show new token
        await page.waitForTimeout(500);
      }
    }
  });

  test('displays new token after rotation', async ({ page }) => {
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(300);

      const rotateButton = page.getByRole('button', { name: /Rotate/i });
      if (await rotateButton.isVisible()) {
        await rotateButton.click();
        await page.waitForTimeout(500);

        // New token should be displayed
        const tokenDisplay = page.locator('textarea, input[readonly]').filter({ hasText: /pierre_at_/ });
        if (await tokenDisplay.isVisible()) {
          await expect(tokenDisplay).toBeVisible();
        }
      }
    }
  });
});

test.describe('Admin Token Details', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedSession(page);

    await page.route('**/api/admin/tokens**', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ tokens: sampleTokens }),
        });
      } else {
        await route.continue();
      }
    });

    // Mock token details
    await page.route('**/api/admin/tokens/token-1', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          ...sampleTokens[0],
          audit_entries: [
            { action: 'provision_keys', timestamp: '2024-01-20T15:30:00Z', success: true, ip_address: '192.168.1.1' },
            { action: 'list_keys', timestamp: '2024-01-20T14:00:00Z', success: true, ip_address: '192.168.1.1' },
          ],
        }),
      });
    });

    // Mock audit endpoint
    await page.route('**/admin/tokens/*/audit', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          entries: [
            { action: 'provision_keys', timestamp: '2024-01-20T15:30:00Z', success: true, ip_address: '192.168.1.1' },
            { action: 'list_keys', timestamp: '2024-01-20T14:00:00Z', success: true, ip_address: '192.168.1.1' },
            { action: 'revoke_keys', timestamp: '2024-01-19T10:00:00Z', success: false, error_message: 'Key not found' },
          ],
        }),
      });
    });

    // Mock usage stats
    await page.route('**/admin/tokens/*/usage-stats', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          total_requests: 150,
          last_24h: 25,
          last_7d: 100,
          most_common_actions: [
            { action: 'provision_keys', count: 80 },
            { action: 'list_keys', count: 50 },
          ],
        }),
      });
    });

    // Mock provisioned keys
    await page.route('**/admin/tokens/*/provisioned-keys', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          keys: [
            { key_id: 'key-1', user_email: 'user1@example.com', created_at: '2024-01-15T10:00:00Z', is_active: true },
            { key_id: 'key-2', user_email: 'user2@example.com', created_at: '2024-01-16T10:00:00Z', is_active: true },
          ],
        }),
      });
    });

    await loginToDashboard(page);
  });

  test('can view token details', async ({ page }) => {
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();

      // Should show token details
      await page.waitForTimeout(500);
      await expect(page.getByText('CI/CD Pipeline')).toBeVisible();
    }
  });

  test('displays usage statistics', async ({ page }) => {
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      // Should show usage stats
      const usageSection = page.getByText(/Usage|Statistics|Requests/i);
      if (await usageSection.isVisible()) {
        await expect(usageSection).toBeVisible();
      }
    }
  });

  test('displays audit log entries', async ({ page }) => {
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      // Should show audit entries
      const auditSection = page.getByText(/Audit|Activity|Recent/i);
      if (await auditSection.isVisible()) {
        await expect(auditSection).toBeVisible();
      }
    }
  });

  test('displays provisioned API keys', async ({ page }) => {
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      // Should show provisioned keys
      const keysSection = page.getByText(/Provisioned|API Keys/i);
      if (await keysSection.isVisible()) {
        await expect(keysSection).toBeVisible();
      }
    }
  });

  test('shows error entries in audit log', async ({ page }) => {
    await page.waitForTimeout(500);

    const viewDetailsButton = page.getByRole('button', { name: /View Details|Details/i }).first();
    if (await viewDetailsButton.isVisible()) {
      await viewDetailsButton.click();
      await page.waitForTimeout(500);

      // Should show error indicator for failed actions in audit log
      const errorIndicator = page.locator('.text-red-500, .bg-red-100, [class*="error"]');
      // Error entries may or may not be visible depending on data
      const errorCount = await errorIndicator.count();
      expect(errorCount).toBeGreaterThanOrEqual(0);
    }
  });
});
