// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for Admin Configuration management.
// ABOUTME: Tests parameter viewing, modification, search, filtering, and audit history.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Mock configuration catalog data
// Uses real category names that match AdminConfiguration.tsx groupings:
// - Server categories: rate_limiting (required so Server/Intelligence toggle appears on load)
// - Intelligence categories: training_stress, heart_rate_zones, algorithms
const mockConfigCatalog = {
  success: true,
  data: {
    total_parameters: 13,
    runtime_configurable_count: 11,
    categories: [
      // Server category - MUST be first so component shows toggle buttons on initial load
      {
        id: 'rate_limiting',
        name: 'rate_limiting',
        display_name: 'Rate Limiting',
        description: 'API rate limiting configuration',
        display_order: 0,
        is_active: true,
        parameters: [
          {
            key: 'rate_limiting.requests_per_minute',
            display_name: 'Requests Per Minute',
            description: 'Maximum API requests allowed per minute',
            category: 'rate_limiting',
            data_type: 'integer',
            current_value: 60,
            default_value: 60,
            is_modified: false,
            valid_range: { min: 10, max: 1000, step: 10 },
            units: 'requests/min',
            is_runtime_configurable: true,
            requires_restart: false,
          },
        ],
      },
      // Intelligence categories
      {
        id: 'training_stress',
        name: 'training_stress',
        display_name: 'Training Stress Balance',
        description: 'TSB thresholds for training load analysis',
        display_order: 1,
        is_active: true,
        parameters: [
          {
            key: 'training_stress.fatigued_threshold',
            display_name: 'Fatigued Threshold',
            description: 'TSB value below which an athlete is considered fatigued',
            category: 'training_stress',
            data_type: 'float',
            current_value: -10.0,
            default_value: -10.0,
            is_modified: false,
            valid_range: { min: -50, max: 0, step: 1 },
            units: 'TSS',
            scientific_basis: 'Based on Banister impulse-response model',
            is_runtime_configurable: true,
            requires_restart: false,
          },
          {
            key: 'training_stress.fresh_max',
            display_name: 'Fresh Maximum',
            description: 'Maximum TSB for optimal freshness',
            category: 'training_stress',
            data_type: 'float',
            current_value: 20.0,
            default_value: 20.0,
            is_modified: false,
            valid_range: { min: 0, max: 50, step: 1 },
            units: 'TSS',
            is_runtime_configurable: true,
            requires_restart: false,
          },
        ],
      },
      {
        id: 'heart_rate_zones',
        name: 'heart_rate_zones',
        display_name: 'Heart Rate Zones',
        description: 'Heart rate zone calculation parameters',
        display_order: 2,
        is_active: true,
        parameters: [
          {
            key: 'heart_rate_zones.zone1_max_percent',
            display_name: 'Zone 1 Max Percentage',
            description: 'Upper bound of Zone 1 as percentage of max HR',
            category: 'heart_rate_zones',
            data_type: 'integer',
            current_value: 60,
            default_value: 60,
            is_modified: false,
            valid_range: { min: 50, max: 70, step: 1 },
            units: '%',
            is_runtime_configurable: true,
            requires_restart: false,
          },
        ],
      },
      {
        id: 'algorithms',
        name: 'algorithms',
        display_name: 'Algorithm Selection',
        description: 'Choose which algorithms to use for analysis',
        display_order: 3,
        is_active: true,
        parameters: [
          {
            key: 'algorithms.running_power_model',
            display_name: 'Running Power Model',
            description: 'Model used for running power estimation',
            category: 'algorithms',
            data_type: 'enum',
            current_value: 'stryd',
            default_value: 'stryd',
            is_modified: false,
            enum_options: ['stryd', 'renato_canova', 'jack_daniels'],
            is_runtime_configurable: true,
            requires_restart: false,
          },
          {
            key: 'algorithms.enable_experimental',
            display_name: 'Enable Experimental Features',
            description: 'Enable experimental algorithms',
            category: 'algorithms',
            data_type: 'boolean',
            current_value: false,
            default_value: false,
            is_modified: false,
            is_runtime_configurable: true,
            requires_restart: true,
          },
        ],
      },
    ],
  },
};

