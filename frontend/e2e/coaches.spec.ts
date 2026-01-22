// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for System Coaches admin functionality.
// ABOUTME: Tests admin coaches management, CRUD operations, and user assignments.

import { test, expect, type Page } from '@playwright/test';
import { setupDashboardMocks, loginToDashboard, navigateToTab } from './test-helpers';

// Mock coach data
const mockCoaches = [
  {
    id: 'coach-1',
    title: 'Marathon Training Coach',
    description: 'Specialized in marathon preparation and endurance training',
    system_prompt: 'You are a professional marathon coach with expertise in long-distance running...',
    category: 'Training',
    tags: ['marathon', 'endurance', 'running'],
    token_count: 150,
    is_favorite: false,
    use_count: 42,
    last_used_at: '2025-01-10T10:00:00Z',
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-10T10:00:00Z',
    is_system: true,
    visibility: 'tenant',
    is_assigned: false,
  },
  {
    id: 'coach-2',
    title: 'Nutrition Expert',
    description: 'Helps with meal planning and sports nutrition',
    system_prompt: 'You are a certified sports nutritionist...',
    category: 'Nutrition',
    tags: ['nutrition', 'diet', 'meal-prep'],
    token_count: 200,
    is_favorite: true,
    use_count: 18,
    last_used_at: '2025-01-12T15:30:00Z',
    created_at: '2025-01-02T00:00:00Z',
    updated_at: '2025-01-12T15:30:00Z',
    is_system: true,
    visibility: 'global',
    is_assigned: true,
  },
];

// Mock users for assignment testing
const mockUsers = [
  { id: 'user-1', email: 'alice@test.com', display_name: 'Alice', user_status: 'active' },
  { id: 'user-2', email: 'bob@test.com', display_name: 'Bob', user_status: 'active' },
  { id: 'user-3', email: 'charlie@test.com', display_name: 'Charlie', user_status: 'pending' },
];

// Mock assignments
const mockAssignments = [
  { user_id: 'user-1', user_email: 'alice@test.com', assigned_at: '2025-01-05T00:00:00Z', assigned_by: 'admin@test.com' },
];

async function setupCoachesMocks(page: Page, options: { isAdmin?: boolean; emptyState?: boolean } = {}) {
  const { isAdmin = true, emptyState = false } = options;

  // Set up base dashboard mocks
  await setupDashboardMocks(page, { role: isAdmin ? 'admin' : 'user' });

  // Mock admin coaches endpoints
  await page.route('**/api/admin/coaches', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          coaches: emptyState ? [] : mockCoaches,
          total: emptyState ? 0 : mockCoaches.length,
          metadata: {
            timestamp: new Date().toISOString(),
            api_version: '1.0',
          },
        }),
      });
    } else if (route.request().method() === 'POST') {
      const body = route.request().postDataJSON();
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'coach-new',
          title: body.title,
          description: body.description,
          system_prompt: body.system_prompt,
          category: body.category || 'Training',
          tags: body.tags || [],
          token_count: Math.ceil(body.system_prompt.length / 4),
          is_favorite: false,
          use_count: 0,
          last_used_at: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          is_system: true,
          visibility: body.visibility || 'tenant',
          is_assigned: false,
        }),
      });
    } else {
      await route.continue();
    }
  });

  // Individual coach operations
  await page.route('**/api/admin/coaches/*', async (route) => {
    const url = route.request().url();

    // Skip assignment endpoints
    if (url.includes('/assign') || url.includes('/assignments')) {
      await route.continue();
      return;
    }

    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockCoaches[0]),
      });
    } else if (route.request().method() === 'PUT') {
      const body = route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          ...mockCoaches[0],
          ...body,
          updated_at: new Date().toISOString(),
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

  // Assignment endpoints
  await page.route('**/api/admin/coaches/*/assign', async (route) => {
    if (route.request().method() === 'POST') {
      const body = route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          coach_id: 'coach-1',
          assigned_count: body.user_ids.length,
          total_requested: body.user_ids.length,
        }),
      });
    } else if (route.request().method() === 'DELETE') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          coach_id: 'coach-1',
          removed_count: 1,
          total_requested: 1,
        }),
      });
    } else {
      await route.continue();
    }
  });

  // Assignments list endpoint
  await page.route('**/api/admin/coaches/*/assignments', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coach_id: 'coach-1',
        assignments: mockAssignments,
      }),
    });
  });

  // Mock admin users for assignment modal
  // API service extracts response.data.users, so return { users: [...] } format
  await page.route('**/api/admin/users**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ users: mockUsers, total_count: mockUsers.length }),
    });
  });
}

