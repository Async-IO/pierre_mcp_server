// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: E2E tests for social feed actions and conversation management
// ABOUTME: Web equivalents of mobile swipe gestures (reactions, adapt, rename, delete)

import { test, expect } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

const mockFeedItems = {
  items: [
    {
      insight: {
        id: 'insight-1',
        user_id: 'user-2',
        visibility: 'friends_only',
        insight_type: 'achievement',
        sport_type: 'Running',
        content: 'Completed a 20km long run at marathon pace!',
        title: 'Marathon Training Progress',
        training_phase: 'build',
        reaction_count: 5,
        adapt_count: 2,
        created_at: '2024-06-01T08:00:00Z',
        updated_at: '2024-06-01T08:00:00Z',
        expires_at: null,
        source_activity_id: null,
        coach_generated: false,
      },
      author: {
        user_id: 'user-2',
        display_name: 'Jane Runner',
        email: 'jane@example.com',
      },
      reactions: {
        like: 3,
        celebrate: 2,
        inspire: 0,
        support: 0,
        total: 5,
      },
      user_reaction: null,
      user_has_adapted: false,
    },
    {
      insight: {
        id: 'insight-2',
        user_id: 'user-3',
        visibility: 'friends_only',
        insight_type: 'recovery',
        sport_type: 'Cycling',
        content: 'Recovery ride after yesterday\'s intervals helped flush lactic acid.',
        title: 'Recovery Strategy',
        training_phase: 'recovery',
        reaction_count: 1,
        adapt_count: 0,
        created_at: '2024-06-01T06:00:00Z',
        updated_at: '2024-06-01T06:00:00Z',
        expires_at: null,
        source_activity_id: null,
        coach_generated: false,
      },
      author: {
        user_id: 'user-3',
        display_name: 'Bob Cyclist',
        email: 'bob@example.com',
      },
      reactions: {
        like: 1,
        celebrate: 0,
        inspire: 0,
        support: 0,
        total: 1,
      },
      user_reaction: null,
      user_has_adapted: false,
    },
  ],
  next_cursor: null,
  has_more: false,
  metadata: { timestamp: '2024-06-01T10:00:00Z', api_version: 'v1' },
};

const mockConversations = {
  conversations: [
    {
      id: 'conv-1',
      title: 'Training Plan Discussion',
      coach_id: 'coach-marathon',
      coach_name: 'Marathon Coach',
      created_at: '2024-06-01T10:00:00Z',
      updated_at: '2024-06-01T12:00:00Z',
      message_count: 5,
    },
    {
      id: 'conv-2',
      title: 'Nutrition Questions',
      coach_id: 'coach-nutrition',
      coach_name: 'Nutrition Coach',
      created_at: '2024-05-28T08:00:00Z',
      updated_at: '2024-05-30T09:00:00Z',
      message_count: 3,
    },
  ],
  total: 2,
  limit: 50,
  offset: 0,
};

async function setupSocialMocks(page: import('@playwright/test').Page) {
  // Social feed
  await page.route('**/api/social/feed**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockFeedItems),
    });
  });

  // Social suggestions
  await page.route('**/api/social/insights/suggestions**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ suggestions: [], total: 0, metadata: { timestamp: '2024-06-01T10:00:00Z', api_version: 'v1' } }),
    });
  });

  // Social friends (needed for Friends tab navigation button)
  await page.route('**/api/social/friends**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ friends: [], total: 0, metadata: { timestamp: '2024-06-01T10:00:00Z', api_version: 'v1' } }),
    });
  });

  // Reaction endpoint
  await page.route('**/api/social/insights/*/reactions', async (route, request) => {
    if (request.method() === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          reaction: {
            id: 'reaction-new',
            insight_id: 'insight-1',
            user_id: 'user-123',
            reaction_type: 'like',
            created_at: '2024-06-01T10:00:00Z',
          },
          updated_counts: { like: 4, celebrate: 2, inspire: 0, support: 0, total: 6 },
          metadata: { timestamp: '2024-06-01T10:00:00Z', api_version: 'v1' },
        }),
      });
    } else if (request.method() === 'DELETE') {
      await route.fulfill({ status: 204 });
    } else {
      await route.fallback();
    }
  });

  // Adapt endpoint: /api/social/insights/:id/adapt (from @pierre/api-client ENDPOINTS.SOCIAL.INSIGHT_ADAPT)
  await page.route('**/api/social/insights/*/adapt', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        adapted: {
          id: 'adapted-1',
          user_id: 'user-123',
          source_insight_id: 'insight-1',
          adapted_content: 'Based on your training profile, try 18km at marathon pace.',
          adaptation_context: 'Personalized for your current fitness level.',
          created_at: '2024-06-01T10:00:00Z',
        },
        source_insight: mockFeedItems.items[0].insight,
        metadata: { timestamp: '2024-06-01T10:00:00Z', api_version: 'v1' },
      }),
    });
  });
}