// Mock audit log data
const mockAuditLog = {
  success: true,
  data: {
    entries: [
      {
        id: 'audit-1',
        timestamp: new Date().toISOString(),
        admin_user_id: 'user-123',
        admin_email: 'admin@test.com',
        category: 'training_stress',
        config_key: 'training_stress.fatigued_threshold',
        old_value: -15.0,
        new_value: -10.0,
        data_type: 'float',
        reason: 'Adjusted based on athlete feedback',
      },
      {
        id: 'audit-2',
        timestamp: new Date(Date.now() - 86400000).toISOString(),
        admin_user_id: 'user-123',
        admin_email: 'admin@test.com',
        category: 'algorithms',
        config_key: 'algorithms.running_power_model',
        old_value: 'jack_daniels',
        new_value: 'stryd',
        data_type: 'enum',
        reason: 'Switched to Stryd model for better accuracy',
      },
    ],
    total_count: 2,
  },
};

async function setupAdminConfigMocks(page: Page) {
  // Set up base dashboard mocks with admin role
  await setupDashboardMocks(page, { role: 'admin' });

  // Mock configuration catalog endpoint - use regex for exact matching
  await page.route(/\/api\/admin\/config\/catalog/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockConfigCatalog),
    });
  });

  // Mock configuration audit log endpoint - use regex for exact matching
  await page.route(/\/api\/admin\/config\/audit/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockAuditLog),
    });
  });

  // Mock configuration update endpoint (PUT only, exact path match)
  // Use regex to match exact path, not subpaths like /catalog or /audit
  await page.route(/\/api\/admin\/config$/, async (route) => {
    if (route.request().method() === 'PUT') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            updated_count: 1,
            requires_restart: false,
          },
        }),
      });
    } else {
      await route.continue();
    }
  });

  // Mock configuration reset endpoint - use regex for exact matching
  await page.route(/\/api\/admin\/config\/reset/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: {
          reset_count: 1,
        },
      }),
    });
  });
}

async function navigateToAdminConfig(page: Page) {
  await loginToDashboard(page);
  await page.waitForSelector('nav', { timeout: 10000 });
  await navigateToTab(page, 'Configuration');
  await page.waitForSelector('h1:has-text("Configuration Management")', { timeout: 10000 });
  // Switch to Intelligence tab since mock data uses Intelligence categories
  // Use locator chain: find button containing the Intelligence text
  await page.locator('button:has-text("Intelligence")').click();
  await page.waitForTimeout(300);
}

test.describe('Admin Configuration - Loading and Display', () => {
  test('displays configuration management header and stats', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Check header - dashboard h1 shows "Configuration", component has "Configuration Management"
    await expect(page.getByText('Configuration Management')).toBeVisible();

    // Check parameter count info for Intelligence view (5 params across 3 categories)
    // Component shows filtered counts based on current view, not API total_parameters
    await expect(page.getByText(/5 parameters/)).toBeVisible();
    await expect(page.getByText(/3 categories/)).toBeVisible();
  });

  test('displays all configuration categories', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Check categories are displayed - use first() to target category header
    await expect(page.getByText('Training Stress Balance').first()).toBeVisible();
    await expect(page.getByText('Heart Rate Zones').first()).toBeVisible();
    await expect(page.getByText('Algorithm Selection').first()).toBeVisible();
  });

  test('expands category to show parameters', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Click on TSB category to expand - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    // Check parameters are visible
    await expect(page.getByText('Fatigued Threshold')).toBeVisible();
    await expect(page.getByText('Fresh Maximum')).toBeVisible();
  });

  test('shows parameter details including description and range', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand TSB category - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    // Check parameter description
    await expect(page.getByText('TSB value below which an athlete is considered fatigued')).toBeVisible();

    // Check default value display - use first() as multiple params show Default:
    await expect(page.getByText('Default:').first()).toBeVisible();
    await expect(page.getByText('-10')).toBeVisible();
  });
});