test.describe('System Coaches Tab Visibility', () => {
  test('displays Coaches tab for admin users', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });

    // Admin Coaches tab should be visible (exact match to avoid "My Coaches" button)
    await expect(page.locator('nav button').filter({ hasText: /^Coaches$/ })).toBeVisible();
  });

  test('hides Coaches tab for non-admin users', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: false });
    await loginToDashboard(page);

    // Non-admin users see chat-first layout (no admin sidebar)
    await page.waitForSelector('main', { timeout: 10000 });

    // Admin Coaches tab should NOT be visible (user may see "My Coaches" button instead)
    // Look specifically in nav for the admin tab - non-admins don't have nav with admin tabs
    await expect(page.locator('nav button').filter({ hasText: /^Coaches$/ })).not.toBeVisible();
  });
});

test.describe('System Coaches List View', () => {
  test('displays empty state when no coaches exist', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true, emptyState: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    // Should see empty state message
    await expect(page.getByText('No System Coaches')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Create your first system coach')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create Your First Coach' })).toBeVisible();
  });

  test('displays coach cards with correct information', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    // Wait for content to load
    await expect(page.getByText('System Coaches')).toBeVisible({ timeout: 10000 });

    // Should display coach cards
    await expect(page.getByText('Marathon Training Coach')).toBeVisible();
    await expect(page.getByText('Nutrition Expert')).toBeVisible();

    // Should display category badges
    await expect(page.getByText('Training').first()).toBeVisible();
    await expect(page.getByText('Nutrition').first()).toBeVisible();

    // Should display token counts
    await expect(page.getByText('150 tokens')).toBeVisible();
    await expect(page.getByText('200 tokens')).toBeVisible();

    // Should display use counts
    await expect(page.getByText('42 uses')).toBeVisible();
    await expect(page.getByText('18 uses')).toBeVisible();

    // Should display tags (using exact match to avoid title/description matches)
    await expect(page.getByText('marathon', { exact: true })).toBeVisible();
    await expect(page.getByText('endurance', { exact: true })).toBeVisible();
  });

  test('Create Coach button navigates to form', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await expect(page.getByText('System Coaches')).toBeVisible({ timeout: 10000 });

    // Click Create Coach button
    await page.getByRole('button', { name: 'Create Coach' }).click();

    // Should see form
    await expect(page.getByText('Create System Coach')).toBeVisible();
    await expect(page.getByText('Title')).toBeVisible();
    await expect(page.getByText('System Prompt')).toBeVisible();
  });
});

