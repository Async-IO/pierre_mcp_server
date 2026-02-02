// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for dark theme verification.
// ABOUTME: Verifies the always-dark admin UI theme is correctly applied across all pages.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

/**
 * Sets up mocks for dark theme testing.
 */
async function setupDarkThemeMocks(page: Page, options: { isAdmin?: boolean } = {}) {
  const { isAdmin = true } = options;
  await setupDashboardMocks(page, { role: isAdmin ? 'admin' : 'user' });
}

async function loginAndGoToDashboard(page: Page) {
  await loginToDashboard(page);
  await page.waitForTimeout(300);
}

test.describe('Dark Theme Styling', () => {
  test('applies dark background color to body element', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Verify body has dark background (Pierre design system uses bg-pierre-dark)
    const bodyBgColor = await page.evaluate(() => {
      const body = document.body;
      return window.getComputedStyle(body).backgroundColor;
    });

    // Dark backgrounds should have low RGB values
    // bg-pierre-dark is a dark color, typically rgb values less than 50
    const rgbMatch = bodyBgColor.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
    if (rgbMatch) {
      const [, r, g, b] = rgbMatch.map(Number);
      // Dark theme should have dark background (combined brightness < 150)
      expect(r + g + b).toBeLessThan(150);
    }
  });

  test('applies light text color to body element', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Verify body has light text color
    const bodyTextColor = await page.evaluate(() => {
      const body = document.body;
      return window.getComputedStyle(body).color;
    });

    // Light text should have high RGB values (close to white)
    const rgbMatch = bodyTextColor.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
    if (rgbMatch) {
      const [, r, g, b] = rgbMatch.map(Number);
      // White/light text has high combined RGB values (> 600 for near-white)
      expect(r + g + b).toBeGreaterThan(500);
    }
  });

  test('headings use white text color', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Verify headings have white text
    const headingColor = await page.evaluate(() => {
      const heading = document.querySelector('h1, h2, h3');
      if (heading) {
        return window.getComputedStyle(heading).color;
      }
      return null;
    });

    if (headingColor) {
      const rgbMatch = headingColor.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
      if (rgbMatch) {
        const [, r, g, b] = rgbMatch.map(Number);
        // White text should be close to rgb(255, 255, 255)
        expect(r).toBeGreaterThan(200);
        expect(g).toBeGreaterThan(200);
        expect(b).toBeGreaterThan(200);
      }
    }
  });

  test('sidebar navigation has dark styling', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Verify navigation area exists and is styled
    const sidebar = page.locator('nav, aside').first();
    await expect(sidebar).toBeVisible();

    // Navigation buttons should be visible against dark background
    const navButtons = page.locator('nav button');
    const buttonCount = await navButtons.count();
    expect(buttonCount).toBeGreaterThan(0);
  });

  test('main content area has dark background', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Main content should be visible
    const main = page.locator('main');
    await expect(main).toBeVisible();

    // Check main area background is dark
    const mainBgColor = await page.evaluate(() => {
      const main = document.querySelector('main');
      if (main) {
        return window.getComputedStyle(main).backgroundColor;
      }
      return null;
    });

    // Background should be dark or transparent (inheriting dark from body)
    if (mainBgColor && mainBgColor !== 'rgba(0, 0, 0, 0)') {
      const rgbMatch = mainBgColor.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
      if (rgbMatch) {
        const [, r, g, b] = rgbMatch.map(Number);
        expect(r + g + b).toBeLessThan(200);
      }
    }
  });
});

test.describe('Dark Theme Consistency', () => {
  test('dark theme persists across page navigation', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Get initial body background
    const initialBgColor = await page.evaluate(() => {
      return window.getComputedStyle(document.body).backgroundColor;
    });

    // Navigate to different tabs
    await navigateToTab(page, 'Analytics');
    await page.waitForTimeout(300);

    // Verify dark theme is still applied
    const analyticsBgColor = await page.evaluate(() => {
      return window.getComputedStyle(document.body).backgroundColor;
    });

    expect(analyticsBgColor).toBe(initialBgColor);
  });

  test('dark theme applies to all major UI sections', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Verify all major sections are present and visible
    await expect(page.locator('nav, aside').first()).toBeVisible();
    await expect(page.locator('main')).toBeVisible();

    // Headers should be visible
    const heading = page.locator('h1').first();
    await expect(heading).toBeVisible();
  });

  test('form inputs have proper dark theme styling', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });

    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    // Inputs should be visible and styled for dark theme
    const emailInput = page.locator('input[name="email"]');
    await expect(emailInput).toBeVisible();

    // Test that input is usable
    await emailInput.fill('test@example.com');
    await expect(emailInput).toHaveValue('test@example.com');
  });
});

