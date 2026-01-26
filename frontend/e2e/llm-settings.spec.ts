// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for LLM Settings management.
// ABOUTME: Tests AI provider configuration, API key validation, and credential management.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard } from './test-helpers';

// Mock LLM settings data - no credentials configured
const mockLlmSettingsEmpty = {
  current_provider: null,
  providers: [
    {
      name: 'gemini',
      display_name: 'Google Gemini',
      has_credentials: false,
      credential_source: null,
      is_active: false,
    },
    {
      name: 'groq',
      display_name: 'Groq (Llama/Mixtral)',
      has_credentials: false,
      credential_source: null,
      is_active: false,
    },
    {
      name: 'local',
      display_name: 'Local LLM (Ollama/vLLM)',
      has_credentials: false,
      credential_source: null,
      is_active: false,
    },
  ],
  user_credentials: [],
  tenant_credentials: [],
};

// Mock LLM settings with user credentials configured
const mockLlmSettingsWithCredentials = {
  current_provider: 'gemini',
  providers: [
    {
      name: 'gemini',
      display_name: 'Google Gemini',
      has_credentials: true,
      credential_source: 'user_specific',
      is_active: true,
    },
    {
      name: 'groq',
      display_name: 'Groq (Llama/Mixtral)',
      has_credentials: false,
      credential_source: null,
      is_active: false,
    },
    {
      name: 'local',
      display_name: 'Local LLM (Ollama/vLLM)',
      has_credentials: false,
      credential_source: null,
      is_active: false,
    },
  ],
  user_credentials: [
    {
      id: 'cred-123',
      provider: 'gemini',
      user_id: 'user-123',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    },
  ],
  tenant_credentials: [],
};

// Mock validation response - valid
const mockValidationSuccess = {
  valid: true,
  provider: 'gemini',
  models: ['gemini-2.0-flash-exp', 'gemini-1.5-pro', 'gemini-1.5-flash'],
  error: null,
};

// Mock validation response - invalid
const mockValidationFailure = {
  valid: false,
  provider: null,
  models: null,
  error: 'API key is invalid or expired',
};

// Mock save response
const mockSaveSuccess = {
  success: true,
  id: 'cred-new-123',
  message: 'GEMINI API key saved successfully',
};

// Mock delete response
const mockDeleteSuccess = {
  success: true,
  message: 'GEMINI API key deleted',
};

async function setupLlmSettingsMocks(page: Page, withCredentials: boolean = false) {
  // Set up base dashboard mocks with user role
  await setupDashboardMocks(page, { role: 'user' });

  // Mock LLM settings GET endpoint
  await page.route('**/api/user/llm-settings', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(withCredentials ? mockLlmSettingsWithCredentials : mockLlmSettingsEmpty),
      });
    } else if (route.request().method() === 'PUT') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockSaveSuccess),
      });
    } else {
      await route.continue();
    }
  });

  // Mock delete endpoint - register BEFORE validate so validate takes precedence
  // (Playwright routes are LIFO - last registered matches first)
  await page.route('**/api/user/llm-settings/*', async (route) => {
    if (route.request().method() === 'DELETE') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockDeleteSuccess),
      });
    } else {
      await route.continue();
    }
  });

  // Mock validation endpoint - register LAST so it takes precedence over wildcard
  await page.route('**/api/user/llm-settings/validate', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockValidationSuccess),
    });
  });
}

async function navigateToAiSettings(page: Page) {
  await loginToDashboard(page);

  // Wait for dashboard to load - all users now have sidebar
  await page.waitForSelector('aside', { timeout: 10000 });

  // Click on Settings tab in sidebar (works for both admin and non-admin users)
  const settingsTab = page.locator('button').filter({ has: page.locator('span:has-text("Settings")') });
  await settingsTab.click();
  await page.waitForTimeout(500);

  // Click on AI Settings tab
  await page.getByRole('button', { name: /AI Settings/i }).click();
  await page.waitForTimeout(300);
}