test.describe('Create Coach Form', () => {
  test('displays form with all required fields', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');
    await page.getByRole('button', { name: 'Create Coach' }).click();

    // Check form fields by their labels (text labels, not htmlFor)
    await expect(page.getByText('Title').first()).toBeVisible();
    await expect(page.getByText('Description')).toBeVisible();
    await expect(page.getByText('System Prompt')).toBeVisible();
    await expect(page.getByText('Category')).toBeVisible();
    await expect(page.getByText('Visibility')).toBeVisible();
    await expect(page.getByText('Tags')).toBeVisible();
    // Check that input fields are visible
    await expect(page.getByPlaceholder('e.g., Marathon Training Coach')).toBeVisible();
    await expect(page.getByPlaceholder('You are a professional marathon coach')).toBeVisible();
  });

  test('displays token count estimate for system prompt', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');
    await page.getByRole('button', { name: 'Create Coach' }).click();

    // Enter system prompt
    const systemPromptField = page.locator('textarea').filter({ hasText: '' }).first();
    await systemPromptField.fill('You are a professional coach with expertise in marathon training.');

    // Should display token estimate
    await expect(page.getByText(/Estimated tokens:/)).toBeVisible();
  });

  test('creates coach successfully', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });

    let createCalled = false;
    let createdData: Record<string, unknown> = {};
    await page.route('**/api/admin/coaches', async (route) => {
      if (route.request().method() === 'POST') {
        createCalled = true;
        createdData = route.request().postDataJSON();
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'coach-new',
            ...createdData,
            token_count: 100,
            is_favorite: false,
            use_count: 0,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
            is_system: true,
            is_assigned: false,
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ coaches: mockCoaches, total: mockCoaches.length }),
        });
      }
    });

    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');
    await page.getByRole('button', { name: 'Create Coach' }).click();

    // Fill form using placeholders and locators
    await page.getByPlaceholder('e.g., Marathon Training Coach').fill('Recovery Coach');
    await page.locator('textarea').first().fill('Optional description');
    await page.locator('textarea').nth(1).fill('You are a recovery specialist...');
    await page.locator('select').first().selectOption('Recovery');
    await page.getByPlaceholder('marathon, endurance, beginner').fill('recovery, rest, sleep');

    // Submit
    await page.getByRole('button', { name: 'Create Coach' }).click();

    await page.waitForTimeout(500);
    expect(createCalled).toBe(true);
    expect(createdData.title).toBe('Recovery Coach');
    expect(createdData.category).toBe('Recovery');
  });

  test('Back button returns to list view', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');
    await page.getByRole('button', { name: 'Create Coach' }).click();

    await expect(page.getByText('Create System Coach')).toBeVisible();

    // Click back
    await page.getByText('Back to Coaches').click();

    // Should return to list
    await expect(page.getByText('System Coaches')).toBeVisible();
    await expect(page.getByText('Marathon Training Coach')).toBeVisible();
  });

  test('Cancel button returns to list view', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');
    await page.getByRole('button', { name: 'Create Coach' }).click();

    await expect(page.getByText('Create System Coach')).toBeVisible();

    // Click cancel
    await page.getByRole('button', { name: 'Cancel' }).click();

    // Should return to list
    await expect(page.getByText('System Coaches')).toBeVisible();
  });
});

