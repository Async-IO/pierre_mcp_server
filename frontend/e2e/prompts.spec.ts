// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for prompt suggestions functionality.
// ABOUTME: Tests admin prompts management, user-facing prompts display, and skip onboarding flow.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Mock prompt suggestions API response
const mockPromptSuggestions = {
  categories: [
    {
      category_key: 'training',
      category_title: 'Training',
      category_icon: '🏃',
      pillar: 'activity',
      prompts: ['Am I ready for a hard workout today?', 'What should my training focus be this week?'],
    },
    {
      category_key: 'nutrition',
      category_title: 'Nutrition',
      category_icon: '🥗',
      pillar: 'nutrition',
      prompts: ['What should I eat before my long run?', 'How can I improve my recovery nutrition?'],
    },
    {
      category_key: 'recovery',
      category_title: 'Recovery',
      category_icon: '😴',
      pillar: 'recovery',
      prompts: ['Am I getting enough rest?', 'How is my sleep affecting my training?'],
    },
    {
      category_key: 'recipes',
      category_title: 'Recipes',
      category_icon: '🍳',
      pillar: 'nutrition',
      prompts: ['Give me a high-protein post-workout meal idea', 'What are some easy pre-race breakfast options?'],
    },
  ],
  welcome_prompt: 'List my last 20 activities with key insights about my training patterns.',
  metadata: {
    timestamp: new Date().toISOString(),
    api_version: '1.0',
  },
};

// Mock admin prompt categories response (includes additional fields)
const mockAdminPromptCategories = [
  {
    id: 'cat-1',
    category_key: 'training',
    category_title: 'Training',
    category_icon: '🏃',
    pillar: 'activity',
    prompts: ['Am I ready for a hard workout today?', 'What should my training focus be this week?'],
    display_order: 0,
    is_active: true,
  },
  {
    id: 'cat-2',
    category_key: 'nutrition',
    category_title: 'Nutrition',
    category_icon: '🥗',
    pillar: 'nutrition',
    prompts: ['What should I eat before my long run?', 'How can I improve my recovery nutrition?'],
    display_order: 1,
    is_active: true,
  },
  {
    id: 'cat-3',
    category_key: 'recovery',
    category_title: 'Recovery',
    category_icon: '😴',
    pillar: 'recovery',
    prompts: ['Am I getting enough rest?', 'How is my sleep affecting my training?'],
    display_order: 2,
    is_active: true,
  },
  {
    id: 'cat-4',
    category_key: 'recipes',
    category_title: 'Recipes',
    category_icon: '🍳',
    pillar: 'nutrition',
    prompts: ['Give me a high-protein post-workout meal idea', 'What are some easy pre-race breakfast options?'],
    display_order: 3,
    is_active: true,
  },
];

// Mock admin welcome prompt response
const mockAdminWelcomePrompt = {
  prompt_text: 'List my last 20 activities with key insights about my training patterns.',
};

async function setupPromptsMocks(page: Page, options: { isAdmin?: boolean } = {}) {
  const { isAdmin = false } = options;

  // Set up base dashboard mocks
  await setupDashboardMocks(page, { role: isAdmin ? 'admin' : 'user' });

  // Mock public prompts endpoint (user-facing)
  await page.route('**/api/prompts/suggestions', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockPromptSuggestions),
    });
  });

  // Mock admin prompts endpoints
  if (isAdmin) {
    // List all prompt categories
    await page.route('**/api/admin/prompts', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockAdminPromptCategories),
        });
      } else if (route.request().method() === 'POST') {
        // Create new category
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'cat-new',
            category_key: 'new_category',
            category_title: 'New Category',
            category_icon: '✨',
            pillar: 'activity',
            prompts: ['New prompt'],
            display_order: 4,
            is_active: true,
          }),
        });
      } else {
        await route.continue();
      }
    });

    // Individual category operations
    await page.route('**/api/admin/prompts/*', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockAdminPromptCategories[0]),
        });
      } else if (route.request().method() === 'PUT') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            ...mockAdminPromptCategories[0],
            category_title: 'Updated Training',
          }),
        });
      } else if (route.request().method() === 'DELETE') {
        await route.fulfill({
          status: 204,
        });
      } else {
        await route.continue();
      }
    });

    // Welcome prompt endpoints
    await page.route('**/api/admin/prompts/welcome', async (route) => {
      if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockAdminWelcomePrompt),
        });
      } else if (route.request().method() === 'PUT') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            prompt_text: 'Updated welcome prompt',
          }),
        });
      } else {
        await route.continue();
      }
    });

    // Reset to defaults endpoint
    await page.route('**/api/admin/prompts/reset', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });
  }
}

