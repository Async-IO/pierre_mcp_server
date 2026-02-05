// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for Admin Coach Store Management functionality.
// ABOUTME: Tests review queue, approve/reject actions, published list, and rejected list.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Mock data for store stats
const mockStoreStats = {
  pending_count: 5,
  published_count: 12,
  rejected_count: 3,
  total_installs: 150,
  rejection_rate: 0.2,
};

// Mock data for pending coaches
const mockPendingCoaches = [
  {
    id: 'pending-coach-1',
    title: 'Marathon Training Coach',
    description: 'A comprehensive marathon training program',
    category: 'Training',
    tags: ['marathon', 'running', 'endurance'],
    sample_prompts: ['What should my weekly mileage be?'],
    token_count: 1200,
    install_count: 0,
    icon_url: null,
    published_at: null,
    author_id: 'author-1',
    author_email: 'coach@example.com',
    system_prompt: 'You are a marathon training coach...',
    created_at: '2024-01-10T00:00:00Z',
    submitted_at: '2024-01-15T10:30:00Z',
    publish_status: 'pending_review',
  },
  {
    id: 'pending-coach-2',
    title: 'Nutrition Advisor',
    description: 'Expert nutrition guidance for athletes',
    category: 'Nutrition',
    tags: ['nutrition', 'diet', 'health'],
    sample_prompts: ['What should I eat before a race?'],
    token_count: 800,
    install_count: 0,
    icon_url: null,
    published_at: null,
    author_id: 'author-2',
    author_email: 'nutrition@example.com',
    system_prompt: 'You are a nutrition advisor...',
    created_at: '2024-01-12T00:00:00Z',
    submitted_at: '2024-01-16T14:00:00Z',
    publish_status: 'pending_review',
  },
];

// Mock data for published coaches
const mockPublishedCoaches = [
  {
    id: 'published-coach-1',
    title: 'Recovery Expert',
    description: 'Optimize your recovery',
    category: 'Recovery',
    tags: ['recovery', 'rest'],
    sample_prompts: ['How should I recover?'],
    token_count: 600,
    install_count: 75,
    icon_url: null,
    published_at: '2024-01-15T00:00:00Z',
    author_id: 'author-3',
    author_email: 'recovery@example.com',
    system_prompt: 'You are a recovery expert...',
    created_at: '2024-01-08T00:00:00Z',
    publish_status: 'published',
  },
];

// Mock data for rejected coaches
const mockRejectedCoaches = [
  {
    id: 'rejected-coach-1',
    title: 'Low Quality Coach',
    description: 'This coach had issues',
    category: 'Training',
    tags: ['fitness'],
    sample_prompts: ['Sample prompt'],
    token_count: 200,
    install_count: 0,
    icon_url: null,
    published_at: null,
    author_id: 'author-4',
    author_email: 'bad@example.com',
    system_prompt: 'You are a coach...',
    created_at: '2024-01-05T00:00:00Z',
    rejected_at: '2024-01-12T00:00:00Z',
    rejection_reason: 'quality_standards',
    rejection_notes: 'System prompt lacks specificity.',
    publish_status: 'rejected',
  },
];

async function setupAdminStoreMocks(page: Page) {
  // Set up base dashboard mocks for admin
  await setupDashboardMocks(page, { role: 'admin' });

  // Mock store stats
  await page.route('**/api/admin/store/stats', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockStoreStats),
    });
  });

  // Mock review queue (dedicated endpoint, not query param)
  await page.route('**/api/admin/store/review-queue', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: mockPendingCoaches,
        total: mockPendingCoaches.length,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });

  // Mock published coaches
  await page.route('**/api/admin/store/coaches?status=published**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: mockPublishedCoaches,
        total: mockPublishedCoaches.length,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });

  // Mock rejected coaches
  await page.route('**/api/admin/store/coaches?status=rejected', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: mockRejectedCoaches,
        total: mockRejectedCoaches.length,
        metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
      }),
    });
  });

  // Mock approve endpoint
  await page.route('**/api/admin/store/coaches/*/approve', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        message: 'Coach approved successfully',
        coach_id: 'pending-coach-1',
      }),
    });
  });

  // Mock reject endpoint
  await page.route('**/api/admin/store/coaches/*/reject', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        message: 'Coach rejected successfully',
        coach_id: 'pending-coach-1',
      }),
    });
  });

  // Mock unpublish endpoint
  await page.route('**/api/admin/store/coaches/*/unpublish', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        message: 'Coach unpublished successfully',
        coach_id: 'published-coach-1',
      }),
    });
  });

  // Mock user coaches endpoint (required for sidebar)
  await page.route('**/api/coaches', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ coaches: [], total: 0 }),
    });
  });
}

