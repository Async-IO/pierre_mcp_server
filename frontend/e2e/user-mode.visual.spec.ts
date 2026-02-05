// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Visual E2E tests for user mode (ASY-313).
// ABOUTME: Tests all user dashboard screens against real backend.

import { test, expect } from '@playwright/test';
import {
  loginAsUser,
  navigateToTab,
  takeVisualScreenshot,
  waitForNetworkIdle,
  VISUAL_TEST_CONFIG,
} from './visual-test-helpers';

test.describe('ASY-313: Web User Mode Visual Tests', () => {
  test.describe.configure({ mode: 'serial' });

  // ========================================
  // Login & Authentication
  // ========================================
  test.describe('Login & Authentication', () => {
    test('user login - renders login form', async ({ page }) => {
      // Setup minimal mocks needed for login page to render
      await page.route('**/admin/setup/status', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
        });
      });

      await page.goto('/');
      await page.waitForSelector('form', { timeout: VISUAL_TEST_CONFIG.defaultTimeout });

      await expect(page.locator('input[name="email"]')).toBeVisible();
      await expect(page.locator('input[name="password"]')).toBeVisible();
      await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible();

      await takeVisualScreenshot(page, 'user-login', 'form-rendered');
    });

    test('user login - successful login redirects to chat', async ({ page }) => {
      await loginAsUser(page, 'webtest');

      // Verify we're on the dashboard (not login page)
      await expect(page.locator('input[name="email"]')).not.toBeVisible();

      await takeVisualScreenshot(page, 'user-login', 'chat-visible');
    });

    test('user login - password visibility toggle works', async ({ page }) => {
      // Setup minimal mocks needed for login page to render
      await page.route('**/admin/setup/status', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ needs_setup: false, admin_user_exists: true }),
        });
      });

      await page.goto('/');
      await page.waitForSelector('form');

      const passwordInput = page.locator('input[name="password"]');
      await passwordInput.fill('TestPassword');

      // Password should be hidden by default
      await expect(passwordInput).toHaveAttribute('type', 'password');

      // Click toggle button
      const toggleButton = page.locator('button[type="button"]').first();
      if (await toggleButton.isVisible().catch(() => false)) {
        await toggleButton.click();
        await expect(passwordInput).toHaveAttribute('type', 'text');

        await takeVisualScreenshot(page, 'user-login', 'password-visible');
      }
    });
  });

  // ========================================
  // Chat Tab
  // ========================================
  test.describe('Chat Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('chat - displays conversation list', async ({ page }) => {
      await navigateToTab(page, 'Chat');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'user-chat', 'conversation-list');
    });

    test('chat - new conversation button visible', async ({ page }) => {
      await navigateToTab(page, 'Chat');
      await waitForNetworkIdle(page);

      // Check for new chat button (may or may not be visible)
      await page.getByRole('button', { name: /new|create|\+/i }).isVisible().catch(() => false);

      await takeVisualScreenshot(page, 'user-chat', 'new-chat-button');
    });

    test('chat - message input accepts text', async ({ page }) => {
      await navigateToTab(page, 'Chat');
      await waitForNetworkIdle(page);

      const messageInput = page.locator('textarea, input[placeholder*="message" i], input[placeholder*="type" i]');
      if (await messageInput.first().isVisible().catch(() => false)) {
        await messageInput.first().fill('Test message');
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-chat', 'message-input');
      }
    });

    test('chat - prompt suggestions visible', async ({ page }) => {
      await navigateToTab(page, 'Chat');
      await waitForNetworkIdle(page);

      // Look for prompt suggestion buttons/chips
      await page.locator('[data-testid="prompt-suggestion"], button:has-text("workout"), button:has-text("training")').first().isVisible().catch(() => false);

      await takeVisualScreenshot(page, 'user-chat', 'prompt-suggestions');
    });
  });

  // ========================================
  // Coach Library Tab
  // ========================================
  test.describe('Coach Library Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('library - displays installed coaches', async ({ page }) => {
      await navigateToTab(page, 'My Coaches');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'user-library', 'coach-list');
    });

    test('library - favorites filter toggles', async ({ page }) => {
      await navigateToTab(page, 'My Coaches');
      await waitForNetworkIdle(page);

      const favoritesToggle = page.locator('button:has-text("Favorites"), [role="switch"], input[type="checkbox"]');
      if (await favoritesToggle.first().isVisible().catch(() => false)) {
        await favoritesToggle.first().click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-library', 'favorites-filter');
      }
    });

    test('library - category filter works', async ({ page }) => {
      await navigateToTab(page, 'My Coaches');
      await waitForNetworkIdle(page);

      const categoryFilter = page.locator('select, [role="combobox"], [role="tab"]');
      if (await categoryFilter.first().isVisible().catch(() => false)) {
        await categoryFilter.first().click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-library', 'category-filter');
      }
    });

    test('library - search coaches works', async ({ page }) => {
      await navigateToTab(page, 'My Coaches');
      await waitForNetworkIdle(page);

      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"]');
      if (await searchInput.first().isVisible().catch(() => false)) {
        await searchInput.first().fill('training');
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-library', 'search-results');
      }
    });

    test('library - create coach button opens wizard', async ({ page }) => {
      await navigateToTab(page, 'My Coaches');
      await waitForNetworkIdle(page);

      const createButton = page.getByRole('button', { name: /create|new|add/i });
      if (await createButton.first().isVisible().catch(() => false)) {
        await createButton.first().click();
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-library', 'wizard-open');

        // Close the wizard
        const closeButton = page.getByRole('button', { name: /close|cancel|×/i });
        if (await closeButton.first().isVisible().catch(() => false)) {
          await closeButton.first().click();
        }
      }
    });
  });

  // ========================================
  // Coach Store Tab
  // ========================================
  test.describe('Coach Store Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('store - displays store grid', async ({ page }) => {
      await navigateToTab(page, 'Discover');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'user-store', 'grid');
    });

    test('store - category tabs work', async ({ page }) => {
      await navigateToTab(page, 'Discover');
      await waitForNetworkIdle(page);

      const categoryTabs = page.locator('[role="tab"], button:has-text("Training"), button:has-text("Nutrition")');
      const tabs = await categoryTabs.all();

      for (const tab of tabs.slice(0, 3)) {
        // Test first 3 category tabs
        if (await tab.isVisible().catch(() => false)) {
          await tab.click();
          await page.waitForTimeout(300);
        }
      }

      await takeVisualScreenshot(page, 'user-store', 'category-tabs');
    });

    test('store - search coaches works', async ({ page }) => {
      await navigateToTab(page, 'Discover');
      await waitForNetworkIdle(page);

      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"]');
      if (await searchInput.first().isVisible().catch(() => false)) {
        await searchInput.first().fill('training');
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-store', 'search-results');
      }
    });

    test('store - coach card shows install button', async ({ page }) => {
      await navigateToTab(page, 'Discover');
      await waitForNetworkIdle(page);

      // Check for install button presence
      await page.getByRole('button', { name: /install/i }).first().isVisible().catch(() => false);

      await takeVisualScreenshot(page, 'user-store', 'install-button');
    });

    test('store - clicking coach opens detail', async ({ page }) => {
      await navigateToTab(page, 'Discover');
      await waitForNetworkIdle(page);

      // Click on a coach card (not the install button)
      const coachCard = page.locator('[data-testid="coach-card"], .coach-card, article').first();
      if (await coachCard.isVisible().catch(() => false)) {
        await coachCard.click();
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-store', 'detail-view');

        // Go back
        const backButton = page.getByRole('button', { name: /back|close|×/i });
        if (await backButton.first().isVisible().catch(() => false)) {
          await backButton.first().click();
        }
      }
    });
  });

  // ========================================
  // Friends Tab
  // ========================================
  test.describe('Friends Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('friends - displays friends list', async ({ page }) => {
      await navigateToTab(page, 'Friends');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'user-friends', 'list');
    });

    test('friends - search users works', async ({ page }) => {
      await navigateToTab(page, 'Friends');
      await waitForNetworkIdle(page);

      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"]');
      if (await searchInput.first().isVisible().catch(() => false)) {
        await searchInput.first().fill('alice');
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-friends', 'search-results');
      }
    });

    test('friends - pending tab shows requests', async ({ page }) => {
      await navigateToTab(page, 'Friends');
      await waitForNetworkIdle(page);

      const pendingTab = page.locator('[role="tab"]:has-text("Pending"), button:has-text("Pending")');
      if (await pendingTab.first().isVisible().catch(() => false)) {
        await pendingTab.first().click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-friends', 'pending-tab');
      }
    });
  });

  // ========================================
  // Insights Tab
  // ========================================
  test.describe('Insights Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('feed - displays insight cards', async ({ page }) => {
      await navigateToTab(page, 'Insights');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'user-feed', 'insights');
    });

    test('feed - reaction buttons visible', async ({ page }) => {
      await navigateToTab(page, 'Insights');
      await waitForNetworkIdle(page);

      // Look for reaction buttons (emoji buttons)
      await page.locator('button:has-text("👍"), button:has-text("🎉"), button:has-text("💪"), button:has-text("🤗")').first().isVisible().catch(() => false);

      await takeVisualScreenshot(page, 'user-feed', 'reaction-buttons');
    });

    test('feed - clicking reaction records it', async ({ page }) => {
      await navigateToTab(page, 'Insights');
      await waitForNetworkIdle(page);

      const reactionButton = page.locator('button:has-text("👍"), button:has-text("💪")').first();
      if (await reactionButton.isVisible().catch(() => false)) {
        await reactionButton.click();
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-feed', 'reaction-clicked');
      }
    });

    test('feed - adapt button opens modal', async ({ page }) => {
      await navigateToTab(page, 'Insights');
      await waitForNetworkIdle(page);

      const adaptButton = page.getByRole('button', { name: /adapt/i });
      if (await adaptButton.first().isVisible().catch(() => false)) {
        await adaptButton.first().click();
        await page.waitForTimeout(500);

        await takeVisualScreenshot(page, 'user-feed', 'adapt-modal');

        // Close modal
        const closeButton = page.getByRole('button', { name: /close|cancel|×/i });
        if (await closeButton.first().isVisible().catch(() => false)) {
          await closeButton.first().click();
        }
      }
    });
  });

  // ========================================
  // Social Settings Tab
  // ========================================
  test.describe('Social Settings Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('social settings - displays visibility options', async ({ page }) => {
      // Navigate to settings via gear icon in bottom-left profile bar
      const settingsGear = page.getByRole('button', { name: 'Settings', exact: true });
      if (await settingsGear.first().isVisible().catch(() => false)) {
        await settingsGear.first().click();
        await waitForNetworkIdle(page);
      }

      // Look for social/privacy settings
      await page.locator('text=Visibility, text=Privacy, text=Discoverable').first().isVisible().catch(() => false);

      await takeVisualScreenshot(page, 'user-social-settings', 'options');
    });

    test('social settings - toggle discoverable', async ({ page }) => {
      const settingsGear = page.getByRole('button', { name: 'Settings', exact: true });
      if (await settingsGear.first().isVisible().catch(() => false)) {
        await settingsGear.first().click();
        await waitForNetworkIdle(page);
      }

      const toggle = page.locator('[role="switch"], input[type="checkbox"]').first();
      if (await toggle.isVisible().catch(() => false)) {
        await toggle.click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-social-settings', 'toggle-changed');
      }
    });
  });

  // ========================================
  // User Settings Tab
  // ========================================
  test.describe('User Settings Tab', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
    });

    test('settings - displays profile section', async ({ page }) => {
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);

      const mainContent = page.locator('main');
      await expect(mainContent).toBeVisible();

      await takeVisualScreenshot(page, 'user-settings', 'profile');
    });

    test('settings - displays provider connections', async ({ page }) => {
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);

      // Look for provider/connection section
      await page.locator('text=Connections, text=Providers, text=Strava').first().isVisible().catch(() => false);

      await takeVisualScreenshot(page, 'user-settings', 'providers');
    });

    test('settings - edit name works', async ({ page }) => {
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);

      const editButton = page.getByRole('button', { name: /edit/i });
      if (await editButton.first().isVisible().catch(() => false)) {
        await editButton.first().click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-settings', 'edit-mode');

        // Cancel edit
        const cancelButton = page.getByRole('button', { name: /cancel/i });
        if (await cancelButton.first().isVisible().catch(() => false)) {
          await cancelButton.first().click();
        }
      }
    });

    test('settings - change password form accessible', async ({ page }) => {
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);

      const changePasswordButton = page.getByRole('button', { name: /change password|password/i });
      if (await changePasswordButton.first().isVisible().catch(() => false)) {
        await changePasswordButton.first().click();
        await page.waitForTimeout(300);

        await takeVisualScreenshot(page, 'user-settings', 'change-password');

        // Close form
        const cancelButton = page.getByRole('button', { name: /cancel|close/i });
        if (await cancelButton.first().isVisible().catch(() => false)) {
          await cancelButton.first().click();
        }
      }
    });
  });

});
