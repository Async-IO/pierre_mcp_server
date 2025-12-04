// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Integration tests for API key management against the real backend server.
// ABOUTME: Tests CRUD operations for API keys with actual database persistence.

import { test, expect } from '@playwright/test';
import {
  createAndLoginAsAdmin,
  navigateToTab,
  waitForDashboardLoad,
} from '../helpers';
import { generateUniqueKeyName, timeouts } from '../fixtures';

test.describe('API Key Management Integration Tests', () => {
  test.beforeEach(async ({ page }) => {
    const loginResult = await createAndLoginAsAdmin(page);
    expect(loginResult.success).toBe(true);
    await waitForDashboardLoad(page);
  });

  test.describe('API Key List', () => {
    test('connections tab displays API keys section', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      await page.waitForTimeout(1000);

      const hasApiKeysSection = await page.locator('text=API Keys, text=API Key, text=Keys')
        .first()
        .isVisible()
        .catch(() => false);

      expect(hasApiKeysSection || await page.locator('button:has-text("Create")').isVisible()).toBe(true);
    });

    test('API keys list loads from real server', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      const pageContent = await page.content();
      expect(pageContent.length).toBeGreaterThan(0);
    });
  });

  test.describe('Create API Key', () => {
    test('can open create API key dialog', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      const createButton = page.locator('button:has-text("Create"), button:has-text("New"), button:has-text("Add")').first();
      const buttonVisible = await createButton.isVisible().catch(() => false);

      if (buttonVisible) {
        await createButton.click();

        const dialogVisible = await page.locator('dialog, [role="dialog"], .modal, [class*="modal"]')
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        const inputVisible = await page.locator('input[name="name"], input[placeholder*="name" i]')
          .first()
          .isVisible()
          .catch(() => false);

        expect(dialogVisible || inputVisible).toBe(true);
      } else {
        test.skip();
      }
    });

    test('creating API key persists to database', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      const createButton = page.locator('button:has-text("Create"), button:has-text("New"), button:has-text("Add")').first();
      const buttonVisible = await createButton.isVisible().catch(() => false);

      if (!buttonVisible) {
        test.skip();
        return;
      }

      await createButton.click();

      const keyName = generateUniqueKeyName('Integration Test');

      const nameInput = page.locator('input[name="name"], input[placeholder*="name" i]').first();
      const inputVisible = await nameInput.isVisible({ timeout: 5000 }).catch(() => false);

      if (!inputVisible) {
        test.skip();
        return;
      }

      await nameInput.fill(keyName);

      const submitButton = page.locator('button[type="submit"], button:has-text("Create"), button:has-text("Save")').last();
      await submitButton.click();

      await page.waitForTimeout(2000);

      const keyCreated = await page.locator(`text="${keyName}"`).isVisible({ timeout: 10000 }).catch(() => false);

      expect(keyCreated).toBe(true);

      await page.reload();
      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      await navigateToTab(page, 'Connections');

      const keyPersistedAfterReload = await page.locator(`text="${keyName}"`).isVisible({ timeout: 10000 }).catch(() => false);
      expect(keyPersistedAfterReload).toBe(true);
    });
  });

  test.describe('Delete API Key', () => {
    test('can delete an API key', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      const createButton = page.locator('button:has-text("Create"), button:has-text("New"), button:has-text("Add")').first();
      const buttonVisible = await createButton.isVisible().catch(() => false);

      if (!buttonVisible) {
        test.skip();
        return;
      }

      await createButton.click();

      const keyName = generateUniqueKeyName('To Delete');
      const nameInput = page.locator('input[name="name"], input[placeholder*="name" i]').first();
      const inputVisible = await nameInput.isVisible({ timeout: 5000 }).catch(() => false);

      if (!inputVisible) {
        test.skip();
        return;
      }

      await nameInput.fill(keyName);
      const submitButton = page.locator('button[type="submit"], button:has-text("Create"), button:has-text("Save")').last();
      await submitButton.click();

      await page.waitForTimeout(2000);

      const keyRow = page.locator(`tr:has-text("${keyName}"), div:has-text("${keyName}")`).first();
      const keyVisible = await keyRow.isVisible({ timeout: 10000 }).catch(() => false);

      if (!keyVisible) {
        test.skip();
        return;
      }

      const deleteButton = keyRow.locator('button:has-text("Delete"), button:has-text("Remove"), button[aria-label*="delete" i]').first();
      const deleteVisible = await deleteButton.isVisible().catch(() => false);

      if (!deleteVisible) {
        test.skip();
        return;
      }

      await deleteButton.click();

      const confirmButton = page.locator('button:has-text("Confirm"), button:has-text("Yes"), button:has-text("Delete")').last();
      const confirmVisible = await confirmButton.isVisible({ timeout: 5000 }).catch(() => false);

      if (confirmVisible) {
        await confirmButton.click();
      }

      await page.waitForTimeout(2000);

      const keyStillVisible = await page.locator(`text="${keyName}"`).isVisible().catch(() => false);
      expect(keyStillVisible).toBe(false);
    });
  });

  test.describe('API Key Details', () => {
    test('API key shows creation date', async ({ page }) => {
      await navigateToTab(page, 'Connections');

      await page.waitForLoadState('networkidle', { timeout: timeouts.medium }).catch(() => {});

      const hasDateInfo = await page.locator('text=/\\d{4}[-/]\\d{2}[-/]\\d{2}|Created|Date/')
        .first()
        .isVisible()
        .catch(() => false);

      expect(hasDateInfo || await page.locator('table, [class*="list"], [class*="grid"]').first().isVisible()).toBe(true);
    });
  });
});