test.describe('LLM Settings - Display and Navigation', () => {
  test('displays AI Settings tab in user settings', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await loginToDashboard(page);
    await page.waitForSelector('aside', { timeout: 10000 });

    // Click on Settings tab in sidebar
    const settingsTab = page.locator('button').filter({ has: page.locator('span:has-text("Settings")') });
    await settingsTab.click();
    await page.waitForTimeout(500);

    // Check AI Settings tab exists
    await expect(page.getByRole('button', { name: /AI Settings/i })).toBeVisible();
  });

  test('displays all provider options', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Check all providers are displayed (use first() since configured providers show name twice)
    await expect(page.getByText('Google Gemini').first()).toBeVisible();
    await expect(page.getByText('Groq (Llama/Mixtral)').first()).toBeVisible();
    await expect(page.getByText('Local LLM (Ollama/vLLM)').first()).toBeVisible();
  });

  test('shows unconfigured state for providers without credentials', async ({ page }) => {
    await setupLlmSettingsMocks(page, false);
    await navigateToAiSettings(page);

    // Configure buttons should be visible for unconfigured providers
    const configureButtons = page.getByRole('button', { name: 'Configure' });
    await expect(configureButtons.first()).toBeVisible();

    // No "Active" badge should be shown (use locator for Badge specifically)
    await expect(page.locator('[class*="badge"]', { hasText: 'Active' })).not.toBeVisible();
  });

  test('shows configured state with active badge', async ({ page }) => {
    await setupLlmSettingsMocks(page, true);
    await navigateToAiSettings(page);

    // Active badge should be visible for Gemini (first match in provider cards)
    await expect(page.getByText('Active').first()).toBeVisible();

    // "Your Key" badge should be visible
    await expect(page.getByText('Your Key').first()).toBeVisible();

    // Update button should be visible for configured provider
    await expect(page.getByRole('button', { name: 'Update' }).first()).toBeVisible();
  });

  test('shows current provider info when configured', async ({ page }) => {
    await setupLlmSettingsMocks(page, true);
    await navigateToAiSettings(page);

    // Should show active provider message
    await expect(page.getByText(/Active Provider.*Google Gemini/i)).toBeVisible();
  });
});

test.describe('LLM Settings - Configuration Form', () => {
  test('clicking Configure opens configuration form', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Form should appear with API key field
    await expect(page.getByLabel('API Key')).toBeVisible();
  });

  test('shows Base URL field only for local provider', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini first
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Base URL should not be visible for Gemini
    await expect(page.getByLabel('Base URL')).not.toBeVisible();

    // Close form by clicking the X button (has aria-hidden svg inside)
    await page.locator('button').filter({ has: page.locator('svg path[d*="M6 18L18 6"]') }).click();
    await page.waitForTimeout(300);

    // Now configure Local LLM - find the specific provider card via its heading
    // Use p-4 class to target only provider cards (not parent containers)
    const localLlmCard = page.locator('div.p-4').filter({
      has: page.locator('h3', { hasText: 'Local LLM (Ollama/vLLM)' })
    });
    await localLlmCard.getByRole('button', { name: 'Configure' }).click();
    await page.waitForTimeout(300);

    // Base URL should be visible for Local
    await expect(page.getByLabel('Base URL')).toBeVisible();
  });

  test('allows entering API key', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    const apiKeyInput = page.getByLabel('API Key');
    await apiKeyInput.fill('test-api-key-12345');

    // Verify value was entered
    await expect(apiKeyInput).toHaveValue('test-api-key-12345');
  });

  test('shows default model placeholder', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Default model field should show placeholder
    const modelInput = page.getByLabel(/Default Model/);
    await expect(modelInput).toHaveAttribute('placeholder', 'gemini-2.0-flash-exp');
  });
});

test.describe('LLM Settings - Validation', () => {
  test('Test Connection button validates credentials', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    await page.getByLabel('API Key').fill('test-api-key-12345');

    // Click Test Connection
    await page.getByRole('button', { name: 'Test Connection' }).click();

    // Should show success message
    await expect(page.getByText('API key is valid!')).toBeVisible({ timeout: 5000 });
  });

  test('shows available models after successful validation', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    await page.getByLabel('API Key').fill('test-api-key-12345');

    // Click Test Connection
    await page.getByRole('button', { name: 'Test Connection' }).click();

    // Should show available models
    await expect(page.getByText(/Available models.*gemini/i)).toBeVisible({ timeout: 5000 });
  });

  test('shows error for invalid credentials', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Mock settings endpoint
    await page.route('**/api/user/llm-settings', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockLlmSettingsEmpty),
        });
      } else {
        await route.continue();
      }
    });

    // Mock validation to fail
    await page.route('**/api/user/llm-settings/validate', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockValidationFailure),
      });
    });

    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    await page.getByLabel('API Key').fill('invalid-key');

    // Click Test Connection
    await page.getByRole('button', { name: 'Test Connection' }).click();

    // Should show error
    await expect(page.getByText(/API key is invalid/i)).toBeVisible({ timeout: 5000 });
  });
});

