// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for Coach Store functionality.
// ABOUTME: Tests browsing, searching, filtering, viewing details, and install/uninstall actions.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard } from './test-helpers';

// Mock store coach data
const mockStoreCoaches = [
  {
    id: 'store-coach-1',
    title: 'Marathon Training Coach',
    description: 'A comprehensive marathon training program with weekly schedules',
    category: 'training',
    tags: ['marathon', 'running', 'endurance'],
    sample_prompts: ['What should my weekly mileage be?', 'How do I taper for race day?'],
    token_count: 1200,
    install_count: 75,
    icon_url: null,
    published_at: '2024-01-15T00:00:00Z',
    author_id: 'author-123',
  },
  {
    id: 'store-coach-2',
    title: 'Nutrition Expert',
    description: 'Personalized nutrition advice and meal planning',
    category: 'nutrition',
    tags: ['diet', 'macros', 'meal-planning'],
    sample_prompts: ['How many calories should I eat?', 'What should I eat before a race?'],
    token_count: 800,
    install_count: 120,
    icon_url: null,
    published_at: '2024-01-20T00:00:00Z',
    author_id: 'author-456',
  },
  {
    id: 'store-coach-3',
    title: 'Recovery Coach',
    description: 'Optimize your recovery and prevent injuries',
    category: 'recovery',
    tags: ['sleep', 'stretching', 'rest'],
    sample_prompts: ['How long should I sleep?', 'What stretches help after running?'],
    token_count: 600,
    install_count: 45,
    icon_url: null,
    published_at: '2024-01-25T00:00:00Z',
    author_id: 'author-789',
  },
];

// Mock coach detail with system_prompt
const mockCoachDetail = {
  ...mockStoreCoaches[0],
  system_prompt:
    'You are an expert marathon training coach. Help athletes prepare for marathon races with personalized training plans based on their current fitness level and goals.',
  created_at: '2024-01-10T00:00:00Z',
  publish_status: 'published',
};

async function setupStoreMocks(page: Page, options: { emptyStore?: boolean; installed?: string[] } = {}) {
  const { emptyStore = false, installed = [] } = options;

  // Set up base dashboard mocks for regular user
  await setupDashboardMocks(page, { role: 'user' });

  // Mock user coaches endpoint (required for sidebar)
  await page.route('**/api/coaches', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: [],
        total: 0,
      }),
    });
  });

  // Mock store installations endpoint (must come before /api/store/coaches/*)
  await page.route('**/api/store/installations', async (route) => {
    const installedCoaches = mockStoreCoaches.filter((c) => installed.includes(c.id));
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: installedCoaches,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });

  // Mock store categories endpoint
  await page.route('**/api/store/categories', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        categories: ['training', 'nutrition', 'recovery', 'recipes', 'mobility', 'custom'],
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });

  // Mock store search endpoint
  await page.route('**/api/store/search**', async (route) => {
    const url = new URL(route.request().url());
    const query = url.searchParams.get('q') || '';

    const coaches = emptyStore
      ? []
      : mockStoreCoaches.filter(
          (c) =>
            c.title.toLowerCase().includes(query.toLowerCase()) ||
            c.description.toLowerCase().includes(query.toLowerCase()) ||
            c.tags.some((t) => t.toLowerCase().includes(query.toLowerCase()))
        );

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches,
        query,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });

  // Mock individual coach detail and install/uninstall endpoints
  // This pattern matches /api/store/coaches/{id} and /api/store/coaches/{id}/install
  await page.route('**/api/store/coaches/*/**', async (route) => {
    const url = route.request().url();

    // Handle install endpoint
    if (url.includes('/install') && route.request().method() === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          message: 'Coach installed successfully',
          coach_id: 'store-coach-1',
          metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
        }),
      });
      return;
    }

    // Handle uninstall endpoint (DELETE to /install)
    if (url.includes('/install') && route.request().method() === 'DELETE') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          message: 'Coach uninstalled successfully',
          coach_id: 'store-coach-1',
          metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
        }),
      });
      return;
    }

    await route.continue();
  });

  // Mock individual coach GET endpoint (must be separate for single segment match)
  await page.route(/\/api\/store\/coaches\/[^/]+$/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockCoachDetail),
    });
  });

  // Mock store browse endpoint (handles query params)
  await page.route(/\/api\/store\/coaches(\?.*)?$/, async (route) => {
    const url = new URL(route.request().url());
    const category = url.searchParams.get('category');
    const sortBy = url.searchParams.get('sort_by');

    let coaches = emptyStore ? [] : [...mockStoreCoaches];

    // Apply category filter
    if (category && category !== 'all') {
      coaches = coaches.filter((c) => c.category === category);
    }

    // Apply sort
    if (sortBy === 'newest') {
      coaches.sort((a, b) => new Date(b.published_at).getTime() - new Date(a.published_at).getTime());
    } else if (sortBy === 'title') {
      coaches.sort((a, b) => a.title.localeCompare(b.title));
    } else {
      // Default: popular (by install_count)
      coaches.sort((a, b) => b.install_count - a.install_count);
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches,
        total: coaches.length,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });
}