test.describe('Admin Configuration - Search and Filter', () => {
  test('search filters parameters by name', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Type in search box
    const searchInput = page.getByPlaceholder('Search parameters');
    await searchInput.fill('fatigued');

    // Should only show matching parameters
    await expect(page.getByText('Fatigued Threshold')).toBeVisible();
    await expect(page.getByText('Fresh Maximum')).not.toBeVisible();
  });

  test('search filters parameters by key', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    const searchInput = page.getByPlaceholder('Search parameters');
    // Search by actual key substring: heart_rate_zones.zone1_max_percent
    await searchInput.fill('zone1_max');

    // Should show HR parameter
    await expect(page.getByText('Zone 1 Max Percentage')).toBeVisible();
  });

  test('shows no results message when search has no matches', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    const searchInput = page.getByPlaceholder('Search parameters');
    await searchInput.fill('nonexistent parameter xyz');

    // Should show no results
    await expect(page.getByText('No parameters found')).toBeVisible();
  });

  test('clear search restores all categories', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    const searchInput = page.getByPlaceholder('Search parameters');
    await searchInput.fill('fatigued');

    // Clear search
    await page.getByLabel('Clear search').click();

    // All categories should be visible again - use first() for category headers
    await expect(page.getByText('Training Stress Balance').first()).toBeVisible();
    await expect(page.getByText('Heart Rate Zones').first()).toBeVisible();
    await expect(page.getByText('Algorithm Selection').first()).toBeVisible();
  });
});

test.describe('Admin Configuration - Parameter Modification', () => {
  test('modifying a parameter shows unsaved changes badge', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand TSB category - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    // Find and modify the fatigued threshold input
    const input = page.locator('input[type="number"]').first();
    await input.fill('-15');

    // Should show unsaved changes indicator
    await expect(page.getByText(/unsaved changes/)).toBeVisible();
  });

  test('enum parameter shows dropdown with options', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand Algorithms category
    await page.getByText('Algorithm Selection').click();
    await page.waitForTimeout(300);

    // Find the enum select
    const select = page.locator('select').first();
    await expect(select).toBeVisible();

    // Check options exist (use toHaveCount since select options aren't "visible" in DOM sense)
    await expect(select.locator('option[value="stryd"]')).toHaveCount(1);
    await expect(select.locator('option[value="renato_canova"]')).toHaveCount(1);
    await expect(select.locator('option[value="jack_daniels"]')).toHaveCount(1);
  });

  test('boolean parameter shows toggle switch', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand Algorithms category
    await page.getByText('Algorithm Selection').click();
    await page.waitForTimeout(300);

    // Find boolean toggle (role="switch")
    const toggle = page.locator('[role="switch"]').first();
    await expect(toggle).toBeVisible();
  });

  test('clicking Review & Save shows confirmation modal', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand and modify a parameter - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    const input = page.locator('input[type="number"]').first();
    await input.fill('-15');

    // Click save button
    await page.getByRole('button', { name: 'Review & Save Changes' }).click();

    // Modal should appear
    await expect(page.getByText('Confirm Configuration Changes')).toBeVisible();
    await expect(page.getByText('You are about to update')).toBeVisible();
  });

  test('confirming changes calls update API', async ({ page }) => {
    await setupAdminConfigMocks(page);

    let updateCalled = false;
    await page.route('**/api/admin/config', async (route) => {
      if (route.request().method() === 'PUT') {
        updateCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: { updated_count: 1, requires_restart: false },
          }),
        });
      } else {
        await route.continue();
      }
    });

    await navigateToAdminConfig(page);

    // Expand and modify a parameter - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    const input = page.locator('input[type="number"]').first();
    await input.fill('-15');

    // Click save and confirm
    await page.getByRole('button', { name: 'Review & Save Changes' }).click();
    await page.getByRole('button', { name: 'Confirm Changes' }).click();

    // Wait for API call
    await page.waitForTimeout(500);
    expect(updateCalled).toBe(true);
  });

  test('discard button clears pending changes', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand and modify a parameter - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    const input = page.locator('input[type="number"]').first();
    await input.fill('-15');

    // Unsaved changes should be visible
    await expect(page.getByText(/unsaved changes/)).toBeVisible();

    // Click discard
    await page.getByRole('button', { name: 'Discard All' }).click();

    // Unsaved changes badge should disappear
    await expect(page.getByText(/unsaved changes/)).not.toBeVisible();
  });
});