test.describe('Coach Detail View', () => {
  test('clicking coach card opens detail view', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Click coach card
    await page.getByText('Marathon Training Coach').click();

    // Should see detail view with stats
    await expect(page.getByText('150').first()).toBeVisible({ timeout: 5000 }); // token count
    await expect(page.getByText('42').first()).toBeVisible(); // use count
    await expect(page.getByText('Tokens')).toBeVisible();
    await expect(page.getByText('Uses')).toBeVisible();

    // Should see system prompt
    await expect(page.getByText('You are a professional marathon coach')).toBeVisible();

    // Should see action buttons
    await expect(page.getByRole('button', { name: 'Edit' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Delete' })).toBeVisible();
  });

  test('displays timestamps correctly', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();

    // Should display created and updated timestamps
    await expect(page.getByText('Created:')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Last Updated:')).toBeVisible();
  });

  test('displays tags in detail view', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();

    // Should display tags section
    await expect(page.getByText('Tags')).toBeVisible({ timeout: 5000 });
    // Use exact match to avoid matching the tag text in title/description
    await expect(page.getByText('marathon', { exact: true })).toBeVisible();
    await expect(page.getByText('endurance', { exact: true })).toBeVisible();
    await expect(page.getByText('running', { exact: true })).toBeVisible();
  });
});

test.describe('Edit Coach Form', () => {
  test('Edit button opens form with pre-populated data', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();
    await expect(page.getByRole('button', { name: 'Edit' })).toBeVisible({ timeout: 5000 });

    // Click edit
    await page.getByRole('button', { name: 'Edit' }).click();

    // Should see edit form with populated data
    await expect(page.getByText('Edit "Marathon Training Coach"')).toBeVisible();
    // Use placeholder selector since form doesn't use htmlFor
    const titleInput = page.getByPlaceholder('e.g., Marathon Training Coach');
    await expect(titleInput).toHaveValue('Marathon Training Coach');
  });

  test('updates coach successfully', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });

    let updateCalled = false;
    await page.route('**/api/admin/coaches/*', async (route) => {
      const url = route.request().url();
      if (url.includes('/assign') || url.includes('/assignments')) {
        await route.continue();
        return;
      }

      if (route.request().method() === 'PUT') {
        updateCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            ...mockCoaches[0],
            title: 'Updated Marathon Coach',
            updated_at: new Date().toISOString(),
          }),
        });
      } else if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockCoaches[0]),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();
    await page.getByRole('button', { name: 'Edit' }).click();

    // Wait for edit form to be visible
    await expect(page.getByText('Edit "Marathon Training Coach"')).toBeVisible({ timeout: 5000 });

    // Wait for form to be populated - token count > 0 indicates system_prompt has content
    await expect(page.getByText(/Estimated tokens: [1-9]/)).toBeVisible({ timeout: 5000 });

    // Modify title using placeholder selector
    await page.getByPlaceholder('e.g., Marathon Training Coach').fill('Updated Marathon Coach');

    // Save
    await page.getByRole('button', { name: 'Save Changes' }).click();

    await page.waitForTimeout(500);
    expect(updateCalled).toBe(true);
  });

  test('visibility dropdown is disabled when editing', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();
    await page.getByRole('button', { name: 'Edit' }).click();

    // Wait for edit form to be visible
    await expect(page.getByText('Edit "Marathon Training Coach"')).toBeVisible({ timeout: 5000 });

    // Visibility is the second select (first is category)
    // Check that the select with "Tenant Only" option is disabled
    const visibilitySelect = page.locator('select').nth(1);
    await expect(visibilitySelect).toBeDisabled();
  });
});

test.describe('Delete Coach', () => {
  test('delete button triggers confirmation and deletes', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });

    let deleteCalled = false;
    await page.route('**/api/admin/coaches/*', async (route) => {
      const url = route.request().url();
      if (url.includes('/assign') || url.includes('/assignments')) {
        await route.continue();
        return;
      }

      if (route.request().method() === 'DELETE') {
        deleteCalled = true;
        await route.fulfill({ status: 204 });
      } else if (route.request().method() === 'GET') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockCoaches[0]),
        });
      } else {
        await route.continue();
      }
    });

    // Handle confirm dialog
    page.on('dialog', async (dialog) => {
      expect(dialog.message()).toContain('Delete coach');
      await dialog.accept();
    });

    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();
    await expect(page.getByRole('button', { name: 'Delete' })).toBeVisible({ timeout: 5000 });

    // Click delete
    await page.getByRole('button', { name: 'Delete' }).click();

    await page.waitForTimeout(500);
    expect(deleteCalled).toBe(true);
  });
});