test.describe('User Prompts Display', () => {
  test('displays prompt categories for users without connected providers', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });
    await loginToDashboard(page);

    // Non-admin users see chat-first layout with header
    await page.waitForSelector('header', { timeout: 10000 });

    // Should see the provider connection screen first - find skip button by aria-label
    // The button shows "Start chatting" text but has aria-label "Skip and start chatting"
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await expect(skipButton).toBeVisible({ timeout: 10000 });

    // Click skip to see the prompts
    await skipButton.click();

    // Now should see prompt categories
    await expect(page.getByText('Training').first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Nutrition').first()).toBeVisible();
    await expect(page.getByText('Recovery').first()).toBeVisible();
    await expect(page.getByText('Recipes').first()).toBeVisible();
  });

  test('displays prompt suggestions within each category', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });
    await loginToDashboard(page);

    // Skip provider onboarding - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await skipButton.click();
    await page.waitForTimeout(300);

    // Check prompts are visible
    await expect(page.getByText('Am I ready for a hard workout today?')).toBeVisible();
    await expect(page.getByText('What should I eat before my long run?')).toBeVisible();
  });

  test('clicking a prompt sends it as a message', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });

    // Mock conversation creation
    let conversationCreated = false;
    await page.route('**/api/chat/conversations', async (route) => {
      if (route.request().method() === 'POST') {
        conversationCreated = true;
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'conv-1',
            title: 'New Conversation',
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ conversations: [], total: 0, limit: 50, offset: 0 }),
        });
      }
    });

    // Mock message sending
    let messageSent = false;
    let sentMessage = '';
    await page.route('**/api/chat/conversations/*/messages', async (route) => {
      if (route.request().method() === 'POST') {
        messageSent = true;
        const body = route.request().postDataJSON();
        sentMessage = body.content;
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'msg-1',
            role: 'user',
            content: sentMessage,
            created_at: new Date().toISOString(),
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ messages: [], total: 0 }),
        });
      }
    });

    await loginToDashboard(page);

    // Skip provider onboarding - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await skipButton.click();
    await page.waitForTimeout(300);

    // Click on a prompt
    await page.getByText('Am I ready for a hard workout today?').click();

    // Wait for API calls
    await page.waitForTimeout(500);

    expect(conversationCreated).toBe(true);
    expect(messageSent).toBe(true);
    expect(sentMessage).toBe('Am I ready for a hard workout today?');
  });

  test('shows loading state while fetching prompts', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Set up slow response
    await page.route('**/api/prompts/suggestions', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockPromptSuggestions),
      });
    });

    await loginToDashboard(page);

    // Skip provider onboarding - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await skipButton.click();

    // Should see loading skeleton
    await expect(page.locator('.animate-pulse').first()).toBeVisible({ timeout: 2000 });
  });

  test('shows error state when prompts API fails', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'user' });

    // Mock failing prompts API
    await page.route('**/api/prompts/suggestions', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);

    // Skip provider onboarding - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await skipButton.click();

    // Should see error message
    await expect(page.getByText('Failed to load prompt suggestions')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Skip Onboarding Flow', () => {
  test('Skip button shows prompt suggestions screen', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });
    await loginToDashboard(page);

    // Wait for provider connection cards
    await page.waitForSelector('header', { timeout: 10000 });

    // Should see skip button - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await expect(skipButton).toBeVisible();

    // Click skip
    await skipButton.click();

    // Should now see prompts (not provider cards)
    await expect(page.getByText('Training').first()).toBeVisible({ timeout: 5000 });

    // Provider connect cards should not be visible - check Strava button is gone
    await expect(page.getByRole('button', { name: /Connect to Strava/ })).not.toBeVisible();
  });

  test('can start new chat after skipping onboarding', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });

    // Mock conversation creation
    await page.route('**/api/chat/conversations', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'conv-1',
            title: 'New Conversation',
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ conversations: [], total: 0, limit: 50, offset: 0 }),
        });
      }
    });

    await page.route('**/api/chat/conversations/*/messages', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ messages: [], total: 0 }),
      });
    });

    await loginToDashboard(page);

    // Skip onboarding - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await skipButton.click();
    await page.waitForTimeout(300);

    // Should see prompt categories
    await expect(page.getByText('Training').first()).toBeVisible();

    // Click a prompt to start chatting
    await page.getByText('Am I ready for a hard workout today?').click();
    await page.waitForTimeout(500);

    // Should have started a conversation
    await expect(page.locator('textarea')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Admin Prompts Management', () => {
  test('displays Prompts tab for admin users', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Prompts tab should be visible for admin users
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Prompts")') })).toBeVisible();
  });

  test('hides Prompts tab for non-admin users', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });
    await loginToDashboard(page);

    // Non-admin users see chat-first layout (no sidebar)
    await page.waitForSelector('header', { timeout: 10000 });

    // Prompts tab should not be visible
    await expect(page.locator('button').filter({ has: page.locator('span:has-text("Prompts")') })).not.toBeVisible();
  });

  test('displays prompt categories in admin view', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');

    // Wait for content to load
    await expect(page.getByText('Prompt Management')).toBeVisible({ timeout: 10000 });

    // Should display categories
    await expect(page.getByText('Training').first()).toBeVisible();
    await expect(page.getByText('Nutrition').first()).toBeVisible();
    await expect(page.getByText('Recovery').first()).toBeVisible();
    await expect(page.getByText('Recipes').first()).toBeVisible();
  });

  test('displays total prompt count in header', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');

    // Wait for content
    await expect(page.getByText('Prompt Management')).toBeVisible({ timeout: 10000 });

    // Should show total prompt counts in header (4 categories × 2 prompts = 8 total)
    // UI shows: "4 categories • 8 prompts"
    await expect(page.getByText('8 prompts')).toBeVisible();
  });

  test('can reset prompts to defaults', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });

    let resetCalled = false;
    await page.route('**/api/admin/prompts/reset', async (route) => {
      resetCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');
    await expect(page.getByText('Prompt Management')).toBeVisible({ timeout: 10000 });

    // Click reset button in header (first one)
    await page.getByRole('button', { name: /Reset to Defaults/i }).first().click();

    // Wait for confirmation modal to appear
    await expect(page.getByText('Are you sure you want to reset all prompt categories')).toBeVisible();

    // Click the confirmation button in the modal (use nth(1) to get the second "Reset to Defaults" button)
    await page.getByRole('button', { name: /Reset to Defaults/i }).nth(1).click();

    await page.waitForTimeout(500);
    expect(resetCalled).toBe(true);
  });

  test('welcome prompt tab shows current welcome message', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');
    await expect(page.getByText('Prompt Management')).toBeVisible({ timeout: 10000 });

    // Click Welcome Prompt tab
    await page.getByRole('tab', { name: /Welcome Prompt/i }).click();
    await page.waitForTimeout(300);

    // Should show current welcome prompt
    await expect(page.getByText('List my last 20 activities')).toBeVisible();
  });

  test('can update welcome prompt', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });

    let updateCalled = false;
    await page.route('**/api/admin/prompts/welcome', async (route) => {
      if (route.request().method() === 'PUT') {
        updateCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            prompt_text: 'New welcome prompt',
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockAdminWelcomePrompt),
        });
      }
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');
    await expect(page.getByText('Prompt Management')).toBeVisible({ timeout: 10000 });

    // Click Welcome Prompt tab
    await page.getByRole('tab', { name: /Welcome Prompt/i }).click();
    await page.waitForTimeout(300);

    // Find and modify the textarea
    const textarea = page.locator('textarea');
    await textarea.clear();
    await textarea.fill('New welcome prompt');

    // Save changes
    const saveButton = page.getByRole('button', { name: /Save/i });
    await saveButton.click();

    await page.waitForTimeout(500);
    expect(updateCalled).toBe(true);
  });
});

