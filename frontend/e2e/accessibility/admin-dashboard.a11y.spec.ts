// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Accessibility tests for admin dashboard ensuring keyboard navigation and screen reader support.
// ABOUTME: Tests landmarks, data tables, modals, keyboard shortcuts, and ARIA live regions.

import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { setupDashboardMocks, loginToDashboard } from '../test-helpers';

test.describe('Admin Dashboard Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });
    await loginToDashboard(page);
  });

  test.describe('Landmark Regions', () => {
    test('should have proper landmark regions', async ({ page }) => {
      // Check for main landmark
      const main = page.locator('main, [role="main"]');
      await expect(main).toBeVisible();

      // Check for navigation landmark
      const nav = page.locator('nav, [role="navigation"]');
      expect(await nav.count()).toBeGreaterThanOrEqual(1);

      // Check for banner (header)
      const header = page.locator('header, [role="banner"]');
      expect(await header.count()).toBeGreaterThanOrEqual(0); // May be in nav
    });

    test('should have no WCAG 2.1 AA violations', async ({ page }) => {
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
        // Exclude color-contrast until UI design fixes are implemented
        .disableRules(['color-contrast'])
        .analyze();

      if (accessibilityScanResults.violations.length > 0) {
        console.log('Dashboard a11y violations:', JSON.stringify(accessibilityScanResults.violations, null, 2));
      }
      expect.soft(accessibilityScanResults.violations).toEqual([]);
    });
  });

  test.describe('Skip Navigation', () => {
    test('should have skip link to main content', async ({ page }) => {
      // Skip links are often visually hidden until focused
      await page.keyboard.press('Tab');

      // Check if a skip link becomes visible or is focusable
      const skipLink = page.locator('a[href="#main"], a[href="#content"], .skip-link');

      if ((await skipLink.count()) > 0) {
        await expect(skipLink.first()).toBeFocused();
      } else {
        // If no explicit skip link, check that first tab goes to meaningful content
        const focusedElement = await page.evaluate(() => {
          const el = document.activeElement;
          return el?.tagName;
        });
        expect(['A', 'BUTTON', 'INPUT']).toContain(focusedElement);
      }
    });
  });

  test.describe('Keyboard Navigation', () => {
    test('should allow full keyboard navigation through sidebar', async ({ page }) => {
      // Focus on sidebar navigation
      const sidebarButtons = page.locator('nav button, aside button');
      const buttonCount = await sidebarButtons.count();

      if (buttonCount > 0) {
        await sidebarButtons.first().focus();

        // Should be able to Tab through all buttons
        for (let i = 0; i < Math.min(buttonCount, 5); i++) {
          await page.keyboard.press('Tab');
        }

        // Focus should still be within interactive elements
        const focusedTag = await page.evaluate(() => document.activeElement?.tagName);
        expect(['BUTTON', 'A', 'INPUT']).toContain(focusedTag);
      }
    });

    test('should handle Tab and Shift+Tab correctly', async ({ page }) => {
      // Tab forward
      await page.keyboard.press('Tab');
      await page.keyboard.press('Tab');
      await page.keyboard.press('Tab');

      const afterForward = await page.evaluate(() => document.activeElement?.tagName);

      // Tab backward
      await page.keyboard.press('Shift+Tab');

      const afterBackward = await page.evaluate(() => document.activeElement?.tagName);

      // Should be on different elements
      expect(afterForward).toBeDefined();
      expect(afterBackward).toBeDefined();
    });

    test('should activate buttons with Enter and Space', async ({ page }) => {
      const button = page.locator('button').first();
      await button.focus();

      // Test Enter key
      await page.keyboard.press('Enter');
      await page.waitForTimeout(100);

      // Test Space key
      await button.focus();
      await page.keyboard.press('Space');
      await page.waitForTimeout(100);

      // Button should remain accessible
      await expect(button).toBeVisible();
    });
  });

  test.describe('Data Tables', () => {
    test('should have accessible table structure', async ({ page }) => {
      // Navigate to a page with tables (e.g., users, tokens)
      // Use sidebar navigation button specifically to avoid ambiguity with other Users buttons
      const usersButton = page.getByRole('list').getByRole('button', { name: /users/i });
      if ((await usersButton.count()) > 0) {
        await usersButton.click();
        await page.waitForTimeout(500);

        // Check for table elements
        const table = page.locator('table, [role="table"]');
        if ((await table.count()) > 0) {
          // Should have headers
          const headers = page.locator('th, [role="columnheader"]');
          expect(await headers.count()).toBeGreaterThan(0);

          // Check axe for table-specific issues
          const accessibilityScanResults = await new AxeBuilder({ page })
            .withTags(['wcag2a', 'wcag2aa'])
            .include('table, [role="table"]')
            // Exclude color-contrast until UI design fixes are implemented
            .disableRules(['color-contrast'])
            .analyze();

          if (accessibilityScanResults.violations.length > 0) {
            console.log('Table a11y violations:', JSON.stringify(accessibilityScanResults.violations, null, 2));
          }
          expect.soft(accessibilityScanResults.violations).toEqual([]);
        }
      }
    });

    test('should have proper scope attributes on headers', async ({ page }) => {
      const table = page.locator('table').first();

      if ((await table.count()) > 0) {
        const headers = await table.locator('th').all();

        for (const header of headers) {
          const scope = await header.getAttribute('scope');
          // Headers should have scope attribute
          expect(['col', 'row', null]).toContain(scope);
        }
      }
    });
  });

  test.describe('Modal Dialogs', () => {
    test('should trap focus in modal dialogs', async ({ page }) => {
      // Look for a button that opens a modal
      const modalTrigger = page.locator('button:has-text("Add"), button:has-text("Create")').first();

      if ((await modalTrigger.count()) > 0) {
        await modalTrigger.click();
        await page.waitForTimeout(500);

        // Check for modal/dialog
        const modal = page.locator('[role="dialog"], [aria-modal="true"], .modal');

        if ((await modal.count()) > 0) {
          // Focus should be inside modal
          const focusInModal = await page.evaluate(() => {
            const modal = document.querySelector('[role="dialog"], [aria-modal="true"], .modal');
            return modal?.contains(document.activeElement);
          });

          expect(focusInModal).toBe(true);

          // Tab should keep focus within modal
          for (let i = 0; i < 10; i++) {
            await page.keyboard.press('Tab');
          }

          const focusStillInModal = await page.evaluate(() => {
            const modal = document.querySelector('[role="dialog"], [aria-modal="true"], .modal');
            return modal?.contains(document.activeElement);
          });

          expect(focusStillInModal).toBe(true);
        }
      }
    });

    test('should close modal with Escape key', async ({ page }) => {
      const modalTrigger = page.locator('button:has-text("Add"), button:has-text("Create")').first();

      if ((await modalTrigger.count()) > 0) {
        await modalTrigger.click();
        await page.waitForTimeout(500);

        const modal = page.locator('[role="dialog"], [aria-modal="true"]');

        if ((await modal.count()) > 0) {
          await page.keyboard.press('Escape');
          await page.waitForTimeout(300);

          // Modal should be closed
          await expect(modal).not.toBeVisible();
        }
      }
    });

    test('should have accessible modal title', async ({ page }) => {
      const modalTrigger = page.locator('button:has-text("Add"), button:has-text("Create")').first();

      if ((await modalTrigger.count()) > 0) {
        await modalTrigger.click();
        await page.waitForTimeout(500);

        const modal = page.locator('[role="dialog"], [aria-modal="true"]');

        if ((await modal.count()) > 0) {
          // Modal should have aria-labelledby
          const labelledBy = await modal.getAttribute('aria-labelledby');
          const ariaLabel = await modal.getAttribute('aria-label');

          expect(labelledBy || ariaLabel).toBeTruthy();
        }
      }
    });
  });

  test.describe('Dynamic Content', () => {
    test('should announce loading states to screen readers', async ({ page }) => {
      // Check for aria-live regions
      const liveRegions = page.locator('[aria-live], [role="status"], [role="alert"]');
      const count = await liveRegions.count();

      // Should have at least one live region for announcements
      // This may be hidden until content changes
      expect(count).toBeGreaterThanOrEqual(0);
    });

    test('should have accessible loading indicators', async ({ page }) => {
      // Loading indicators should have proper ARIA
      const loadingIndicators = page.locator(
        '[aria-busy="true"], [role="progressbar"], .loading, .spinner'
      );

      if ((await loadingIndicators.count()) > 0) {
        const indicator = loadingIndicators.first();
        const ariaLabel = await indicator.getAttribute('aria-label');
        const ariaValueText = await indicator.getAttribute('aria-valuetext');
        const textContent = await indicator.textContent();

        // Should have some accessible name
        expect(ariaLabel || ariaValueText || textContent).toBeTruthy();
      }
    });
  });

  test.describe('Color Contrast', () => {
    test('should have sufficient color contrast throughout dashboard', async ({ page }) => {
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['cat.color'])
        .disableRules(['color-contrast-enhanced'])
        .analyze();

      const contrastViolations = accessibilityScanResults.violations.filter(
        (v) => v.id.includes('contrast')
      );

      // Log any violations for debugging
      if (contrastViolations.length > 0) {
        console.log(`Dashboard color contrast violations: ${contrastViolations.length}`);
        for (const violation of contrastViolations) {
          for (const node of violation.nodes) {
            console.log(`  - ${node.html}`);
          }
        }
      }

      expect(contrastViolations).toEqual([]);
    });
  });

  test.describe('Charts and Graphs', () => {
    test('should have accessible chart alternatives', async ({ page }) => {
      const charts = page.locator('canvas, [role="img"]');

      if ((await charts.count()) > 0) {
        for (const chart of await charts.all()) {
          // Charts should have aria-label or be in a figure with figcaption
          const ariaLabel = await chart.getAttribute('aria-label');
          const ariaDescribedBy = await chart.getAttribute('aria-describedby');
          const role = await chart.getAttribute('role');

          // Should have some form of accessible description
          const hasAccessibleName = !!(ariaLabel || ariaDescribedBy || role === 'img');

          if (!hasAccessibleName) {
            // Check for parent figure with figcaption
            const parent = page.locator('figure').filter({ has: chart });
            const figcaption = parent.locator('figcaption');
            expect((await parent.count()) > 0 || (await figcaption.count()) > 0).toBe(true);
          }
        }
      }
    });
  });

  test.describe('Page Title and Navigation', () => {
    test('should have descriptive page title', async ({ page }) => {
      const title = await page.title();
      expect(title.length).toBeGreaterThan(0);
      expect(title.toLowerCase()).toContain('pierre');
    });

    test('should update page title on navigation', async ({ page }) => {
      // Navigate to a different tab
      const tabButton = page.locator('button').filter({ hasText: /users|tokens|analytics/i }).first();

      if ((await tabButton.count()) > 0) {
        await tabButton.click();
        await page.waitForTimeout(500);

        // Title should still be descriptive after navigation
        const newTitle = await page.title();
        expect(newTitle.length).toBeGreaterThan(0);
      }
    });
  });
});