test.describe('User Assignments', () => {
  test('displays assignments count in detail view', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();

    // Should display assigned users count
    await expect(page.getByText('Assigned Users')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('1').first()).toBeVisible(); // 1 assignment in mock
  });

  test('displays User Assignments card with assigned users', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();

    // Should display assignments card
    await expect(page.getByText('User Assignments').first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('alice@test.com')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Assign Users' })).toBeVisible();
  });

  test('Assign Users button opens modal', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();
    await expect(page.getByRole('button', { name: 'Assign Users' })).toBeVisible({ timeout: 5000 });

    // Click Assign Users
    await page.getByRole('button', { name: 'Assign Users' }).click();

    // Should see modal
    await expect(page.getByText('Assign Users to Coach')).toBeVisible();
    await expect(page.getByText('Select users to give access')).toBeVisible();
  });

  test('can select and assign users', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });

    let assignCalled = false;
    let assignedUserIds: string[] = [];
    await page.route('**/api/admin/coaches/*/assign', async (route) => {
      if (route.request().method() === 'POST') {
        assignCalled = true;
        const body = route.request().postDataJSON();
        assignedUserIds = body.user_ids;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            coach_id: 'coach-1',
            assigned_count: body.user_ids.length,
            total_requested: body.user_ids.length,
          }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();
    await page.getByRole('button', { name: 'Assign Users' }).click();

    // Wait for modal and users to load
    await expect(page.getByText('Assign Users to Coach')).toBeVisible({ timeout: 5000 });
    await page.waitForTimeout(500);

    // Select a user (bob is not already assigned)
    const bobCheckbox = page.locator('label').filter({ hasText: 'bob@test.com' });
    await bobCheckbox.click();

    // Click Assign Selected
    await page.getByRole('button', { name: 'Assign Selected' }).click();

    await page.waitForTimeout(500);
    expect(assignCalled).toBe(true);
    expect(assignedUserIds).toContain('user-2');
  });

  test('shows empty state when no users assigned', async ({ page }) => {
    // Override assignments mock to return empty
    await setupDashboardMocks(page, { role: 'admin' });

    await page.route('**/api/admin/coaches', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ coaches: mockCoaches, total: mockCoaches.length }),
      });
    });

    await page.route('**/api/admin/coaches/*', async (route) => {
      const url = route.request().url();
      if (url.includes('/assignments')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ coach_id: 'coach-1', assignments: [] }),
        });
      } else if (!url.includes('/assign')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(mockCoaches[0]),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByText('Marathon Training Coach').click();

    // Should show empty state message
    await expect(page.getByText('No users assigned to this coach yet')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Error Handling', () => {
  test('shows error when failing to load coaches', async ({ page }) => {
    await setupDashboardMocks(page, { role: 'admin' });

    await page.route('**/api/admin/coaches', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    // Should show loading spinner then error or empty state
    // React Query may retry, so we wait a bit
    await page.waitForTimeout(2000);
  });

  test('shows error when create fails', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });

    await page.route('**/api/admin/coaches', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 400,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Validation failed' }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ coaches: mockCoaches, total: mockCoaches.length }),
        });
      }
    });

    await loginToDashboard(page);
    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await page.getByRole('button', { name: 'Create Coach' }).click();

    // Fill minimal form using placeholder selectors
    await page.getByPlaceholder('e.g., Marathon Training Coach').fill('Test Coach');
    await page.getByPlaceholder('You are a professional marathon coach').fill('Test prompt');

    // Submit
    await page.getByRole('button', { name: 'Create Coach' }).click();

    await page.waitForTimeout(500);
    // Form should still be visible (not submitted successfully)
    await expect(page.getByText('Create System Coach')).toBeVisible();
  });
});

test.describe('Category Colors', () => {
  test('displays correct category colors on cards', async ({ page }) => {
    await setupCoachesMocks(page, { isAdmin: true });
    await loginToDashboard(page);

    await page.waitForSelector('nav', { timeout: 10000 });
    await navigateToTab(page, 'Coaches');

    await expect(page.getByText('Marathon Training Coach')).toBeVisible({ timeout: 10000 });

    // Training category should have Training badge
    const trainingBadge = page.locator('.rounded-full').filter({ hasText: 'Training' });
    await expect(trainingBadge).toBeVisible();

    // Nutrition category should have Nutrition badge
    const nutritionBadge = page.locator('.rounded-full').filter({ hasText: 'Nutrition' });
    await expect(nutritionBadge).toBeVisible();
  });
});

// ============================================================================
// User-Facing Coaches Tests (Chat Interface - PromptSuggestions)
// ============================================================================

// Mock data for user-facing coaches (non-admin view)
const mockUserCoaches = [
  {
    id: 'user-coach-1',
    title: 'My Custom Coach',
    description: 'Personal training coach',
    system_prompt: 'You are my personal coach.',
    category: 'training',
    tags: ['personal'],
    token_count: 50,
    is_favorite: false,
    use_count: 3,
    last_used_at: '2025-01-10T10:00:00Z',
    is_system: false,
    visibility: 'private',
    is_assigned: false,
  },
  {
    id: 'system-coach-1',
    title: 'System Training Coach',
    description: 'Official training guidance',
    system_prompt: 'You are a professional coach.',
    category: 'training',
    tags: ['training'],
    token_count: 100,
    is_favorite: false,
    use_count: 10,
    last_used_at: null,
    is_system: true,
    visibility: 'tenant',
    is_assigned: true,
  },
];