test.describe('Admin Store Management Access', () => {
  test('displays Coach Store tab in admin sidebar', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Should see Coach Store tab in sidebar
    await expect(page.locator('nav').getByRole('button', { name: /Coach Store/i })).toBeVisible({ timeout: 5000 });
  });

  test('navigates to Coach Store Management when tab is clicked', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    // Should see store management header
    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Admin Store Stats Dashboard', () => {
  test('displays stats cards', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });

    // Should display stats cards (use specific selectors to avoid matching description text)
    await expect(page.locator('.text-sm.text-zinc-400').filter({ hasText: 'Pending Reviews' })).toBeVisible({ timeout: 10000 });
    await expect(page.locator('.text-sm.text-zinc-400').filter({ hasText: 'Published Coaches' })).toBeVisible();
    await expect(page.locator('.text-sm.text-zinc-400').filter({ hasText: 'Total Installs' })).toBeVisible();
    await expect(page.locator('.text-sm.text-zinc-400').filter({ hasText: 'Rejection Rate' })).toBeVisible();
  });

  test('displays correct stats values', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });

    // Should display correct values
    await expect(page.getByText('12')).toBeVisible({ timeout: 10000 }); // published_count
    await expect(page.getByText('150')).toBeVisible(); // total_installs
    await expect(page.getByText('20.0%')).toBeVisible(); // rejection_rate
  });
});

test.describe('Admin Review Queue', () => {
  test('displays pending coaches in review queue', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });

    // Review Queue should be the default tab
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Nutrition Advisor')).toBeVisible();
  });

  test('displays author emails in queue', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Should display author emails
    await expect(page.getByText('coach@example.com')).toBeVisible();
    await expect(page.getByText('nutrition@example.com')).toBeVisible();
  });

  test('displays token counts in queue', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Should display token counts
    await expect(page.getByText('1,200 tokens')).toBeVisible();
    await expect(page.getByText('800 tokens')).toBeVisible();
  });

  test('opens review drawer when coach is clicked', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click on the coach
    await page.getByText('Marathon Training Coach').click();

    // Should see review drawer
    await expect(page.getByRole('heading', { name: 'Review Coach' })).toBeVisible({ timeout: 5000 });
  });

  test('displays approve and reject buttons in drawer', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click on the coach
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('heading', { name: 'Review Coach' })).toBeVisible({ timeout: 5000 });

    // Should see approve and reject buttons
    await expect(page.getByRole('button', { name: /Approve/i })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Reject', exact: true })).toBeVisible();
  });

  test('shows empty state when no pending coaches', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    // Override with empty queue
    await page.route('**/api/admin/store/stats', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          pending_count: 0,
          published_count: 0,
          rejected_count: 0,
          total_installs: 0,
          rejection_rate: 0,
        }),
      });
    });

    await page.route('**/api/admin/store/coaches?status=pending_review', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          coaches: [],
          total: 0,
          metadata: { timestamp: new Date().toISOString(), api_version: '1.0' },
        }),
      });
    });

    await page.route('**/api/coaches', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ coaches: [], total: 0 }),
      });
    });

    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });

    // Should show empty state
    await expect(page.getByText('All Caught Up!')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Admin Coach Approval', () => {
  test('approves coach when Approve button is clicked', async ({ page }) => {
    await setupAdminStoreMocks(page);

    let approveCalled = false;
    await page.route('**/api/admin/store/coaches/*/approve', async (route) => {
      approveCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          message: 'Coach approved successfully',
          coach_id: 'pending-coach-1',
        }),
      });
    });

    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click on the coach to open drawer
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('heading', { name: 'Review Coach' })).toBeVisible({ timeout: 5000 });

    // Click Approve
    await page.getByRole('button', { name: /Approve/i }).click();

    await page.waitForTimeout(500);
    expect(approveCalled).toBe(true);
  });
});

