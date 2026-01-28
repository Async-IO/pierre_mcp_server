// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Shared test helpers for visual testing scenarios.
// ABOUTME: Extends base test-helpers with visual test specific utilities and mocks.

import type { Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab as baseNavigateToTab } from './test-helpers';

// Re-export navigateToTab from base helpers
export const navigateToTab = baseNavigateToTab;

// Visual test user configurations (matching scripts/visual-test-setup.sh)
export const TEST_USERS = {
  admin: {
    email: 'admin@example.com',
    displayName: 'Admin',
    role: 'admin' as const,
  },
  webtest: {
    email: 'webtest@pierre.dev',
    displayName: 'Web Test User',
    role: 'user' as const,
  },
  mobiletest: {
    email: 'mobiletest@pierre.dev',
    displayName: 'Mobile Test User',
    role: 'user' as const,
  },
} as const;

export type TestUserKey = keyof typeof TEST_USERS;

// Visual test configuration
export const VISUAL_TEST_CONFIG = {
  defaultTimeout: 10000,
  screenshotDir: 'test-results/visual-screenshots',
};

/**
 * Setup additional API mocks for visual tests.
 * Complements the base dashboard mocks with endpoints needed for visual tests.
 */
async function setupAdditionalMocks(page: Page): Promise<void> {
  // Mock request logs (Monitor tab) - returns array with correct field names
  await page.route('**/api/dashboard/request-logs**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'log-1',
          api_key_id: 'key-1',
          api_key_name: 'test-key',
          tool_name: 'get_activities',
          status_code: 200,
          response_time_ms: 150,
          timestamp: new Date().toISOString(),
          error_message: null,
        },
        {
          id: 'log-2',
          api_key_id: 'key-1',
          api_key_name: 'test-key',
          tool_name: 'get_athlete',
          status_code: 200,
          response_time_ms: 85,
          timestamp: new Date().toISOString(),
          error_message: null,
        },
      ]),
    });
  });

  // Mock request stats (Monitor tab) - matches RequestStats interface with all fields
  await page.route('**/api/dashboard/request-stats**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total_requests: 100,
        successful_requests: 95,
        average_response_time: 120,
        error_requests: 5,
        requests_per_minute: 2.5,
      }),
    });
  });

  // Mock tool usage breakdown (Tools tab) - this is what ToolUsageBreakdown component uses
  await page.route('**/api/dashboard/tool-usage**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          tool_name: 'get_activities',
          display_name: 'Get Activities',
          request_count: 150,
          success_rate: 98.5,
          average_response_time: 120,
          error_count: 2,
        },
        {
          tool_name: 'get_athlete',
          display_name: 'Get Athlete Profile',
          request_count: 85,
          success_rate: 100.0,
          average_response_time: 45,
          error_count: 0,
        },
      ]),
    });
  });

  // Mock global disabled tools (ToolAvailability component)
  await page.route('**/api/admin/tools/global-disabled', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        message: 'Global disabled tools retrieved',
        data: { disabled_tools: [], count: 0 },
      }),
    });
  });

  // Mock tenant tools list (Tools tab)
  // NOTE: Playwright matches routes in REVERSE order (last registered first)
  // So we check the URL and route.fallback() for /summary requests
  await page.route('**/api/admin/tools/tenant/*', async (route) => {
    const url = route.request().url();
    // Let /summary requests fall through to the next handler
    if (url.includes('/summary')) {
      await route.fallback();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        message: 'Tenant tools retrieved',
        data: [
          {
            tool_name: 'get_activities',
            display_name: 'Get Activities',
            description: 'Retrieve user activities from connected providers',
            category: 'Activities',
            is_enabled: true,
            source: 'Default',
            min_plan: 'free',
          },
          {
            tool_name: 'get_athlete',
            display_name: 'Get Athlete Profile',
            description: 'Retrieve athlete profile information',
            category: 'Profile',
            is_enabled: true,
            source: 'Default',
            min_plan: 'free',
          },
        ],
      }),
    });
  });

  // Mock tenant tools summary (Tools tab) - registered after generic route
  // but checked first due to Playwright's reverse matching, then falls back to above
  await page.route('**/api/admin/tools/tenant/*/summary', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        message: 'Tool availability summary retrieved',
        data: {
          tenant_id: 'user-123',
          total_tools: 25,
          enabled_tools: 23,
          disabled_tools: 2,
          overridden_tools: 1,
          globally_disabled_count: 0,
          plan_restricted_count: 2,
        },
      }),
    });
  });

  // Mock system coaches (Admin Coaches tab) - correct field names
  await page.route('**/api/admin/coaches**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: [
          {
            id: 'system-coach-1',
            title: 'Training Coach',
            description: 'Built-in training coach for all users',
            system_prompt: 'You are a professional training coach...',
            category: 'Training',
            tags: ['training', 'fitness'],
            token_count: 150,
            use_count: 42,
            visibility: 'tenant',
            is_favorite: false,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
        ],
        total: 1,
      }),
    });
  });

  // Mock hidden coaches
  await page.route('**/api/coaches/hidden', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ coaches: [] }),
    });
  });

  // Mock user profile endpoint
  await page.route('**/api/user/profile**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        user_id: 'user-123',
        email: 'test@pierre.dev',
        display_name: 'Test User',
        created_at: new Date().toISOString(),
      }),
    });
  });

  // Mock admin tokens list
  await page.route('**/api/admin/mcp-tokens**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ tokens: [] }),
    });
  });

  // Mock admin config settings
  await page.route('**/api/admin/config**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ settings: [] }),
    });
  });
}