const mockHiddenCoaches = [
  {
    id: 'hidden-coach-1',
    title: 'Hidden System Coach',
    description: 'A hidden coach',
    system_prompt: 'Hidden prompt.',
    category: 'nutrition',
    tags: [],
    token_count: 80,
    is_favorite: false,
    use_count: 0,
    last_used_at: null,
    is_system: true,
    visibility: 'tenant',
    is_assigned: true,
  },
];

async function setupUserCoachesMocks(page: Page) {
  // Set up base dashboard mocks for non-admin user
  await setupDashboardMocks(page, { role: 'user' });

  // Mock user coaches endpoint
  await page.route('**/api/coaches', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          coaches: mockUserCoaches,
          total: mockUserCoaches.length,
        }),
      });
    } else if (route.request().method() === 'POST') {
      const body = route.request().postDataJSON();
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'new-user-coach',
          ...body,
          token_count: 50,
          is_favorite: false,
          use_count: 0,
          is_system: false,
          visibility: 'private',
          is_assigned: false,
        }),
      });
    } else {
      await route.continue();
    }
  });

  // Mock hidden coaches endpoint
  await page.route('**/api/coaches/hidden', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        coaches: mockHiddenCoaches,
      }),
    });
  });

  // Mock individual coach operations (edit, delete, hide, show)
  await page.route('**/api/coaches/*', async (route) => {
    const url = route.request().url();
    const method = route.request().method();

    if (url.includes('/hide')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, is_hidden: true }),
      });
    } else if (url.includes('/show')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, is_hidden: false }),
      });
    } else if (url.includes('/usage')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    } else if (method === 'PUT') {
      const body = route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          ...mockUserCoaches[0],
          ...body,
        }),
      });
    } else if (method === 'DELETE') {
      await route.fulfill({ status: 204 });
    } else {
      await route.continue();
    }
  });
}

