// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: E2E tests to prevent UI hallucinations and hardcoded fake data
// ABOUTME: Tests verify that unimplemented features do NOT appear in the UI

import { test, expect } from '@playwright/test';
import {
  loginAsUser,
  navigateToTab,
  waitForNetworkIdle,
} from './visual-test-helpers';

test.describe('Anti-Hallucination Tests - User Mode', () => {
  test.describe.configure({ mode: 'serial' });

  // ========================================
  // Settings Screen - Hallucinated Elements Must NOT Exist
  // ========================================
  test.describe('Settings - No Hallucinated Elements', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);
    });

    test('should NOT show Apple Health (not implemented for web)', async ({ page }) => {
      const appleHealth = page.getByText('Apple Health', { exact: false });
      await expect(appleHealth).not.toBeVisible();
    });

    test('should NOT show Export Data (not implemented)', async ({ page }) => {
      const exportData = page.getByText('Export Data', { exact: false });
      await expect(exportData).not.toBeVisible();
    });

    test('should NOT show Push Notifications (not implemented - ASY-355)', async ({ page }) => {
      const pushNotifications = page.getByText('Push Notifications', { exact: false });
      await expect(pushNotifications).not.toBeVisible();
    });

    test('should NOT show Email Updates (not implemented - ASY-356)', async ({ page }) => {
      const emailUpdates = page.getByText('Email Updates', { exact: false });
      await expect(emailUpdates).not.toBeVisible();
    });

    test('should NOT show Notifications section header (not implemented)', async ({ page }) => {
      // Check for Notifications as a section header, not just any mention
      const notificationsHeader = page.locator('h2, h3').filter({ hasText: 'Notifications' });
      await expect(notificationsHeader).not.toBeVisible();
    });

    test('should NOT show hardcoded user stats like 127 activities', async ({ page }) => {
      // These are fake hardcoded values that indicate hallucination
      const fakeStats127 = page.getByText('127', { exact: true });
      await expect(fakeStats127).not.toBeVisible();
    });

    test('should NOT show hardcoded user stats like 89 hours', async ({ page }) => {
      const fakeStats89 = page.getByText('89', { exact: true });
      await expect(fakeStats89).not.toBeVisible();
    });

    test('should NOT show hardcoded user stats like 12 insights', async ({ page }) => {
      const fakeStats12 = page.getByText('12', { exact: true });
      await expect(fakeStats12).not.toBeVisible();
    });

    test('Profile stats should come from backend (not hardcoded)', async ({ page }) => {
      // Verify we can see the stats section with real data labels
      const connectedProviders = page.getByText('Connected Providers');
      const daysActive = page.getByText('Days Active');

      await expect(connectedProviders).toBeVisible();
      await expect(daysActive).toBeVisible();
    });

    test('Account tab should show real user status', async ({ page }) => {
      // Click Account tab
      const accountTab = page.getByRole('button', { name: 'Account' });
      await accountTab.click();
      await page.waitForTimeout(300);

      // Should show Status row with real value (Active/Pending)
      const statusLabel = page.getByText('Status', { exact: true });
      await expect(statusLabel).toBeVisible();

      // Should show Role row with real value
      const roleLabel = page.getByText('Role', { exact: true });
      await expect(roleLabel).toBeVisible();
    });
  });

  // ========================================
  // Chat Screen - No Hallucinated Elements
  // ========================================
  test.describe('Chat - No Hallucinated Elements', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
      await navigateToTab(page, 'Chat');
      await waitForNetworkIdle(page);
    });

    test('should NOT show hardcoded fake conversation count', async ({ page }) => {
      // Fake conversation counts like "42 conversations" without backend
      const fakeCount = page.getByText('42 conversations', { exact: false });
      await expect(fakeCount).not.toBeVisible();
    });

    test('should NOT show fake AI response time metrics', async ({ page }) => {
      // Fake metrics like "Average response time: 1.2s"
      const fakeMetric = page.getByText('Average response time', { exact: false });
      await expect(fakeMetric).not.toBeVisible();
    });
  });

  // ========================================
  // Coach Library - No Hallucinated Elements
  // ========================================
  test.describe('Coach Library - No Hallucinated Elements', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
      await navigateToTab(page, 'Coaches');
      await waitForNetworkIdle(page);
    });

    test('should NOT show hardcoded coach count', async ({ page }) => {
      // Should not show fake count like "You have 15 coaches"
      const fakeCoachCount = page.getByText('You have 15 coaches', { exact: false });
      await expect(fakeCoachCount).not.toBeVisible();
    });

    test('should show real category filters from backend', async ({ page }) => {
      // These categories should exist and match backend
      // Use exact: true to differentiate "All" from "All Sources"
      await expect(page.getByRole('button', { name: 'All', exact: true })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Training' })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Nutrition' })).toBeVisible();
    });
  });

  // ========================================
  // Connections Tab - No Hallucinated Providers
  // ========================================
  test.describe('Connections - No Hallucinated Providers', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);

      // Click Data Providers tab
      const dataProvidersTab = page.getByRole('button', { name: 'Data Providers' });
      await dataProvidersTab.click();
      await page.waitForTimeout(300);
    });

    test('should NOT show MyFitnessPal (not a supported provider)', async ({ page }) => {
      const myFitnessPal = page.getByText('MyFitnessPal', { exact: false });
      await expect(myFitnessPal).not.toBeVisible();
    });

    test('should NOT show Peloton (not a supported provider)', async ({ page }) => {
      const peloton = page.getByText('Peloton', { exact: false });
      await expect(peloton).not.toBeVisible();
    });

    test('should NOT show Apple Watch (not a supported provider)', async ({ page }) => {
      const appleWatch = page.getByText('Apple Watch', { exact: false });
      await expect(appleWatch).not.toBeVisible();
    });
  });

  // ========================================
  // API Tokens Tab - No Hallucinated Data
  // ========================================
  test.describe('API Tokens - No Hallucinated Data', () => {
    test.beforeEach(async ({ page }) => {
      await loginAsUser(page, 'webtest');
      await navigateToTab(page, 'Settings');
      await waitForNetworkIdle(page);

      // Click API Tokens tab
      const tokensTab = page.getByRole('button', { name: 'API Tokens' });
      await tokensTab.click();
      await page.waitForTimeout(300);
    });

    test('should NOT show hardcoded token usage stats', async ({ page }) => {
      // Should not show fake stats like "1,234 requests today"
      const fakeRequests = page.getByText('1,234 requests', { exact: false });
      await expect(fakeRequests).not.toBeVisible();
    });

    test('should show real active tokens count from backend', async ({ page }) => {
      // Should see "X active tokens" where X comes from backend
      const activeTokensText = page.getByText(/\d+ active tokens?/);
      await expect(activeTokensText).toBeVisible();
    });
  });
});
