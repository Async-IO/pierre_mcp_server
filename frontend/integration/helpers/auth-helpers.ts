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
    console.log(`[Login] Starting login for ${email}`);
    await page.goto('/');

    console.log('[Login] Waiting for login form...');
    await page.waitForSelector('form', { timeout: 15000 });

    await page.locator('input[name="email"]').fill(email);
    await page.locator('input[name="password"]').fill(password);

    console.log('[Login] Clicking sign in button...');
    await page.getByRole('button', { name: 'Sign in' }).click();

    // Wait for navigation after login attempt - either dashboard loads or error appears
    // Use a longer timeout for CI environments where server startup may be slower
    const loginTimeout = process.env.CI ? 30000 : 15000;

    // First, check if an error message appears quickly (within 5 seconds)
    console.log('[Login] Checking for error message...');
    const errorAppeared = await page.waitForSelector('.bg-red-50', { timeout: 5000 })
      .then(() => true)
      .catch(() => false);

    if (errorAppeared) {
      const errorElement = page.locator('.bg-red-50');
      const errorText = await errorElement.textContent().catch(() => 'Unknown error');
      console.log(`[Login] Error appeared: ${errorText}`);
      return { success: false, error: errorText || 'Login failed' };
    }

    // No error appeared, wait for the login form to disappear (indicating successful navigation)
    console.log('[Login] No error, waiting for form to disappear...');
    try {
      await page.waitForSelector('input[name="email"]', { state: 'hidden', timeout: loginTimeout });
      console.log('[Login] Form disappeared');
    } catch {
      // Login form still visible after timeout - login likely failed
      console.log('[Login] Form still visible after timeout');
      return { success: false, error: 'Login timed out - form still visible' };
    }

    // Additional verification: wait for dashboard content to appear
    // Use OR pattern since selectors have different syntax (CSS vs Playwright text=)
    console.log('[Login] Waiting for dashboard content...');
    try {
      const dashboardLocator = page.locator('nav')
        .or(page.locator('text=Pierre'))
        .or(page.locator('[class*="dashboard"]'));
      await dashboardLocator.first().waitFor({ state: 'visible', timeout: 10000 });
      console.log('[Login] Dashboard content visible');
    } catch {
      // Dashboard didn't load, but login form disappeared - ambiguous state
      console.log('[Login] Dashboard did not load');
      return { success: false, error: 'Login redirect occurred but dashboard did not load' };
    }

    await page.waitForTimeout(500);

    console.log('[Login] Success!');
    return { success: true };
  } catch (error) {
    const errorMsg = `Login failed: ${error instanceof Error ? error.message : String(error)}`;
    console.log(`[Login] Exception: ${errorMsg}`);
    return {
      success: false,
      error: errorMsg,
    };
  }
}

/**
 * Create a test admin user in the database and then log in.
 * This is the primary way to set up an authenticated session for tests.
 */
export async function createAndLoginAsAdmin(page: Page): Promise<LoginResult> {
  const user = testUsers.admin;
  console.log(`[Auth] Creating admin user: ${user.email}`);

  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    console.log(`[Auth] Failed to create admin user: ${createResult.error}`);
    return { success: false, error: createResult.error };
  }
  console.log(`[Auth] Admin user created, proceeding to login`);

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
 * Tries multiple selectors to handle different UI states (icon button vs text button).
 */
export async function logout(page: Page): Promise<void> {
  // Try multiple selectors: icon button with title, or text button
  const selectors = [
    page.locator('button[title="Sign out"]'),
    page.locator('button:has-text("Sign Out")'),
    page.locator('button:has-text("Logout")'),
  ];

  for (const selector of selectors) {
    const isVisible = await selector.first().isVisible().catch(() => false);
    if (isVisible) {
      await selector.first().click();
      await page.waitForSelector('input[name="email"]', { timeout: 10000 });
      return;
    }
  }

  // Fallback: navigate to root which will redirect to login if not authenticated
  await page.goto('/');
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
 * Tries multiple selector strategies to handle both expanded and collapsed sidebar states.
 */
export async function navigateToTab(page: Page, tabName: string): Promise<void> {
  // Wait for the sidebar to be present
  await page.waitForSelector('nav', { timeout: 10000 });

  const selectors = [
    // Button with span containing the text (expanded sidebar)
    page.locator('button').filter({ has: page.locator(`span:has-text("${tabName}")`) }),
    // Button with title attribute (collapsed sidebar)
    page.locator(`button[title="${tabName}"]`),
    // Button containing the text directly
    page.locator(`button:has-text("${tabName}")`),
    // Button with div containing text
    page.locator('button').filter({ has: page.locator(`div:has-text("${tabName}")`) }),
    // Sidebar button by role and accessible name
    page.getByRole('button', { name: new RegExp(`.*${tabName}.*`, 'i') }),
  ];

  for (const selector of selectors) {
    try {
      const count = await selector.count();
      if (count > 0) {
        const isVisible = await selector.first().isVisible().catch(() => false);
        if (isVisible) {
          await selector.first().click();
          await page.waitForTimeout(500);
          return;
        }
      }
    } catch {
      // Continue to next selector
    }
  }

  // Last resort: find by data-testid or aria-label if available
  const lastResort = page.locator(`[data-testid*="${tabName.toLowerCase()}"], [aria-label*="${tabName}" i]`);
  const lastResortCount = await lastResort.count().catch(() => 0);
  if (lastResortCount > 0) {
    await lastResort.first().click();
    await page.waitForTimeout(500);
    return;
  }

  throw new Error(`Could not find tab button for "${tabName}". Available buttons: check screenshot for details.`);
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
