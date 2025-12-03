// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright E2E tests for user registration flow.
// ABOUTME: Tests form validation, successful registration, error handling, and navigation.

import { test, expect } from '@playwright/test';

// Helper to set up common API mocks for the registration page
async function setupBasicMocks(page: import('@playwright/test').Page) {
  // Mock setup status
  await page.route('**/admin/setup/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        needs_setup: false,
        admin_user_exists: true,
        message: 'Admin user configured',
      }),
    });
  });
}

// Helper to navigate to registration page from login
async function navigateToRegistration(page: import('@playwright/test').Page) {
  await setupBasicMocks(page);
  await page.goto('/');
  await page.waitForSelector('form', { timeout: 10000 });

  // Click the "Don't have an account? Create one" link to show registration form
  await page.getByText("Don't have an account?").click();

  // Wait for registration form to appear
  await page.waitForSelector('input[name="displayName"]', { timeout: 5000 });
}

test.describe('Registration Page - Form Display', () => {
  test('renders registration form with all expected elements', async ({ page }) => {
    await navigateToRegistration(page);

    // Check for main heading
    const heading = page.locator('h1');
    await expect(heading).toContainText('Create Your Account');

    // Check for form elements
    await expect(page.locator('input[name="displayName"]')).toBeVisible();
    await expect(page.locator('input[name="email"]')).toBeVisible();
    await expect(page.locator('input[name="password"]')).toBeVisible();
    await expect(page.locator('input[name="confirmPassword"]')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create Account' })).toBeVisible();

    // Check for link back to login
    await expect(page.getByText('Already have an account?')).toBeVisible();
  });

  test('allows typing in all form fields', async ({ page }) => {
    await navigateToRegistration(page);

    const displayNameInput = page.locator('input[name="displayName"]');
    const emailInput = page.locator('input[name="email"]');
    const passwordInput = page.locator('input[name="password"]');
    const confirmPasswordInput = page.locator('input[name="confirmPassword"]');

    await displayNameInput.fill('Test User');
    await emailInput.fill('newuser@test.com');
    await passwordInput.fill('SecurePassword123');
    await confirmPasswordInput.fill('SecurePassword123');

    await expect(displayNameInput).toHaveValue('Test User');
    await expect(emailInput).toHaveValue('newuser@test.com');
    await expect(passwordInput).toHaveValue('SecurePassword123');
    await expect(confirmPasswordInput).toHaveValue('SecurePassword123');
  });

  test('toggles password visibility when clicking eye icon', async ({ page }) => {
    await navigateToRegistration(page);

    const passwordInput = page.locator('input[name="password"]');
    await passwordInput.fill('SecurePassword123');

    // Password should be hidden by default
    await expect(passwordInput).toHaveAttribute('type', 'password');

    // Click the toggle button
    const toggleButton = page.locator('button[aria-label="Show password"]').first();
    await toggleButton.click();

    // Password should now be visible
    await expect(passwordInput).toHaveAttribute('type', 'text');
  });

  test('navigates back to login when clicking sign in link', async ({ page }) => {
    await navigateToRegistration(page);

    // Click the sign in link
    await page.getByText('Already have an account?').click();

    // Should see login form
    await expect(page.locator('h1')).toContainText('Pierre Fitness Platform');
    await expect(page.locator('input[name="displayName"]')).not.toBeVisible();
  });
});

test.describe('Registration Page - Form Validation', () => {
  test('shows error when passwords do not match', async ({ page }) => {
    await navigateToRegistration(page);

    await page.locator('input[name="email"]').fill('newuser@test.com');
    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('DifferentPassword');

    await page.getByRole('button', { name: 'Create Account' }).click();

    await expect(page.getByText('Passwords do not match')).toBeVisible();
  });

  test('shows error when password is too short', async ({ page }) => {
    await navigateToRegistration(page);

    await page.locator('input[name="email"]').fill('newuser@test.com');
    await page.locator('input[name="password"]').fill('short');
    await page.locator('input[name="confirmPassword"]').fill('short');

    await page.getByRole('button', { name: 'Create Account' }).click();

    await expect(page.getByText('Password must be at least 8 characters')).toBeVisible();
  });

  test('validates required email field', async ({ page }) => {
    await navigateToRegistration(page);

    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('SecurePassword123');

    // Try to submit without email - HTML5 validation should prevent
    const submitButton = page.getByRole('button', { name: 'Create Account' });
    await submitButton.click();

    // Form should still be visible
    await expect(page.locator('input[name="email"]')).toBeVisible();
  });
});

test.describe('Registration Page - API Interaction', () => {
  test('shows loading state during form submission', async ({ page }) => {
    await navigateToRegistration(page);

    // Mock a slow registration response
    await page.route('**/api/auth/register', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 500));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          message: 'Registration successful. Awaiting admin approval.',
        }),
      });
    });

    await page.locator('input[name="email"]').fill('newuser@test.com');
    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('SecurePassword123');

    await page.getByRole('button', { name: 'Create Account' }).click();

    // Should show loading text
    await expect(page.getByRole('button', { name: 'Creating account...' })).toBeVisible();
  });

  test('successful registration shows success message', async ({ page }) => {
    await navigateToRegistration(page);

    // Mock successful registration
    await page.route('**/api/auth/register', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          message: 'Registration successful. Awaiting admin approval.',
        }),
      });
    });

    await page.locator('input[name="displayName"]').fill('Test User');
    await page.locator('input[name="email"]').fill('newuser@test.com');
    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('SecurePassword123');

    await page.getByRole('button', { name: 'Create Account' }).click();

    // Should show success message (on login page after redirect)
    await expect(page.getByText('Awaiting admin approval')).toBeVisible({ timeout: 5000 });
  });

  test('displays error message on registration failure', async ({ page }) => {
    await navigateToRegistration(page);

    // Mock failed registration - email already exists
    await page.route('**/api/auth/register', async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Email address is already registered',
        }),
      });
    });

    await page.locator('input[name="email"]').fill('existing@test.com');
    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('SecurePassword123');

    await page.getByRole('button', { name: 'Create Account' }).click();

    await expect(page.getByText('Email address is already registered')).toBeVisible();
  });

  test('displays generic error for network failures', async ({ page }) => {
    await navigateToRegistration(page);

    // Mock network error
    await page.route('**/api/auth/register', async (route) => {
      await route.abort('failed');
    });

    await page.locator('input[name="email"]').fill('newuser@test.com');
    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('SecurePassword123');

    await page.getByRole('button', { name: 'Create Account' }).click();

    await expect(page.getByText('Registration failed')).toBeVisible();
  });
});