test.describe('LLM Settings - Save Credentials', () => {
  test('Save API Key button saves credentials', async ({ page }) => {
    await setupLlmSettingsMocks(page);

    let saveCalled = false;
    await page.route('**/api/user/llm-settings', async (route) => {
      if (route.request().method() === 'PUT') {
        saveCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockSaveSuccess),
        });
      } else if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockLlmSettingsEmpty),
        });
      } else {
        await route.continue();
      }
    });

    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    await page.getByLabel('API Key').fill('test-api-key-12345');

    // Click Save API Key
    await page.getByRole('button', { name: 'Save API Key' }).click();

    // Wait for save to complete
    await page.waitForTimeout(500);

    expect(saveCalled).toBe(true);
  });

  test('shows success message after saving', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    await page.getByLabel('API Key').fill('test-api-key-12345');

    // Click Save API Key
    await page.getByRole('button', { name: 'Save API Key' }).click();

    // Should show success message
    await expect(page.getByText(/saved successfully/i)).toBeVisible({ timeout: 5000 });
  });

  test('disables save button when API key is empty', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Save button should be disabled without API key
    const saveButton = page.getByRole('button', { name: 'Save API Key' });
    await expect(saveButton).toBeDisabled();
  });
});

test.describe('LLM Settings - Delete Credentials', () => {
  test('Remove button shows confirmation dialog', async ({ page }) => {
    await setupLlmSettingsMocks(page, true);
    await navigateToAiSettings(page);

    // Click Remove button
    await page.getByRole('button', { name: 'Remove' }).click();

    // Confirmation dialog should appear
    await expect(page.getByText('Remove API Key')).toBeVisible();
    await expect(page.getByText(/Are you sure/i)).toBeVisible();
  });

  test('confirming delete calls delete API', async ({ page }) => {
    await setupLlmSettingsMocks(page, true);

    let deleteCalled = false;
    await page.route('**/api/user/llm-settings/gemini', async (route) => {
      if (route.request().method() === 'DELETE') {
        deleteCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockDeleteSuccess),
        });
      } else {
        await route.continue();
      }
    });

    await navigateToAiSettings(page);

    // Click Remove button
    await page.getByRole('button', { name: 'Remove' }).click();
    await page.waitForTimeout(300);

    // Confirm delete
    await page.getByRole('button', { name: 'Remove' }).last().click();

    await page.waitForTimeout(500);
    expect(deleteCalled).toBe(true);
  });

  test('canceling delete closes dialog', async ({ page }) => {
    await setupLlmSettingsMocks(page, true);
    await navigateToAiSettings(page);

    // Click Remove button
    await page.getByRole('button', { name: 'Remove' }).click();
    await page.waitForTimeout(300);

    // Cancel delete
    await page.getByRole('button', { name: 'Cancel' }).click();

    // Dialog should close
    await expect(page.getByText('Are you sure')).not.toBeVisible();
  });
});

test.describe('LLM Settings - Error Handling', () => {
  test('shows error when settings fail to load', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Mock settings endpoint to fail
    await page.route('**/api/user/llm-settings', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('aside', { timeout: 10000 });

    // Click on Settings tab in sidebar
    const settingsTab = page.locator('button').filter({ has: page.locator('span:has-text("Settings")') });
    await settingsTab.click();
    await page.waitForTimeout(500);

    // Click on AI Settings tab
    await page.getByRole('button', { name: /AI Settings/i }).click();
    await page.waitForTimeout(300);

    // Should show loading skeleton while failing (TanStack Query handles retries)
    // The component shows a skeleton on error state
    const skeleton = page.locator('.animate-pulse');
    const hasError = await skeleton.isVisible().catch(() => false);

    // Either shows skeleton (loading/error) or error message
    expect(hasError || (await page.getByText(/error/i).isVisible().catch(() => false))).toBeTruthy();
  });

  test('shows error when save fails', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Mock settings endpoint
    await page.route('**/api/user/llm-settings', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockLlmSettingsEmpty),
        });
      } else if (route.request().method() === 'PUT') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Failed to save' }),
        });
      } else {
        await route.continue();
      }
    });

    await navigateToAiSettings(page);

    // Click Configure on Gemini
    await page.getByRole('button', { name: 'Configure' }).first().click();
    await page.waitForTimeout(300);

    // Enter API key
    await page.getByLabel('API Key').fill('test-api-key-12345');

    // Click Save API Key
    await page.getByRole('button', { name: 'Save API Key' }).click();

    // Should show error message
    await expect(page.getByText(/failed/i)).toBeVisible({ timeout: 5000 });
  });
});

test.describe('LLM Settings - Documentation Links', () => {
  test('shows documentation link for each provider', async ({ page }) => {
    await setupLlmSettingsMocks(page);
    await navigateToAiSettings(page);

    // Check documentation links exist
    const docLinks = page.getByText('Documentation');
    await expect(docLinks.first()).toBeVisible();

    // Should have 3 documentation links (one per provider)
    await expect(docLinks).toHaveCount(3);
  });
});