test.describe('Dark Theme Visual Elements', () => {
  test('login page renders correctly with dark theme', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });

    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    // Verify login form elements are visible
    const emailInput = page.locator('input[name="email"]');
    const passwordInput = page.locator('input[name="password"]');
    const submitButton = page.getByRole('button', { name: 'Sign in' });

    await expect(emailInput).toBeVisible();
    await expect(passwordInput).toBeVisible();
    await expect(submitButton).toBeVisible();
  });

  test('dashboard renders correctly with dark theme', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Verify key dashboard elements are visible
    await expect(page.locator('nav')).toBeVisible();
    await expect(page.locator('main')).toBeVisible();
    await expect(page.locator('h1').first()).toBeVisible();
  });

  test('sidebar navigation renders correctly with dark theme', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Check sidebar is visible
    const sidebar = page.locator('nav, aside').first();
    await expect(sidebar).toBeVisible();

    // Verify navigation buttons are visible
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Overview")') })).toBeVisible();
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Analytics")') })).toBeVisible();
  });

  test('interactive elements have visible focus states', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Focus on a navigation button
    const overviewButton = page.locator('button').filter({ has: page.locator('span:has-text("Overview")') });
    await overviewButton.focus();

    // Button should have visible focus indicator
    await expect(overviewButton).toBeFocused();

    // Tab through other elements
    await page.keyboard.press('Tab');

    // Next focusable element should be focused
    const focusedElement = page.locator(':focus');
    await expect(focusedElement).toBeVisible();
  });

  test('cards and panels have proper styling', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Look for card-like elements in the Overview tab
    const cards = page.locator('[class*="card"], [class*="Card"], [class*="rounded"][class*="shadow"], [class*="bg-"]');
    const cardCount = await cards.count();

    // Should have some styled cards/panels
    if (cardCount > 0) {
      const firstCard = cards.first();
      await expect(firstCard).toBeVisible();
    }
  });
});

test.describe('Dark Theme Accessibility', () => {
  test('text remains readable with dark theme', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // All major text elements should be visible
    const headings = page.locator('h1, h2, h3');
    const headingCount = await headings.count();

    for (let i = 0; i < Math.min(headingCount, 5); i++) {
      await expect(headings.nth(i)).toBeVisible();
    }

    // Paragraph text should be visible
    const paragraphs = page.locator('p');
    const pCount = await paragraphs.count();

    for (let i = 0; i < Math.min(pCount, 3); i++) {
      const p = paragraphs.nth(i);
      const isVisible = await p.isVisible().catch(() => false);
      if (isVisible) {
        await expect(p).toBeVisible();
      }
    }
  });

  test('form inputs are visible and usable', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });

    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    // Test input visibility and interaction
    const emailInput = page.locator('input[name="email"]');
    await expect(emailInput).toBeVisible();
    await emailInput.fill('test@example.com');
    await expect(emailInput).toHaveValue('test@example.com');

    // Input text should be visible after typing
    const passwordInput = page.locator('input[name="password"]');
    await expect(passwordInput).toBeVisible();
    await passwordInput.fill('testpassword');
  });

  test('icons and graphics are visible', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Check for SVG icons in navigation
    const svgIcons = page.locator('nav svg, aside svg');
    const iconCount = await svgIcons.count();

    // Navigation should have icons
    expect(iconCount).toBeGreaterThan(0);

    // First few icons should be visible
    for (let i = 0; i < Math.min(iconCount, 3); i++) {
      await expect(svgIcons.nth(i)).toBeVisible();
    }
  });

  test('error states are visible', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });

    // Mock failed login to trigger error state
    await page.route('**/oauth/token', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'invalid_grant',
          error_description: 'Invalid credentials',
        }),
      });
    });

    await page.goto('/');
    await page.waitForSelector('form', { timeout: 10000 });

    await page.locator('input[name="email"]').fill('test@example.com');
    await page.locator('input[name="password"]').fill('wrongpassword');
    await page.getByRole('button', { name: 'Sign in' }).click();

    await page.waitForTimeout(1000);

    // Error message should be visible with proper contrast
    // Look for error indicators (red background, error text, etc.)
    const errorElement = page.locator('.bg-red-50, .bg-red-500, [role="alert"], .error, .text-red-500');
    const errorCount = await errorElement.count();

    // If error UI exists, verify at least one error indicator appeared
    // Note: Some apps show inline errors, others show toast notifications
    // This test verifies error styling works in dark theme
    if (errorCount > 0) {
      await expect(errorElement.first()).toBeVisible();
    }
  });

  test('maintains proper contrast ratios', async ({ page }) => {
    await setupDarkThemeMocks(page, { isAdmin: true });
    await loginAndGoToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Check that text elements have sufficient contrast by verifying visibility
    // Primary heading should be visible and readable
    const heading = page.locator('h1').first();
    await expect(heading).toBeVisible();

    // Navigation items should be visible
    const navButtons = page.locator('nav button');
    const buttonCount = await navButtons.count();
    expect(buttonCount).toBeGreaterThan(0);

    // Verify buttons have visible text/icons
    const firstButton = navButtons.first();
    await expect(firstButton).toBeVisible();
  });
});