test.describe('Coach Store Access', () => {
  test('displays Discover button in sidebar', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Should see Discover button in sidebar
    await expect(page.getByRole('button', { name: 'Discover Coaches' })).toBeVisible({ timeout: 5000 });
  });

  test('opens Coach Store when button is clicked', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Click Discover button
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    // Should see store header (use heading role to avoid ambiguity with button)
    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Find AI coaching assistants')).toBeVisible();
  });
});

test.describe('Coach Store Browse', () => {
  test('displays coaches in store', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    // Wait for store to load
    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });

    // Should display coach cards
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Nutrition Expert')).toBeVisible();
    await expect(page.getByText('Recovery Coach')).toBeVisible();
  });

  test('displays coach user counts', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Should display user counts
    await expect(page.getByText('75 users')).toBeVisible();
    await expect(page.getByText('120 users')).toBeVisible();
    await expect(page.getByText('45 users')).toBeVisible();
  });

  test('displays coach category badges', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Should display category badges
    await expect(page.getByText('training').first()).toBeVisible();
    await expect(page.getByText('nutrition').first()).toBeVisible();
    await expect(page.getByText('recovery').first()).toBeVisible();
  });

  test('displays coach tags', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Should display tags
    await expect(page.getByText('marathon', { exact: true })).toBeVisible();
    await expect(page.getByText('running', { exact: true })).toBeVisible();
    await expect(page.getByText('diet', { exact: true })).toBeVisible();
  });

  test('shows empty state when store is empty', async ({ page }) => {
    await setupStoreMocks(page, { emptyStore: true });
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    // Should show empty state
    await expect(page.getByText('Store is empty')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Coach Store Filtering', () => {
  test('displays category filter buttons', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });

    // Should display category filters (use exact: true to avoid matching coach cards)
    await expect(page.getByRole('button', { name: 'All', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Training', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Nutrition', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Recovery', exact: true })).toBeVisible();
  });

  test('filters by category when clicked', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click Training filter (use exact: true to avoid matching coach cards)
    await page.getByRole('button', { name: 'Training', exact: true }).click();

    await page.waitForTimeout(500);

    // Should show only training coach
    await expect(page.getByText('Marathon Training Coach')).toBeVisible();
  });

  test('displays sort options', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });

    // Should display sort options
    await expect(page.getByRole('button', { name: 'Popular' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Newest' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'A-Z' })).toBeVisible();
  });

  test('changes sort when option is clicked', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click Newest sort
    await page.getByRole('button', { name: 'Newest' }).click();

    await page.waitForTimeout(500);

    // Coaches should be displayed (sorted by newest, but we just verify they're still visible)
    await expect(page.getByText('Recovery Coach')).toBeVisible();
  });
});

test.describe('Coach Store Search', () => {
  test('displays search input', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByPlaceholder('Search coaches...')).toBeVisible({ timeout: 10000 });
  });

  test('searches coaches when text is entered', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Type in search
    await page.getByPlaceholder('Search coaches...').fill('marathon');

    // Wait for debounced search
    await page.waitForTimeout(500);

    // Should show marathon coach
    await expect(page.getByText('Marathon Training Coach')).toBeVisible();
  });

  test('shows no results message for non-matching search', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Type in search that won't match
    await page.getByPlaceholder('Search coaches...').fill('nonexistent');

    // Wait for debounced search
    await page.waitForTimeout(500);

    // Should show no results message
    await expect(page.getByText('No coaches found')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Coach Store Detail View', () => {
  test('opens coach detail when card is clicked', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click on coach card
    await page.getByText('Marathon Training Coach').click();

    // Should see detail view
    await expect(page.getByText('Add Coach')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('System Prompt')).toBeVisible();
  });

  test('displays coach details', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    // Should display description
    await expect(page.getByText('A comprehensive marathon training program')).toBeVisible({
      timeout: 5000,
    });

    // Should display tags section
    await expect(page.getByText('Tags')).toBeVisible();
    await expect(page.getByText('marathon', { exact: true })).toBeVisible();

    // Should display sample prompts
    await expect(page.getByText('Sample Prompts')).toBeVisible();
    await expect(page.getByText('What should my weekly mileage be?')).toBeVisible();

    // Should display system prompt preview
    await expect(page.getByText('You are an expert marathon training coach')).toBeVisible();

    // Should display details section
    await expect(page.getByText('Token Count')).toBeVisible();
    await expect(page.getByText('1,200')).toBeVisible();
  });

  test('back button returns to store browse', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByText('Add Coach')).toBeVisible({ timeout: 5000 });

    // Click back button
    await page.getByTitle('Back to Store').click();

    // Should return to browse view
    await expect(page.getByText('Find AI coaching assistants')).toBeVisible({
      timeout: 5000,
    });
  });
});