test.describe('Registration Page - Accessibility', () => {
  test('form fields have proper labels', async ({ page }) => {
    await navigateToRegistration(page);

    // Check that inputs have proper name attributes
    await expect(page.locator('input[name="displayName"]')).toHaveAttribute('id', 'displayName');
    await expect(page.locator('input[name="email"]')).toHaveAttribute('id', 'email');
    await expect(page.locator('input[name="password"]')).toHaveAttribute('id', 'password');
    await expect(page.locator('input[name="confirmPassword"]')).toHaveAttribute('id', 'confirmPassword');
  });

  test('form can be navigated with keyboard', async ({ page }) => {
    await navigateToRegistration(page);

    // Focus on display name field
    const displayNameInput = page.locator('input[name="displayName"]');
    await displayNameInput.focus();
    await expect(displayNameInput).toBeFocused();

    // Tab through fields
    await page.keyboard.press('Tab');
    await expect(page.locator('input[name="email"]')).toBeFocused();

    await page.keyboard.press('Tab');
    await expect(page.locator('input[name="password"]')).toBeFocused();
  });

  test('submit button shows disabled state when loading', async ({ page }) => {
    await navigateToRegistration(page);

    await page.route('**/api/auth/register', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 3000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          message: 'Registration successful.',
        }),
      });
    });

    await page.locator('input[name="email"]').fill('newuser@test.com');
    await page.locator('input[name="password"]').fill('SecurePassword123');
    await page.locator('input[name="confirmPassword"]').fill('SecurePassword123');

    await page.getByRole('button', { name: 'Create Account' }).click();

    // Button should be disabled during loading
    await expect(page.getByRole('button', { name: 'Creating account...' })).toBeDisabled();
  });
});
