// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Authentication helper functions for integration tests.
// ABOUTME: Provides real login flows that interact with the actual backend server.

import type { Page } from '@playwright/test';
import { testUsers } from '../fixtures/test-data';
import { createTestAdminUser, type TestUser } from './db-setup';

export interface LoginResult {
  success: boolean;
  error?: string;
}

/**
 * Perform a real login through the login form.
 * This interacts with the actual backend server, not mocked endpoints.
 */
export async function loginWithCredentials(
  page: Page,
  email: string,
  password: string
): Promise<LoginResult> {
  try {
    console.log(`[Auth] Attempting login for: ${email}`);
    await page.goto('/');

    await page.waitForSelector('form', { timeout: 15000 });
    console.log('[Auth] Login form found');

    await page.locator('input[name="email"]').fill(email);
    await page.locator('input[name="password"]').fill(password);

    await page.getByRole('button', { name: 'Sign in' }).click();
    console.log('[Auth] Sign in button clicked');

    // Poll for login result - either error appears or login form disappears
    const maxWaitMs = 15000;
    const checkIntervalMs = 500;
    let elapsedMs = 0;

    while (elapsedMs < maxWaitMs) {
      await page.waitForTimeout(checkIntervalMs);
      elapsedMs += checkIntervalMs;

      // Check for error message (multiple possible selectors)
      // Note: Pierre uses pierre-red-* classes, not standard Tailwind red-* classes
      const errorSelectors = '[class*="pierre-red"], [class*="bg-pierre-red"], [role="alert"], .error-message';
      const errorElement = page.locator(errorSelectors).first();
      const hasError = await errorElement.isVisible().catch(() => false);

      if (hasError) {
        const errorText = await errorElement.textContent().catch(() => 'Login failed');
        console.log(`[Auth] Error detected after ${elapsedMs}ms: ${errorText}`);
        return { success: false, error: errorText || 'Login failed' };
      }

      // Check if login form disappeared (indicates successful redirect to dashboard)
      const loginFormVisible = await page.locator('input[name="email"]').isVisible().catch(() => false);

      if (!loginFormVisible) {
        console.log(`[Auth] Login form disappeared after ${elapsedMs}ms - login successful`);
        // Double-check we're not on an error page
        await page.waitForTimeout(500);
        const stillNoForm = !(await page.locator('input[name="email"]').isVisible().catch(() => false));
        if (stillNoForm) {
          return { success: true };
        }
      }
    }

    // Timeout - still on login page, check final state
    const finalFormVisible = await page.locator('input[name="email"]').isVisible().catch(() => true);
    if (finalFormVisible) {
      console.log('[Auth] Timeout - login form still visible after 15s');
      return { success: false, error: 'Login timeout - still on login page' };
    }

    console.log('[Auth] Login completed after timeout');
    return { success: true };
  } catch (error) {
    console.log(`[Auth] Login exception: ${error}`);
    return {
      success: false,
      error: `Login failed: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}

/**
 * Create a test admin user in the database and then log in.
 * This is the primary way to set up an authenticated session for tests.
 */
export async function createAndLoginAsAdmin(page: Page): Promise<LoginResult> {
  const user = testUsers.admin;
  console.log(`[Auth] createAndLoginAsAdmin starting for: ${user.email}`);

  const createResult = await createTestAdminUser(user);
  console.log(`[Auth] User creation result: ${JSON.stringify(createResult)}`);

  if (!createResult.success) {
    console.log(`[Auth] User creation failed: ${createResult.error}`);
    return { success: false, error: createResult.error };
  }

  console.log('[Auth] User created, proceeding with login...');
  return loginWithCredentials(page, user.email, user.password);
}

/**
 * Create a test super admin user and log in.
 */
export async function createAndLoginAsSuperAdmin(page: Page): Promise<LoginResult> {
  const user = testUsers.superAdmin;

  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    return { success: false, error: createResult.error };
  }

  return loginWithCredentials(page, user.email, user.password);
}

/**
 * Create a custom test user and log in.
 */
export async function createAndLoginTestUser(
  page: Page,
  user: TestUser
): Promise<LoginResult> {
  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    return { success: false, error: createResult.error };
  }

  return loginWithCredentials(page, user.email, user.password);
}

/**
 * Log out of the current session.
 */
export async function logout(page: Page): Promise<void> {
  // Try multiple selectors for the logout button (icon button with title or text button)
  const logoutSelectors = [
    'button[title="Sign out"]',
    'button:has-text("Sign out")',
    'button:has-text("Logout")',
  ];

  for (const selector of logoutSelectors) {
    const logoutButton = page.locator(selector).first();
    const isVisible = await logoutButton.isVisible().catch(() => false);

    if (isVisible) {
      await logoutButton.click();
      // Wait for login form to appear after logout
      await page.waitForSelector('input[name="email"]', { timeout: 15000 }).catch(() => {});
      return;
    }
  }

  // Fallback: clear localStorage and reload to trigger logout
  await page.evaluate(() => {
    localStorage.removeItem('user');
    localStorage.removeItem('jwt_token');
  });
  await page.goto('/');
  await page.waitForSelector('input[name="email"]', { timeout: 15000 }).catch(() => {});
}

/**
 * Check if the user is currently logged in.
 */
export async function isLoggedIn(page: Page): Promise<boolean> {
  try {
    const loginForm = page.locator('input[name="email"]');
    const isLoginVisible = await loginForm.isVisible().catch(() => true);
    return !isLoginVisible;
  } catch {
    return false;
  }
}

/**
 * Navigate to a specific dashboard tab.
 */
export async function navigateToTab(page: Page, tabName: string): Promise<void> {
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
      await page.waitForTimeout(500);
      return;
    }
  }

  const buttonByName = page.getByRole('button', { name: new RegExp(`.*${tabName}.*`, 'i') });
  await buttonByName.click();
  await page.waitForTimeout(500);
}

/**
 * Wait for the dashboard to fully load after login.
 */
export async function waitForDashboardLoad(page: Page): Promise<void> {
  await page.waitForSelector('text=Pierre', { timeout: 15000 });

  await page.waitForLoadState('networkidle', { timeout: 10000 }).catch(() => {
    // Network idle timeout is acceptable, page may have ongoing requests
  });

  await page.waitForTimeout(500);
}