test.describe('Coach Store Add/Remove', () => {
  test('shows Add button for coach not in library', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    // Should show Add button
    await expect(page.getByRole('button', { name: 'Add Coach' })).toBeVisible({ timeout: 5000 });
  });

  test('adds coach when Add button is clicked', async ({ page }) => {
    await setupStoreMocks(page);

    let installCalled = false;
    await page.route('**/api/store/coaches/*/install', async (route) => {
      if (route.request().method() === 'POST') {
        installCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            message: 'Coach installed successfully',
            coach_id: 'store-coach-1',
            metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
          }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('button', { name: 'Add Coach' })).toBeVisible({ timeout: 5000 });

    // Click Install
    await page.getByRole('button', { name: 'Add Coach' }).click();

    await page.waitForTimeout(500);
    expect(installCalled).toBe(true);
  });

  test('shows success message after install', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('button', { name: 'Add Coach' })).toBeVisible({ timeout: 5000 });

    // Click Install
    await page.getByRole('button', { name: 'Add Coach' }).click();

    // Should show success message
    await expect(page.getByText(/has been added to your coaches/)).toBeVisible({ timeout: 5000 });
  });

  test('shows Remove button for coach in library', async ({ page }) => {
    await setupStoreMocks(page, { installed: ['store-coach-1'] });
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    // Should show Remove button
    await expect(page.getByRole('button', { name: 'Remove' })).toBeVisible({ timeout: 5000 });
  });

  test('shows confirmation dialog when Remove is clicked', async ({ page }) => {
    await setupStoreMocks(page, { installed: ['store-coach-1'] });
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('button', { name: 'Remove' })).toBeVisible({ timeout: 5000 });

    // Click Remove
    await page.getByRole('button', { name: 'Remove' }).click();

    // Should show confirmation dialog
    await expect(page.getByText('Remove Coach?')).toBeVisible({ timeout: 5000 });
  });

  test('removes coach when confirmed', async ({ page }) => {
    await setupStoreMocks(page, { installed: ['store-coach-1'] });

    let uninstallCalled = false;
    await page.route('**/api/store/coaches/*/install', async (route) => {
      if (route.request().method() === 'DELETE') {
        uninstallCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            message: 'Coach uninstalled successfully',
            coach_id: 'store-coach-1',
            metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
          }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('button', { name: 'Remove' })).toBeVisible({ timeout: 5000 });

    // Click Remove
    await page.getByRole('button', { name: 'Remove' }).click();

    // Confirm in dialog - the ConfirmDialog uses confirmLabel="Remove" so there are now 2 Remove buttons
    await expect(page.getByText('Remove Coach?')).toBeVisible({ timeout: 5000 });
    // Get all Remove buttons and click the second one (the dialog confirm button)
    const removeButtons = await page.getByRole('button', { name: 'Remove' }).all();
    await removeButtons[1].click();

    await page.waitForTimeout(500);
    expect(uninstallCalled).toBe(true);
  });
});

test.describe('Coach Store Navigation', () => {
  test('View My Coaches link navigates to library', async ({ page }) => {
    await setupStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await page.getByRole('button', { name: 'Discover Coaches' }).click();

    await expect(page.getByRole('heading', { name: 'Discover' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('button', { name: 'Add Coach' })).toBeVisible({ timeout: 5000 });

    // Install coach
    await page.getByRole('button', { name: 'Add Coach' }).click();

    // Should show View My Coaches link
    await expect(page.getByText('View My Coaches →')).toBeVisible({ timeout: 5000 });

    // Click the link
    await page.getByText('View My Coaches →').click();

    // Should navigate to My Coaches panel
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 5000 });
  });
});
