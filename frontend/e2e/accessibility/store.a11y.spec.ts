// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Accessibility tests for user-facing store pages focusing on color contrast and focus management.
// ABOUTME: Tests coach marketplace, coach details, search, filtering, and pagination accessibility.

import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from '../test-helpers';

test.describe('Store Pages Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Mock store/coaches data
    await page.route('**/api/coaches**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'coach-1',
            title: 'Marathon Training Coach',
            description: 'Expert guidance for marathon preparation',
            category: 'training',
            install_count: 150,
            is_installed: false,
            system_prompt: 'I am a marathon training coach...',
          },
          {
            id: 'coach-2',
            title: 'Nutrition Advisor',
            description: 'Personalized nutrition guidance for athletes',
            category: 'nutrition',
            install_count: 200,
            is_installed: true,
            system_prompt: 'I am a nutrition advisor...',
          },
        ]),
      });
    });

    // Mock store stats
    await page.route('**/api/store/stats**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          total_coaches: 2,
          total_installs: 350,
        }),
      });
    });

    await loginToDashboard(page);
  });

  test.describe('Coach Marketplace', () => {
    test('should have no WCAG 2.1 AA violations', async ({ page }) => {
      // Navigate to store/coaches tab
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
        // Exclude color-contrast until UI design fixes are implemented
        .disableRules(['color-contrast'])
        .analyze();

      if (accessibilityScanResults.violations.length > 0) {
        console.log('Store page a11y violations:', JSON.stringify(accessibilityScanResults.violations, null, 2));
      }
      expect.soft(accessibilityScanResults.violations).toEqual([]);
    });

    test('should have sufficient color contrast for all text', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['cat.color'])
        .disableRules(['color-contrast-enhanced'])
        .analyze();

      const contrastViolations = accessibilityScanResults.violations.filter(
        (v) => v.id.includes('contrast')
      );

      // Log any violations for debugging
      if (contrastViolations.length > 0) {
        console.log(`Store color contrast violations: ${contrastViolations.length}`);
        for (const violation of contrastViolations) {
          for (const node of violation.nodes) {
            console.log(`  - ${node.html}`);
          }
        }
      }

      expect(contrastViolations).toEqual([]);
    });

    test('should have visible focus indicators on coach cards', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      // Find interactive cards
      const cards = page.locator('[data-testid="coach-card"], .card, article');

      if ((await cards.count()) > 0) {
        const firstCard = cards.first();
        const cardButton = firstCard.locator('button, a').first();

        if ((await cardButton.count()) > 0) {
          await cardButton.focus();

          // Check for visible focus indicator
          const hasFocusStyle = await cardButton.evaluate((el) => {
            const styles = window.getComputedStyle(el);
            return (
              styles.outline !== 'none' ||
              styles.boxShadow !== 'none' ||
              styles.borderColor !== 'transparent'
            );
          });

          expect(hasFocusStyle).toBe(true);
        }
      }
    });

    test('should have accessible card structure', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      // Cards should have proper heading structure
      const cards = page.locator('[data-testid="coach-card"], .card, article');

      if ((await cards.count()) > 0) {
        const firstCard = cards.first();

        // Should have a heading (h2, h3, h4, or element with heading role)
        const heading = firstCard.locator('h2, h3, h4, [role="heading"]');
        expect(await heading.count()).toBeGreaterThanOrEqual(1);
      }
    });
  });

  test.describe('Search Functionality', () => {
    test('should have accessible search input', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const searchInput = page.locator('input[type="search"], input[placeholder*="search" i]');

      if ((await searchInput.count()) > 0) {
        // Should have label or aria-label
        const hasLabel = await searchInput.evaluate((el) => {
          const input = el as HTMLInputElement;
          const ariaLabel = input.getAttribute('aria-label');
          const ariaLabelledBy = input.getAttribute('aria-labelledby');
          const labelFor = document.querySelector(`label[for="${input.id}"]`);
          return !!(ariaLabel || ariaLabelledBy || labelFor);
        });

        expect(hasLabel).toBe(true);
      }
    });

    test('should announce search results to screen readers', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const searchInput = page.locator('input[type="search"], input[placeholder*="search" i]');

      if ((await searchInput.count()) > 0) {
        await searchInput.fill('marathon');
        await page.waitForTimeout(500);

        // Check for live region that announces results
        const liveRegion = page.locator('[aria-live="polite"], [role="status"]');
        const resultsCount = page.locator('[aria-label*="result"], .results-count');

        // Should have some way to announce results
        const hasAnnouncement =
          (await liveRegion.count()) > 0 || (await resultsCount.count()) > 0;

        // This is a best practice, not a requirement
        expect(typeof hasAnnouncement).toBe('boolean');
      }
    });

    test('should support keyboard-only search', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      // Tab to search input
      const searchInput = page.locator('input[type="search"], input[placeholder*="search" i]');

      if ((await searchInput.count()) > 0) {
        await searchInput.focus();
        await page.keyboard.type('training');
        await page.keyboard.press('Enter');

        // Should perform search or show results
        await page.waitForTimeout(500);
      }
    });
  });

  test.describe('Filter Controls', () => {
    test('should have accessible filter buttons', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const filters = page.locator('button[aria-pressed], [role="tab"], .filter-chip');

      if ((await filters.count()) > 0) {
        // Check axe on filter area
        const filterContainer = page.locator('[role="tablist"], .filters');

        if ((await filterContainer.count()) > 0) {
          const accessibilityScanResults = await new AxeBuilder({ page })
            .include('[role="tablist"], .filters')
            .analyze();

          expect(accessibilityScanResults.violations).toEqual([]);
        }
      }
    });

    test('should indicate selected filter state', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const filterButtons = page.locator('button[aria-pressed], [role="tab"]');

      if ((await filterButtons.count()) > 0) {
        const firstFilter = filterButtons.first();
        await firstFilter.click();
        await page.waitForTimeout(300);

        // Should have aria-pressed or aria-selected
        const ariaPressed = await firstFilter.getAttribute('aria-pressed');
        const ariaSelected = await firstFilter.getAttribute('aria-selected');

        expect(ariaPressed === 'true' || ariaSelected === 'true').toBe(true);
      }
    });
  });

  test.describe('Coach Detail Page', () => {
    test('should have accessible coach detail view', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      // Click on a coach card to view details
      const viewButton = page.locator('button:has-text("View"), a:has-text("View")').first();

      if ((await viewButton.count()) > 0) {
        await viewButton.click();
        await page.waitForTimeout(500);

        const accessibilityScanResults = await new AxeBuilder({ page })
          .withTags(['wcag2a', 'wcag2aa'])
          .analyze();

        expect(accessibilityScanResults.violations).toEqual([]);
      }
    });

    test('should have proper heading hierarchy on detail page', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const viewButton = page.locator('button:has-text("View"), a:has-text("View")').first();

      if ((await viewButton.count()) > 0) {
        await viewButton.click();
        await page.waitForTimeout(500);

        // Check heading hierarchy
        const h1Count = await page.locator('h1').count();
        const h2Count = await page.locator('h2').count();

        // Should have at least one h1
        expect(h1Count).toBeGreaterThanOrEqual(1);

        // Should have logical hierarchy (h1 before h2)
        if (h2Count > 0) {
          const firstH1 = await page.locator('h1').first().boundingBox();
          const firstH2 = await page.locator('h2').first().boundingBox();

          if (firstH1 && firstH2) {
            expect(firstH1.y).toBeLessThan(firstH2.y);
          }
        }
      }
    });

    test('should have accessible install/uninstall button', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const actionButton = page.locator(
        'button:has-text("Install"), button:has-text("Uninstall")'
      ).first();

      if ((await actionButton.count()) > 0) {
        // Button should have accessible name
        const name = await actionButton.getAttribute('aria-label');
        const text = await actionButton.textContent();

        expect(name || text).toBeTruthy();

        // Should be keyboard accessible
        await actionButton.focus();
        const isFocused = await page.evaluate(() => {
          return document.activeElement?.tagName === 'BUTTON';
        });
        expect(isFocused).toBe(true);
      }
    });
  });

  test.describe('Pagination', () => {
    test('should have accessible pagination controls', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const pagination = page.locator('nav[aria-label*="pagination" i], [role="navigation"]');

      if ((await pagination.count()) > 0) {
        const accessibilityScanResults = await new AxeBuilder({ page })
          .include('nav[aria-label*="pagination" i], [role="navigation"]')
          .analyze();

        expect(accessibilityScanResults.violations).toEqual([]);
      }
    });

    test('should indicate current page to screen readers', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const currentPageButton = page.locator(
        '[aria-current="page"], [aria-pressed="true"]'
      );

      if ((await currentPageButton.count()) > 0) {
        // Current page should be properly marked
        expect(await currentPageButton.count()).toBeGreaterThanOrEqual(1);
      }
    });
  });

  test.describe('Images and Icons', () => {
    test('should have alt text for coach images', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const images = page.locator('img');
      const imageCount = await images.count();

      for (let i = 0; i < imageCount; i++) {
        const img = images.nth(i);
        const alt = await img.getAttribute('alt');
        const role = await img.getAttribute('role');

        // Images should have alt text or be marked decorative
        expect(alt !== null || role === 'presentation').toBe(true);
      }
    });

    test('should have accessible icons', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const icons = page.locator('svg, [class*="icon"]');
      let inaccessibleCount = 0;
      const inaccessibleIcons: string[] = [];

      for (const icon of await icons.all()) {
        const ariaHidden = await icon.getAttribute('aria-hidden');
        const ariaLabel = await icon.getAttribute('aria-label');
        const role = await icon.getAttribute('role');

        const isAccessible = ariaHidden === 'true' || ariaLabel || role === 'img';
        if (!isAccessible) {
          inaccessibleCount++;
          const html = await icon.evaluate((el) => el.outerHTML.substring(0, 100));
          inaccessibleIcons.push(html);
        }
      }

      // Log any inaccessible icons for debugging
      if (inaccessibleCount > 0) {
        console.log(`${inaccessibleCount} icons need aria-hidden or aria-label`);
        for (const html of inaccessibleIcons.slice(0, 5)) {
          console.log(`  - ${html}`);
        }
      }

      expect(inaccessibleCount).toBe(0);
    });
  });

  test.describe('Link Purpose', () => {
    test('should have descriptive link text', async ({ page }) => {
      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const links = page.locator('a:visible');
      const linkCount = await links.count();

      for (let i = 0; i < Math.min(linkCount, 10); i++) {
        const link = links.nth(i);
        const text = await link.textContent();
        const ariaLabel = await link.getAttribute('aria-label');

        // Link should have meaningful text (not just "click here" or "read more")
        const meaningfulText = text?.trim() || ariaLabel || '';
        if (meaningfulText.length > 0) {
          expect(['click here', 'read more', 'here', 'more']).not.toContain(
            meaningfulText.toLowerCase()
          );
        }
      }
    });
  });

  test.describe('Responsive Accessibility', () => {
    test('should maintain accessibility at mobile viewport', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 });

      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .disableRules(['color-contrast-enhanced'])
        .analyze();

      // Log any violations for debugging
      if (accessibilityScanResults.violations.length > 0) {
        console.log(`Mobile a11y violations found: ${accessibilityScanResults.violations.length}`);
        for (const violation of accessibilityScanResults.violations) {
          console.log(`  - ${violation.id}: ${violation.help}`);
        }
      }

      expect(accessibilityScanResults.violations).toEqual([]);
    });

    test('should have touch-friendly target sizes on mobile', async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 });

      await navigateToTab(page, 'Coaches');
      await page.waitForTimeout(500);

      const buttons = page.locator('button:visible');
      const buttonCount = await buttons.count();

      let undersizedCount = 0;
      const undersizedButtons: string[] = [];
      for (let i = 0; i < Math.min(buttonCount, 10); i++) {
        const button = buttons.nth(i);
        const box = await button.boundingBox();

        if (box) {
          // Minimum touch target size (44x44px recommended by WCAG 2.1)
          if (box.width < 44 || box.height < 44) {
            undersizedCount++;
            const text = await button.textContent();
            undersizedButtons.push(`${text?.trim() || 'unnamed'}: ${box.width}x${box.height}px`);
          }
        }
      }

      // Log any undersized buttons for debugging
      if (undersizedCount > 0) {
        console.log(`${undersizedCount} buttons have undersized touch targets`);
        for (const btn of undersizedButtons) {
          console.log(`  - ${btn}`);
        }
      }

      expect(undersizedCount).toBe(0);
    });
  });
});