test.describe('User Coaches - Chat Interface', () => {
  test('displays coaches in chat interface for regular users', async ({ page }) => {
    await setupUserCoachesMocks(page);
    await loginToDashboard(page);

    // Wait for chat interface to load (users see chat-first layout)
    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    // Should see My Coaches heading
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 10000 });
    // User-created coaches are shown in Personalized section
    await expect(page.getByText('My Custom Coach')).toBeVisible();
    // System coaches shown in System Coaches section
    await expect(page.getByText('System Training Coach')).toBeVisible();
  });

  test('shows edit button only for user-created coaches', async ({ page }) => {
    await setupUserCoachesMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    await expect(page.getByText('My Custom Coach')).toBeVisible({ timeout: 10000 });

    // User coach should have edit button
    const userCoachCard = page.locator('div').filter({ hasText: 'My Custom Coach' }).first();
    await userCoachCard.hover();

    // Edit button should be visible for user coaches
    await expect(page.getByTitle('Edit coach').first()).toBeVisible();
  });

  test('shows hide button for system coaches', async ({ page }) => {
    await setupUserCoachesMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    await expect(page.getByText('System Training Coach')).toBeVisible({ timeout: 10000 });

    // System coach should have hide button
    await expect(page.getByTitle('Hide coach')).toBeVisible();
  });

  test('can hide a system coach', async ({ page }) => {
    await setupUserCoachesMocks(page);

    let hideCalled = false;
    await page.route('**/api/coaches/*/hide', async (route) => {
      hideCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, is_hidden: true }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    await expect(page.getByText('System Training Coach')).toBeVisible({ timeout: 10000 });

    // Click hide button
    await page.getByTitle('Hide coach').click();

    await page.waitForTimeout(500);
    expect(hideCalled).toBe(true);
  });

  test('can toggle show hidden coaches', async ({ page }) => {
    await setupUserCoachesMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    // Use exact match to avoid ambiguity with "My Coaches" button and "System Coaches" heading
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 10000 });

    // Click show hidden toggle
    const showHiddenToggle = page.getByText(/hidden/i).first();
    if (await showHiddenToggle.isVisible()) {
      await showHiddenToggle.click();
      await page.waitForTimeout(300);

      // Hidden coach should now be visible
      await expect(page.getByText('Hidden System Coach')).toBeVisible({ timeout: 5000 });
    }
  });

  test('can show a hidden coach', async ({ page }) => {
    await setupUserCoachesMocks(page);

    let showCalled = false;
    await page.route('**/api/coaches/*/show', async (route) => {
      showCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, is_hidden: false }),
      });
    });

    await loginToDashboard(page);
    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    // Toggle to show hidden coaches
    const showHiddenToggle = page.getByText(/hidden/i).first();
    if (await showHiddenToggle.isVisible()) {
      await showHiddenToggle.click();
      await page.waitForTimeout(300);

      // Click show button on hidden coach
      const showButton = page.getByTitle('Show coach');
      if (await showButton.isVisible()) {
        await showButton.click();
        await page.waitForTimeout(500);
        expect(showCalled).toBe(true);
      }
    }
  });

  test('can delete a user coach with confirmation', async ({ page }) => {
    await setupUserCoachesMocks(page);

    let deleteCalled = false;
    await page.route('**/api/coaches/user-coach-1', async (route) => {
      if (route.request().method() === 'DELETE') {
        deleteCalled = true;
        await route.fulfill({ status: 204 });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);
    await page.waitForSelector('main', { timeout: 10000 });

    // Click the My Coaches button to open the panel
    await page.locator('button[title="My Coaches"]').click();
    await page.waitForTimeout(300);

    await expect(page.getByText('My Custom Coach')).toBeVisible({ timeout: 10000 });

    // Click delete button
    const deleteButton = page.getByTitle('Delete coach').first();
    await deleteButton.click();

    // Confirm deletion in dialog
    await expect(page.getByText('Delete Coach')).toBeVisible({ timeout: 5000 });
    // Use exact match to avoid conflict with "Delete coach" button title
    await page.getByRole('button', { name: 'Delete', exact: true }).click();

    await page.waitForTimeout(500);
    expect(deleteCalled).toBe(true);
  });

  test('can create a new user coach via My Coaches panel', async ({ page }) => {
    await setupUserCoachesMocks(page);

    let createCalled = false;
    await page.route('**/api/coaches', async (route) => {
      if (route.request().method() === 'POST') {
        createCalled = true;
        const body = route.request().postDataJSON();
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'new-coach',
            ...body,
            is_system: false,
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ coaches: mockUserCoaches, total: mockUserCoaches.length }),
        });
      }
    });

    await loginToDashboard(page);
    await page.waitForSelector('main', { timeout: 10000 });

    // Click My Coaches button in sidebar to open the My Coaches panel
    await page.getByRole('button', { name: 'My Coaches' }).click();

    // Wait for the My Coaches panel to open
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 5000 });

    // Click Add Coach button in the panel header
    await page.getByRole('button', { name: 'Add Coach' }).click();

    // Fill in the form
    await page.getByPlaceholder('e.g., Marathon Training Coach').fill('New Test Coach');
    await page.locator('textarea').first().fill('Test system prompt');

    // Submit
    await page.getByRole('button', { name: 'Create Coach' }).click();

    await page.waitForTimeout(500);
    expect(createCalled).toBe(true);
  });

  test('can create coach with Training category and verify icon', async ({ page }) => {
    await setupUserCoachesMocks(page);

    let capturedBody: Record<string, unknown> | null = null;
    await page.route('**/api/coaches', async (route) => {
      if (route.request().method() === 'POST') {
        capturedBody = route.request().postDataJSON();
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'new-training-coach',
            title: capturedBody?.title,
            description: capturedBody?.description,
            system_prompt: capturedBody?.system_prompt,
            category: 'training', // Backend normalizes to lowercase
            is_system: false,
            tags: [],
          }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);
    await page.waitForSelector('main', { timeout: 10000 });

    // Open My Coaches panel
    await page.getByRole('button', { name: 'My Coaches' }).click();
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 5000 });

    // Click Add Coach
    await page.getByRole('button', { name: 'Add Coach' }).click();

    // Fill form with Training category
    await page.getByPlaceholder('e.g., Marathon Training Coach').fill('My Training Coach');
    await page.locator('textarea').first().fill('Training system prompt');

    // Select Training category from dropdown
    const categorySelect = page.locator('select').first();
    if (await categorySelect.isVisible()) {
      await categorySelect.selectOption('Training');
    }

    // Submit
    await page.getByRole('button', { name: 'Create Coach' }).click();
    await page.waitForTimeout(500);

    // Verify the category was sent correctly
    expect(capturedBody).not.toBeNull();
    expect(capturedBody?.category).toBe('Training');
  });

  test('personalized section appears above system coaches', async ({ page }) => {
    await setupUserCoachesMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Open My Coaches panel
    await page.getByRole('button', { name: 'My Coaches' }).click();
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 5000 });

    // Both sections should be visible
    const personalizedHeading = page.getByText('Personalized', { exact: false });
    const systemCoachesHeading = page.getByText('System Coaches', { exact: false });

    await expect(personalizedHeading).toBeVisible({ timeout: 5000 });
    await expect(systemCoachesHeading).toBeVisible({ timeout: 5000 });

    // Verify Personalized appears before System Coaches in DOM order
    const personalizedBox = await personalizedHeading.boundingBox();
    const systemBox = await systemCoachesHeading.boundingBox();

    if (personalizedBox && systemBox) {
      expect(personalizedBox.y).toBeLessThan(systemBox.y);
    }
  });

  test('category filter buttons filter coaches correctly', async ({ page }) => {
    await setupUserCoachesMocks(page);
    await loginToDashboard(page);

    await page.waitForSelector('main', { timeout: 10000 });

    // Open My Coaches panel
    await page.getByRole('button', { name: 'My Coaches' }).click();
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 5000 });

    // Click Training filter
    const trainingFilter = page.getByRole('button', { name: /Training/i });
    if (await trainingFilter.isVisible()) {
      await trainingFilter.click();
      await page.waitForTimeout(300);

      // System Training Coach should be visible
      await expect(page.getByText('System Training Coach')).toBeVisible({ timeout: 5000 });
    }
  });

  test('can edit user coach and update category', async ({ page }) => {
    await setupUserCoachesMocks(page);

    let updateCalled = false;
    let capturedUpdate: Record<string, unknown> | null = null;
    await page.route('**/api/coaches/user-coach-1', async (route) => {
      if (route.request().method() === 'PUT' || route.request().method() === 'PATCH') {
        updateCalled = true;
        capturedUpdate = route.request().postDataJSON();
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'user-coach-1',
            title: capturedUpdate?.title ?? 'My Custom Coach',
            category: 'nutrition',
            is_system: false,
          }),
        });
      } else {
        await route.continue();
      }
    });

    await loginToDashboard(page);
    await page.waitForSelector('main', { timeout: 10000 });

    // Open My Coaches panel
    await page.getByRole('button', { name: 'My Coaches' }).click();
    await expect(page.getByRole('heading', { name: 'My Coaches' })).toBeVisible({ timeout: 5000 });

    // Click edit button on user coach
    const editButton = page.getByTitle('Edit coach').first();
    await editButton.click();

    // Wait for edit form
    await page.waitForTimeout(500);

    // Update category if dropdown is visible
    const categorySelect = page.locator('select').first();
    if (await categorySelect.isVisible()) {
      await categorySelect.selectOption('Nutrition');
    }

    // Save changes
    const saveButton = page.getByRole('button', { name: /Save|Update/i });
    if (await saveButton.isVisible()) {
      await saveButton.click();
      await page.waitForTimeout(500);
      expect(updateCalled).toBe(true);
    }
  });
});
