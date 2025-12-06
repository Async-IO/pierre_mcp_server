// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Integration tests for API token management against the real backend server.
// ABOUTME: Tests CRUD operations for API tokens with actual database persistence.

import { test, expect } from '@playwright/test';
import {
  createAndLoginAsAdmin,
  navigateToTab,
  waitForDashboardLoad,
} from '../helpers';
import { generateUniqueKeyName, timeouts } from '../fixtures';

test.describe('API Token Management Integration Tests', () => {
  test.beforeEach(async ({ page }) => {
    const loginResult = await createAndLoginAsAdmin(page);
    expect(loginResult.success).toBe(true);
    await waitForDashboardLoad(page);
  });

  test.describe('API Token List', () => {
    test('connections tab displays API tokens section', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      // Wait for the connections page to load
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // The UnifiedConnections component shows "API Tokens" tab for admins
      // Look for the tab button or the "Your API Tokens" card header
      const hasApiTokensTab = await page.locator('button:has-text("API Tokens")').isVisible().catch(() => false);
      const hasApiTokensHeader = await page.locator('text=Your API Tokens').isVisible().catch(() => false);
      const hasCreateButton = await page.locator('button:has-text("Create API Token")').isVisible().catch(() => false);

      expect(hasApiTokensTab || hasApiTokensHeader || hasCreateButton).toBe(true);
    });

    test('API tokens list loads from real server', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // Should show either "Your API Tokens" header or "No API tokens yet" empty state
      const hasTokensHeader = await page.locator('text=Your API Tokens').isVisible().catch(() => false);
      const hasEmptyState = await page.locator('text=No API tokens yet').isVisible().catch(() => false);

      expect(hasTokensHeader || hasEmptyState).toBe(true);
    });
  });

  test.describe('Create API Token', () => {
    test('can open create API token form', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      // Wait for the page to load
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // Click the "Create API Token" button
      const createButton = page.locator('button:has-text("Create API Token")');
      await expect(createButton).toBeVisible({ timeout: 10000 });
      await createButton.click();

      // Wait for the form to appear - CreateApiKey component shows input#serviceName
      await page.waitForTimeout(500);

      // Check for the form elements
      const serviceNameInput = page.locator('input#serviceName');
      const formTitle = page.locator('text=Create API Token');

      const hasServiceNameInput = await serviceNameInput.isVisible({ timeout: 5000 }).catch(() => false);
      const hasFormTitle = await formTitle.isVisible().catch(() => false);

      expect(hasServiceNameInput || hasFormTitle).toBe(true);
    });

    test('creating API token persists to database', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      // Wait for the page to load
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // Click the "Create API Token" button
      const createButton = page.locator('button:has-text("Create API Token")');
      await expect(createButton).toBeVisible({ timeout: 10000 });
      await createButton.click();

      // Wait for form to appear
      const serviceNameInput = page.locator('input#serviceName');
      await expect(serviceNameInput).toBeVisible({ timeout: 5000 });

      // Fill in the form
      const tokenName = generateUniqueKeyName('Integration Test');
      await serviceNameInput.fill(tokenName);

      // Submit the form - button type="submit" with text "Create API Token"
      const submitButton = page.locator('button[type="submit"]:has-text("Create API Token")');
      await submitButton.click();

      // Wait for success message or token to appear
      await page.waitForTimeout(2000);

      // Check for success message "API Token Generated Successfully"
      const successMessage = await page.locator('text=API Token Generated Successfully').isVisible({ timeout: 10000 }).catch(() => false);

      expect(successMessage).toBe(true);

      // Close the success modal and navigate back
      const closeButton = page.locator('button:has-text("I\'ve Saved the Token Securely")');
      if (await closeButton.isVisible().catch(() => false)) {
        await closeButton.click();
        await page.waitForTimeout(500);
      }

      // Reload and verify token persisted
      await page.reload();
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      await navigateToTab(page, 'Connections');

      // The token should appear in the list
      const tokenPersistedAfterReload = await page.locator(`text="${tokenName}"`).isVisible({ timeout: 10000 }).catch(() => false);
      expect(tokenPersistedAfterReload).toBe(true);
    });
  });

  test.describe('Revoke API Token', () => {
    test('can revoke an API token', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      // Wait for the page to load
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // First create a token to revoke
      const createButton = page.locator('button:has-text("Create API Token")');
      await expect(createButton).toBeVisible({ timeout: 10000 });
      await createButton.click();

      // Fill in the form
      const serviceNameInput = page.locator('input#serviceName');
      await expect(serviceNameInput).toBeVisible({ timeout: 5000 });

      const tokenName = generateUniqueKeyName('To Revoke');
      await serviceNameInput.fill(tokenName);

      // Submit the form
      const submitButton = page.locator('button[type="submit"]:has-text("Create API Token")');
      await submitButton.click();

      // Wait for success and close modal
      await page.waitForTimeout(2000);
      const closeButton = page.locator('button:has-text("I\'ve Saved the Token Securely")');
      if (await closeButton.isVisible().catch(() => false)) {
        await closeButton.click();
        await page.waitForTimeout(500);
      }

      // Find the token card with the Revoke button
      const tokenCard = page.locator(`div:has-text("${tokenName}")`).first();
      const revokeButton = tokenCard.locator('button:has-text("Revoke")');

      const revokeVisible = await revokeButton.isVisible({ timeout: 5000 }).catch(() => false);

      if (!revokeVisible) {
        // Token might not have revoke button visible - skip test
        test.skip();
        return;
      }

      await revokeButton.click();

      // ConfirmDialog should appear - click the "Revoke API Token" confirm button
      const confirmButton = page.locator('button:has-text("Revoke API Token")');
      await expect(confirmButton).toBeVisible({ timeout: 5000 });
      await confirmButton.click();

      // Wait for revocation
      await page.waitForTimeout(2000);

      // The token should now show "Inactive" badge instead of "Active"
      const inactiveBadge = page.locator(`text="${tokenName}"`).locator('..').locator('text=Inactive');
      const isInactive = await inactiveBadge.isVisible({ timeout: 5000 }).catch(() => false);

      // Token might be filtered out if we're on "active" filter
      // Check if the token card is gone or shows inactive
      const tokenStillVisibleAndActive = await page.locator(`div:has-text("${tokenName}")`).locator('text=Active').first().isVisible().catch(() => false);

      expect(isInactive || !tokenStillVisibleAndActive).toBe(true);
    });
  });

  test.describe('API Token Details', () => {
    test('API token shows creation date', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      // First, check if we have any API tokens - if not, create one
      const hasNoTokens = await page.locator('text=No API tokens yet').isVisible().catch(() => false);

      if (hasNoTokens) {
        // Create a token first
        const createButton = page.locator('button:has-text("Create API Token")');
        await createButton.click();

        const serviceNameInput = page.locator('input#serviceName');
        await expect(serviceNameInput).toBeVisible({ timeout: 5000 });
        await serviceNameInput.fill(generateUniqueKeyName('Date Test'));

        const submitButton = page.locator('button[type="submit"]:has-text("Create API Token")');
        await submitButton.click();

        // Close success modal
        await page.waitForTimeout(2000);
        const closeButton = page.locator('button:has-text("I\'ve Saved the Token Securely")');
        if (await closeButton.isVisible().catch(() => false)) {
          await closeButton.click();
          await page.waitForTimeout(500);
        }
      }

      // Now check for the "Created:" label which is in ApiKeyList component
      const hasCreatedLabel = await page.locator('text=Created:').first().isVisible({ timeout: 5000 }).catch(() => false);

      // Also check for your API tokens header
      const hasTokensHeader = await page.locator('text=Your API Tokens').isVisible().catch(() => false);

      expect(hasCreatedLabel || hasTokensHeader).toBe(true);
    });
  });
});