test.describe('Admin Configuration - Reset Functionality', () => {
  test('reset category button shows confirmation modal', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Expand category - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    // Click reset category button
    await page.getByRole('button', { name: 'Reset Category' }).click();

    // Modal should appear - use role to distinguish heading from button
    await expect(page.getByRole('heading', { name: 'Reset to Defaults' })).toBeVisible();
    await expect(page.getByText(/reset all parameters/i)).toBeVisible();
  });

  test('confirming reset calls reset API', async ({ page }) => {
    await setupAdminConfigMocks(page);

    let resetCalled = false;
    await page.route('**/api/admin/config/reset**', async (route) => {
      resetCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { reset_count: 2 },
        }),
      });
    });

    await navigateToAdminConfig(page);

    // Expand category and reset - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    await page.getByRole('button', { name: 'Reset Category' }).click();
    await page.getByRole('button', { name: 'Reset to Defaults' }).click();

    await page.waitForTimeout(500);
    expect(resetCalled).toBe(true);
  });
});

test.describe('Admin Configuration - Audit History', () => {
  test('switching to history tab shows audit log', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Click on History tab
    await page.getByRole('tab', { name: 'Change History' }).click();
    await page.waitForTimeout(500);

    // Should show audit entries
    await expect(page.getByText('admin@test.com').first()).toBeVisible();
    await expect(page.getByText('training_stress.fatigued_threshold')).toBeVisible();
  });

  test('audit log shows old and new values', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Click on History tab
    await page.getByRole('tab', { name: 'Change History' }).click();
    await page.waitForTimeout(500);

    // Should show value changes
    await expect(page.getByText('-15')).toBeVisible();
    await expect(page.getByText('-10')).toBeVisible();
  });

  test('audit log shows change reasons', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await navigateToAdminConfig(page);

    // Click on History tab
    await page.getByRole('tab', { name: 'Change History' }).click();
    await page.waitForTimeout(500);

    // Should show reasons
    await expect(page.getByText('Adjusted based on athlete feedback')).toBeVisible();
  });
});

test.describe('Admin Configuration - Error Handling', () => {
  test('shows error message when catalog fails to load', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    // Mock failing catalog endpoint
    await page.route('**/api/admin/config/catalog**', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Configuration');

    // Should show error message
    await expect(page.getByText('Failed to load configuration catalog')).toBeVisible({ timeout: 10000 });
  });

  test('shows error in modal when update fails', async ({ page }) => {
    await setupAdminConfigMocks(page);

    // Override update endpoint to fail
    await page.route('**/api/admin/config', async (route) => {
      if (route.request().method() === 'PUT') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Update failed' }),
        });
      } else {
        await route.continue();
      }
    });

    await navigateToAdminConfig(page);

    // Modify and try to save - use first() for category header
    await page.getByText('Training Stress Balance').first().click();
    await page.waitForTimeout(300);

    const input = page.locator('input[type="number"]').first();
    await input.fill('-15');

    await page.getByRole('button', { name: 'Review & Save Changes' }).click();
    await page.getByRole('button', { name: 'Confirm Changes' }).click();

    // Should show error in modal
    await expect(page.getByText('Failed to update configuration')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Admin Configuration - Access Control', () => {
  test('non-admin users cannot see Configuration tab', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });
    await loginToDashboard(page);
    // Non-admin users see sidebar with tabs (Chat, Friends, Social Feed, Settings, etc.)
    await page.waitForSelector('aside', { timeout: 10000 });

    // Non-admin users see the gear icon (Settings) in bottom profile bar, but NOT admin-specific tabs like Configuration
    await expect(page.getByRole('button', { name: 'Settings', exact: true }).first()).toBeVisible();
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Configuration")') })).not.toBeVisible();
  });

  test('admin users can see Configuration tab', async ({ page }) => {
    await setupAdminConfigMocks(page);
    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Configuration tab should be visible for admin users
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Configuration")') })).toBeVisible();
  });
});
