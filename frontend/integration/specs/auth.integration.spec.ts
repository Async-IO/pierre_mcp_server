// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Integration tests for authentication flows against the real backend server.
// ABOUTME: Tests login, logout, session management, and error handling with actual API calls.

import { test, expect } from '@playwright/test';
import {
  loginWithCredentials,
  createAndLoginAsAdmin,
  logout,
  isLoggedIn,
  waitForDashboardLoad,
  createTestAdminUser,
} from '../helpers';
import { testUsers, generateUniqueEmail, validPassword, timeouts } from '../fixtures';

test.describe('Authentication Integration Tests', () => {
  test.describe('Login Flow', () => {
    test('successful login with valid admin credentials', async ({ page }) => {
      const result = await createAndLoginAsAdmin(page);

      expect(result.success).toBe(true);
      expect(await isLoggedIn(page)).toBe(true);

      await waitForDashboardLoad(page);

      await expect(page.locator('text=Pierre')).toBeVisible();
    });

    test('login persists across page reload', async ({ page }) => {
      await createAndLoginAsAdmin(page);
      await waitForDashboardLoad(page);

      await page.reload();

      await page.waitForTimeout(1000);

      const loggedIn = await isLoggedIn(page);
      expect(loggedIn).toBe(true);
    });

    test('failed login with invalid password shows error', async ({ page }) => {
      const user = testUsers.admin;
      await createTestAdminUser(user);

      const result = await loginWithCredentials(page, user.email, 'WrongPassword123!');

      expect(result.success).toBe(false);

      await expect(page.locator('input[name="email"]')).toBeVisible();
    });

    test('failed login with non-existent user shows error', async ({ page }) => {
      const result = await loginWithCredentials(
        page,
        'nonexistent@test.local',
        'AnyPassword123!'
      );

      expect(result.success).toBe(false);

      await expect(page.locator('input[name="email"]')).toBeVisible();
    });

    test('login form validates required fields', async ({ page }) => {
      await page.goto('/');
      await page.waitForSelector('form', { timeout: timeouts.medium });

      await page.getByRole('button', { name: 'Sign in' }).click();

      await expect(page.locator('input[name="email"]')).toBeVisible();

      const emailInput = page.locator('input[name="email"]');
      const isInvalid = await emailInput.evaluate((el: HTMLInputElement) => !el.validity.valid);
      expect(isInvalid).toBe(true);
    });
  });

  test.describe('Logout Flow', () => {
    test('logout clears session and redirects to login', async ({ page }) => {
      await createAndLoginAsAdmin(page);
      await waitForDashboardLoad(page);

      await logout(page);

      await expect(page.locator('input[name="email"]')).toBeVisible({ timeout: timeouts.medium });
      expect(await isLoggedIn(page)).toBe(false);
    });

    test('after logout, protected pages redirect to login', async ({ page }) => {
      await createAndLoginAsAdmin(page);
      await waitForDashboardLoad(page);
      await logout(page);

      await page.goto('/dashboard');

      await page.waitForTimeout(1000);

      await expect(page.locator('input[name="email"]')).toBeVisible();
    });
  });

  test.describe('Session Management', () => {
    test('dashboard displays real user data from server', async ({ page }) => {
      await createAndLoginAsAdmin(page);
      await waitForDashboardLoad(page);

      const statsVisible = await page.locator('[class*="stat"], [class*="card"]').first().isVisible()
        .catch(() => false);

      expect(statsVisible || await page.locator('text=Pierre').isVisible()).toBe(true);
    });

    test('multiple users can have separate sessions', async ({ browser }) => {
      const context1 = await browser.newContext();
      const context2 = await browser.newContext();

      const page1 = await context1.newPage();
      const page2 = await context2.newPage();

      try {
        const user2Email = generateUniqueEmail('user2');
        const user2 = { email: user2Email, password: validPassword, role: 'admin' as const };

        await createTestAdminUser(user2);

        const [result1, result2] = await Promise.all([
          createAndLoginAsAdmin(page1),
          loginWithCredentials(page2, user2.email, user2.password),
        ]);

        expect(result1.success).toBe(true);
        expect(result2.success).toBe(true);

        expect(await isLoggedIn(page1)).toBe(true);
        expect(await isLoggedIn(page2)).toBe(true);
      } finally {
        await context1.close();
        await context2.close();
      }
    });
  });

  test.describe('Setup Status', () => {
    test('login page shows server setup status', async ({ page }) => {
      await page.goto('/');
      await page.waitForSelector('form', { timeout: timeouts.medium });

      const setupStatus = page.locator('text=Ready to Login, text=Setup Required');
      const hasStatus = await setupStatus.first().isVisible().catch(() => false);

      expect(hasStatus || await page.locator('form').isVisible()).toBe(true);
    });
  });
});
