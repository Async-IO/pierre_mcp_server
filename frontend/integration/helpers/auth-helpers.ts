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
    await page.goto('/');

    await page.waitForSelector('form', { timeout: 15000 });

    await page.locator('input[name="email"]').fill(email);
    await page.locator('input[name="password"]').fill(password);

    await page.getByRole('button', { name: 'Sign in' }).click();

    const loginSucceeded = await Promise.race([
      page.waitForSelector('input[name="email"]', { state: 'hidden', timeout: 15000 })
        .then(() => true)
        .catch(() => false),
      page.waitForSelector('.bg-red-50', { timeout: 5000 })
        .then(() => false)
        .catch(() => true),
    ]);

    if (!loginSucceeded) {
      const errorElement = page.locator('.bg-red-50');
      const errorText = await errorElement.textContent().catch(() => 'Unknown error');
      return { success: false, error: errorText || 'Login failed' };
    }

    await page.waitForTimeout(500);

    return { success: true };
  } catch (error) {
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

  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    return { success: false, error: createResult.error };
  }

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
  const logoutButton = page.locator('button:has-text("Logout")');
  const isVisible = await logoutButton.isVisible().catch(() => false);

  if (isVisible) {
    await logoutButton.click();
    await page.waitForSelector('input[name="email"]', { timeout: 10000 });
  } else {
    await page.goto('/');
  }
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