test.describe('Admin Prompts Error Handling', () => {
  test('shows error when failing to load admin prompts', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    // Mock failing admin prompts API
    await page.route('**/api/admin/prompts', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');

    // Should show error message
    await expect(page.getByText(/Failed to load|Error/i)).toBeVisible({ timeout: 10000 });
  });

  test('shows error when reset fails', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: true });

    // Override reset endpoint to fail
    await page.route('**/api/admin/prompts/reset', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Reset failed' }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });

    // Navigate to Prompts tab
    await navigateToTab(page, 'Prompts');
    await expect(page.getByText('Prompt Management')).toBeVisible({ timeout: 10000 });

    // Click reset button in header (first one)
    await page.getByRole('button', { name: /Reset to Defaults/i }).first().click();

    // Wait for confirmation modal to appear
    await expect(page.getByText('Are you sure you want to reset all prompt categories')).toBeVisible();

    // Click the confirmation button in the modal
    await page.getByRole('button', { name: /Reset to Defaults/i }).nth(1).click();

    // Should show error message in modal
    await expect(page.getByText('Failed to reset prompts')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Prompts Pillar Styling', () => {
  test('displays correct pillar gradients for categories', async ({ page }) => {
    await setupPromptsMocks(page, { isAdmin: false });
    await loginToDashboard(page);

    // Skip provider onboarding - use aria-label selector
    const skipButton = page.getByRole('button', { name: 'Skip and start chatting' });
    await skipButton.click();
    await page.waitForTimeout(300);

    // Training category should have activity gradient
    const trainingIcon = page.locator('[role="img"][aria-label="Training category"]');
    await expect(trainingIcon).toHaveClass(/bg-gradient-activity/);

    // Nutrition category should have nutrition gradient
    const nutritionIcon = page.locator('[role="img"][aria-label="Nutrition category"]');
    await expect(nutritionIcon).toHaveClass(/bg-gradient-nutrition/);

    // Recovery category should have recovery gradient
    const recoveryIcon = page.locator('[role="img"][aria-label="Recovery category"]');
    await expect(recoveryIcon).toHaveClass(/bg-gradient-recovery/);
  });
});