/**
 * Setup social feature mocks for visual tests.
 */
async function setupSocialMocks(page: Page): Promise<void> {
  // Mock friends list - check URL to handle both /friends and /friends/pending
  // NOTE: Playwright matches routes in REVERSE order (last registered first)
  await page.route('**/api/social/friends**', async (route) => {
    const url = route.request().url();

    // Handle /pending requests
    if (url.includes('/pending')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          received: [],
          sent: [],
          metadata: {
            timestamp: new Date().toISOString(),
            api_version: '1.0',
          },
        }),
      });
      return;
    }

    // Handle regular friends list
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        friends: [
          {
            id: 'connection-1',
            initiator_id: 'user-123',
            receiver_id: 'friend-1',
            status: 'accepted',
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
            accepted_at: new Date().toISOString(),
            friend_display_name: 'Mobile Test User',
            friend_email: 'mobiletest@pierre.dev',
            friend_user_id: 'friend-1',
          },
        ],
        total: 1,
        metadata: {
          timestamp: new Date().toISOString(),
          api_version: '1.0',
        },
      }),
    });
  });

  // Mock social feed with correct format (items array, not insights)
  await page.route('**/api/social/feed**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        items: [
          {
            insight: {
              id: 'insight-1',
              user_id: 'friend-1',
              visibility: 'friends',
              insight_type: 'achievement',
              sport_type: 'running',
              content: 'Just completed a great tempo run! Feeling strong.',
              title: 'Tempo Run PR',
              training_phase: 'build',
              reaction_count: 3,
              adapt_count: 1,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
              expires_at: null,
              source_activity_id: null,
              coach_generated: false,
            },
            author: {
              user_id: 'friend-1',
              display_name: 'Mobile Test User',
              email: 'mobiletest@pierre.dev',
            },
            reactions: {
              like: 2,
              celebrate: 1,
              inspire: 0,
              support: 0,
              total: 3,
            },
            user_reaction: null,
            user_has_adapted: false,
          },
        ],
        next_cursor: null,
        has_more: false,
      }),
    });
  });

  // Mock insight suggestions for Social Feed
  await page.route('**/api/social/insights/suggestions**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        suggestions: [],
      }),
    });
  });

  // Mock social settings
  await page.route('**/api/social/settings**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        default_visibility: 'friends',
        discoverable: true,
        notify_reactions: true,
        notify_friend_requests: true,
      }),
    });
  });

  // Mock user search
  await page.route('**/api/social/users/search**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        users: [
          { user_id: 'alice-1', display_name: 'Alice Johnson', email: 'alice@acme.com' },
        ],
      }),
    });
  });

  // Mock adapted insights
  await page.route('**/api/social/adapted-insights**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ insights: [] }),
    });
  });

  // Mock reactions endpoint
  await page.route('**/api/social/insights/*/reactions', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true }),
    });
  });

  // Mock coaches list - matches ListCoachesResponse interface
  await page.route('**/api/coaches**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: [
          {
            id: 'coach-1',
            title: 'Training Coach',
            description: 'Your personal training assistant',
            system_prompt: 'You are a helpful training coach.',
            category: 'Training',
            tags: ['training', 'fitness'],
            token_count: 50,
            is_favorite: false,
            use_count: 5,
            visibility: 'private',
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
        ],
        total: 1,
        metadata: {
          timestamp: new Date().toISOString(),
          api_version: '1.0',
        },
      }),
    });
  });

  // Mock store coaches - matches BrowseCoachesResponse interface
  await page.route('**/api/store/coaches**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: [
          {
            id: 'store-coach-1',
            title: 'Nutrition Expert',
            description: 'Expert nutrition guidance',
            category: 'Nutrition',
            tags: ['nutrition', 'health'],
            sample_prompts: ['What should I eat before a workout?'],
            token_count: 100,
            install_count: 50,
            icon_url: null,
            published_at: new Date().toISOString(),
            author_id: 'author-1',
          },
        ],
        next_cursor: null,
      }),
    });
  });
}

/**
 * Login to the application with mocked backend.
 * Sets up all necessary mocks before login.
 */
export async function loginAsUser(page: Page, userKey: TestUserKey): Promise<void> {
  const user = TEST_USERS[userKey];

  // Setup dashboard mocks with appropriate role
  await setupDashboardMocks(page, {
    role: user.role,
    email: user.email,
    displayName: user.displayName,
  });

  // Setup additional API mocks (monitor, admin coaches, etc.)
  await setupAdditionalMocks(page);

  // Setup social mocks
  await setupSocialMocks(page);

  // Login through the form
  await loginToDashboard(page, { email: user.email, password: 'TestPassword123!' });
}

/**
 * Take a screenshot with consistent naming.
 */
export async function takeVisualScreenshot(
  page: Page,
  testName: string,
  stepName: string
): Promise<void> {
  const filename = `${testName.replace(/\s+/g, '-').toLowerCase()}_${stepName.replace(/\s+/g, '-').toLowerCase()}.png`;
  await page.screenshot({
    path: `${VISUAL_TEST_CONFIG.screenshotDir}/${filename}`,
    fullPage: false,
  });
}

/**
 * Wait for network idle (useful after actions that trigger API calls).
 */
export async function waitForNetworkIdle(page: Page, timeout = 5000): Promise<void> {
  await page.waitForLoadState('networkidle', { timeout }).catch(() => {
    // Ignore timeout - network may never be truly idle with mocks
  });
}