test.describe('Admin Coach Rejection', () => {
  test('opens rejection modal when Reject button is clicked', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click on the coach to open drawer
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('heading', { name: 'Review Coach' })).toBeVisible({ timeout: 5000 });

    // Click Reject
    await page.getByRole('button', { name: 'Reject', exact: true }).click();

    // Should see rejection modal
    await expect(page.getByRole('heading', { name: 'Reject Coach' })).toBeVisible({ timeout: 5000 });
  });

  test('shows rejection reason dropdown', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('heading', { name: 'Review Coach' })).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: 'Reject', exact: true }).click();

    await expect(page.getByRole('heading', { name: 'Reject Coach' })).toBeVisible({ timeout: 5000 });

    // Should see reason dropdown with placeholder
    const combobox = page.getByRole('combobox');
    await expect(combobox).toBeVisible();
    // Verify the select has the placeholder option (value will be empty string)
    await expect(combobox).toHaveValue('');
  });

  test('rejects coach when reason selected and confirmed', async ({ page }) => {
    await setupAdminStoreMocks(page);

    let rejectCalled = false;
    await page.route('**/api/admin/store/coaches/*/reject', async (route) => {
      rejectCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          message: 'Coach rejected successfully',
          coach_id: 'pending-coach-1',
        }),
      });
    });

    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });
    await page.getByText('Marathon Training Coach').click();

    await expect(page.getByRole('heading', { name: 'Review Coach' })).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: 'Reject', exact: true }).click();

    await expect(page.getByRole('heading', { name: 'Reject Coach' })).toBeVisible({ timeout: 5000 });

    // Select a reason
    await page.getByRole('combobox').selectOption('quality_standards');

    // Click Reject Coach button (find the one with btn-primary class in the modal)
    const rejectButtons = await page.getByRole('button', { name: /Reject Coach/i }).all();
    const confirmButton = rejectButtons.find(async (btn) => {
      const className = await btn.getAttribute('class');
      return className?.includes('btn-primary');
    });
    if (confirmButton) {
      await confirmButton.click();
    } else {
      // Fallback: click the last Reject Coach button (the one in the modal)
      await rejectButtons[rejectButtons.length - 1].click();
    }

    await page.waitForTimeout(500);
    expect(rejectCalled).toBe(true);
  });
});

test.describe('Admin Published Coaches Tab', () => {
  test('switches to Published tab when clicked', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });

    // Click Published tab
    await page.getByRole('button', { name: /Published$/i }).click();

    await page.waitForTimeout(500);

    // Should display published coaches
    await expect(page.getByText('Recovery Expert')).toBeVisible({ timeout: 10000 });
  });

  test('displays install counts for published coaches', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /Published$/i }).click();

    await expect(page.getByText('Recovery Expert')).toBeVisible({ timeout: 10000 });

    // Should display install count
    await expect(page.getByText('75 installs')).toBeVisible();
  });

  test('has Unpublish button for published coaches', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /Published$/i }).click();

    await expect(page.getByText('Recovery Expert')).toBeVisible({ timeout: 10000 });

    // Should have Unpublish button
    await expect(page.getByRole('button', { name: /Unpublish/i })).toBeVisible();
  });
});

test.describe('Admin Rejected Coaches Tab', () => {
  test('switches to Rejected tab when clicked', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });

    // Click Rejected tab
    await page.getByRole('button', { name: /Rejected$/i }).click();

    await page.waitForTimeout(500);

    // Should display rejected coaches
    await expect(page.getByText('Low Quality Coach')).toBeVisible({ timeout: 10000 });
  });

  test('displays rejection reason for rejected coaches', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /Rejected$/i }).click();

    await expect(page.getByText('Low Quality Coach')).toBeVisible({ timeout: 10000 });

    // Should display rejection reason
    await expect(page.getByText('Quality standards not met')).toBeVisible();
  });

  test('displays rejection notes when present', async ({ page }) => {
    await setupAdminStoreMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });
    await navigateToTab(page, 'Coach Store');

    await expect(page.locator('h1').filter({ hasText: 'Coach Store' })).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /Rejected$/i }).click();

    await expect(page.getByText('Low Quality Coach')).toBeVisible({ timeout: 10000 });

    // Should display rejection notes
    await expect(page.getByText('System prompt lacks specificity.')).toBeVisible();
  });
});