async function setupConversationMocks(page: import('@playwright/test').Page) {
  // Override the default empty conversations mock from setupDashboardMocks
  await page.route('**/api/chat/conversations**', async (route, request) => {
    if (request.method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockConversations),
      });
    } else if (request.method() === 'DELETE') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    } else if (request.method() === 'PATCH') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          ...mockConversations.conversations[0],
          title: 'Renamed Conversation',
        }),
      });
    } else {
      await route.fallback();
    }
  });
}

test.describe('Social Feed Actions', () => {
  test.beforeEach(async ({ page }) => {
    // Catch-all: forward unmocked API/OAuth requests to specific route handlers via fallback().
    // Uses URL function (not glob) to avoid intercepting Vite source file paths like /src/services/api/client.ts.
    // Registered first so it runs last in LIFO; fallback() ensures specific mocks take priority.
    await page.route((url) => url.pathname.startsWith('/api/') || url.pathname.startsWith('/oauth/'), async (route) => {
      await route.fallback();
    });
    await setupDashboardMocks(page, { role: 'user' });
    await setupSocialMocks(page);
    // Mock endpoints needed by ChatTab (user's default tab) to prevent 401 logout
    await page.route('**/api/providers', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ providers: [] }),
      });
    });
    await page.route('**/api/coaches**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ coaches: [], total: 0 }),
      });
    });
    await loginToDashboard(page);
  });

  test('should display feed items with reactions and adapt buttons', async ({ page }) => {
    await navigateToTab(page, 'Insights');

    // Feed items should render
    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Recovery Strategy')).toBeVisible();

    // Author names
    await expect(page.getByText('Jane Runner')).toBeVisible();
    await expect(page.getByText('Bob Cyclist')).toBeVisible();

    // Insight content
    await expect(page.getByText('Completed a 20km long run at marathon pace!')).toBeVisible();
  });

  test('should display reaction buttons with emoji icons', async ({ page }) => {
    await navigateToTab(page, 'Insights');

    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    // Reaction buttons should be visible with emoji
    const likeButtons = page.locator('button[title="Like"]');
    await expect(likeButtons.first()).toBeVisible();

    const celebrateButtons = page.locator('button[title="Celebrate"]');
    await expect(celebrateButtons.first()).toBeVisible();
  });

  test('should display insight type and context badges', async ({ page }) => {
    await navigateToTab(page, 'Insights');

    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    // Insight type badge (exact match to avoid hitting headings/content containing the same word)
    await expect(page.getByText('Achievement', { exact: true })).toBeVisible();
    await expect(page.getByText('Recovery', { exact: true })).toBeVisible();

    // Sport type badge
    await expect(page.getByText('Running', { exact: true })).toBeVisible();
    await expect(page.getByText('Cycling', { exact: true })).toBeVisible();
  });

  test('should show Adapt to My Training button on feed items', async ({ page }) => {
    await navigateToTab(page, 'Insights');

    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    const adaptButtons = page.getByRole('button', { name: /Adapt to My Training/i });
    await expect(adaptButtons.first()).toBeVisible();
  });

  test('should click reaction button and trigger API call', async ({ page }) => {
    let reactionCalled = false;
    await page.route('**/api/social/insights/insight-1/reactions', async (route, request) => {
      if (request.method() === 'POST') {
        reactionCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            reaction: { id: 'r-1', insight_id: 'insight-1', user_id: 'user-123', reaction_type: 'like', created_at: '2024-06-01T10:00:00Z' },
            updated_counts: { like: 4, celebrate: 2, inspire: 0, support: 0, total: 6 },
            metadata: { timestamp: '2024-06-01T10:00:00Z', api_version: 'v1' },
          }),
        });
      } else {
        await route.fallback();
      }
    });

    await navigateToTab(page, 'Insights');
    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    // Click the first like button
    const likeButton = page.locator('button[title="Like"]').first();
    await likeButton.click();

    // Verify the API was called
    expect(reactionCalled).toBe(true);
  });

  test('should open adapt modal when clicking Adapt to My Training', async ({ page }) => {
    await navigateToTab(page, 'Insights');
    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    // Click the first Adapt button and wait for the adapt API response
    const adaptButton = page.getByRole('button', { name: /Adapt to My Training/i }).first();
    const [adaptResponse] = await Promise.all([
      page.waitForResponse((resp) => resp.url().includes('/api/social/insights/') && resp.url().includes('/adapt') && resp.status() === 200),
      adaptButton.click(),
    ]);
    expect(adaptResponse.status()).toBe(200);

    // Modal should appear with adapted content
    await expect(page.getByRole('heading', { name: 'Adapt to My Training' })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('heading', { name: 'Personalized for You' })).toBeVisible();
    await expect(page.getByText(/Based on your training profile/)).toBeVisible();
  });

  test('should show Save to Library button in adapt modal', async ({ page }) => {
    await navigateToTab(page, 'Insights');
    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    const adaptButton = page.getByRole('button', { name: /Adapt to My Training/i }).first();
    await Promise.all([
      page.waitForResponse((resp) => resp.url().includes('/api/social/insights/') && resp.url().includes('/adapt') && resp.status() === 200),
      adaptButton.click(),
    ]);

    await expect(page.getByRole('heading', { name: 'Adapt to My Training' })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('button', { name: /Save to Library/i })).toBeVisible();
  });

  test('should show Adapted state after successful adapt', async ({ page }) => {
    await navigateToTab(page, 'Insights');
    await expect(page.getByText('Marathon Training Progress')).toBeVisible({ timeout: 10000 });

    const adaptButton = page.getByRole('button', { name: /Adapt to My Training/i }).first();
    await Promise.all([
      page.waitForResponse((resp) => resp.url().includes('/api/social/insights/') && resp.url().includes('/adapt') && resp.status() === 200),
      adaptButton.click(),
    ]);

    await expect(page.getByRole('heading', { name: 'Adapt to My Training' })).toBeVisible({ timeout: 10000 });

    // Click Save to Library
    await page.getByRole('button', { name: /Save to Library/i }).click();

    // After saving, the button should show "Adapted" state
    await expect(page.getByRole('button', { name: /Adapted/i })).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Conversation Management Actions', () => {
  test.beforeEach(async ({ page }) => {
    // Catch-all: forward unmocked API/OAuth requests to specific route handlers via fallback().
    // Uses URL function (not glob) to avoid intercepting Vite source file paths like /src/services/api/client.ts.
    // Registered first so it runs last in LIFO; fallback() ensures specific mocks take priority.
    await page.route((url) => url.pathname.startsWith('/api/') || url.pathname.startsWith('/oauth/'), async (route) => {
      await route.fallback();
    });
    await setupDashboardMocks(page, { role: 'user' });
    await setupConversationMocks(page);
    // Mock endpoints needed by ChatTab (user's default tab) to prevent 401 logout
    await page.route('**/api/providers', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ providers: [] }),
      });
    });
    await page.route('**/api/coaches**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ coaches: [], total: 0 }),
      });
    });
    await loginToDashboard(page);
  });

  test('should display conversations in sidebar', async ({ page }) => {
    // User defaults to Chat tab, conversations should be in sidebar
    await expect(page.getByText('Training Plan Discussion')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Nutrition Questions')).toBeVisible();
  });

  test('should show rename and delete buttons on hover', async ({ page }) => {
    await expect(page.getByText('Training Plan Discussion')).toBeVisible({ timeout: 10000 });

    // Hover over the conversation item to reveal action buttons (group-hover)
    const conversationItem = page.locator('button:has-text("Training Plan Discussion")');
    await conversationItem.hover();

    // Rename and delete buttons should appear (scoped to this conversation item)
    await expect(conversationItem.getByRole('button', { name: /Rename conversation/i })).toBeVisible();
    await expect(conversationItem.getByRole('button', { name: /Delete conversation/i })).toBeVisible();
  });

  test('should enable rename mode when clicking rename button', async ({ page }) => {
    await expect(page.getByText('Training Plan Discussion')).toBeVisible({ timeout: 10000 });

    // Hover over conversation item to reveal action buttons
    const conversationItem = page.locator('button:has-text("Training Plan Discussion")');
    await conversationItem.hover();

    // Click rename button (scoped to this conversation item)
    const renameButton = conversationItem.getByRole('button', { name: /Rename conversation/i });
    await renameButton.click();

    // Input field should appear with the current title
    const input = page.locator('input[type="text"]').first();
    await expect(input).toBeVisible();
    await expect(input).toHaveValue('Training Plan Discussion');
  });
});
