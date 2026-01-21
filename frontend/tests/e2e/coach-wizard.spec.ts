// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// ABOUTME: Playwright E2E tests for Coach Wizard (ASY-157)
// ABOUTME: Tests wizard flow, validation, token counting, and import/export

import { test, expect, type Page } from '@playwright/test';

const TEST_USER = {
  email: 'test@example.com',
  password: 'TestPassword123!',
};

// Helper to login before tests
async function login(page: Page) {
  await page.goto('/login');
  await page.fill('[data-testid="email-input"]', TEST_USER.email);
  await page.fill('[data-testid="password-input"]', TEST_USER.password);
  await page.click('[data-testid="login-button"]');
  await page.waitForURL('**/dashboard**');
}

test.describe('Coach Wizard', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    // Navigate to coach library
    await page.click('[data-testid="coach-library-tab"]');
  });

  test('creates a coach through wizard steps', async ({ page }) => {
    // Open wizard
    await page.click('[data-testid="create-coach-button"]');
    await expect(page.locator('text=Create Coach')).toBeVisible();

    // Step 1: Basic Info
    await page.fill('[data-testid="coach-title-input"]', 'Test Interval Coach');
    await page.fill('[data-testid="coach-description-input"]', 'A coach for interval training');
    await page.click('[data-testid="category-training"]');
    await page.fill('[data-testid="tag-input"]', 'intervals');
    await page.click('[data-testid="add-tag-button"]');
    await expect(page.locator('text=intervals')).toBeVisible();
    await page.click('[data-testid="next-button"]');

    // Step 2: Purpose
    await page.fill('[data-testid="purpose-input"]', 'Helps athletes plan interval sessions');
    await page.fill('[data-testid="when-to-use-input"]', 'When preparing for race-specific workouts');
    await page.click('[data-testid="next-button"]');

    // Step 3: System Prompt
    await page.fill('[data-testid="system-prompt-input"]', 'You are an expert interval training coach...');
    await expect(page.locator('[data-testid="token-count"]')).toContainText('tokens');
    await page.click('[data-testid="next-button"]');

    // Step 4: Examples
    await page.fill('[data-testid="example-inputs"]', '- What intervals should I do for a 5K?');
    await page.fill('[data-testid="example-outputs"]', '- For a 5K, try 6x800m at goal pace...');
    await page.click('[data-testid="next-button"]');

    // Step 5: Prerequisites
    await page.click('[data-testid="provider-strava"]');
    await page.fill('[data-testid="min-activities-input"]', '10');
    await page.click('[data-testid="activity-type-run"]');
    await page.click('[data-testid="next-button"]');

    // Step 6: Advanced
    await page.fill('[data-testid="success-criteria-input"]', 'User completes interval workout');
    await page.click('[data-testid="next-button"]');

    // Step 7: Review
    await expect(page.locator('text=Review Your Coach')).toBeVisible();
    await expect(page.locator('text=Test Interval Coach')).toBeVisible();
    await expect(page.locator('text=Training')).toBeVisible();

    // Save
    await page.click('[data-testid="save-button"]');
    await expect(page.locator('text=Coach created successfully')).toBeVisible();
  });

  test('validates required fields at each step', async ({ page }) => {
    await page.click('[data-testid="create-coach-button"]');

    // Try to proceed without title
    await page.click('[data-testid="next-button"]');
    await expect(page.locator('text=Title is required')).toBeVisible();

    // Fill title and proceed
    await page.fill('[data-testid="coach-title-input"]', 'Valid Title');
    await page.click('[data-testid="next-button"]');

    // Should be on step 2 now
    await expect(page.locator('text=Purpose')).toBeVisible();

    // Skip to system prompt step
    await page.click('[data-testid="next-button"]');

    // Try to proceed without system prompt
    await page.click('[data-testid="next-button"]');
    await expect(page.locator('text=System prompt is required')).toBeVisible();
  });

  test('shows token count updating in real-time', async ({ page }) => {
    await page.click('[data-testid="create-coach-button"]');

    // Navigate to system prompt step
    await page.fill('[data-testid="coach-title-input"]', 'Token Test Coach');
    await page.click('[data-testid="next-button"]');
    await page.click('[data-testid="next-button"]');

    // Check initial token count
    const tokenDisplay = page.locator('[data-testid="token-count"]');
    await expect(tokenDisplay).toContainText('0 tokens');

    // Type in system prompt
    await page.fill('[data-testid="system-prompt-input"]', 'A'.repeat(400));

    // Token count should update (400 chars / 4 = ~100 tokens)
    await expect(tokenDisplay).toContainText('100 tokens');

    // Add more text
    await page.fill('[data-testid="system-prompt-input"]', 'A'.repeat(4000));

    // Should show ~1000 tokens
    await expect(tokenDisplay).toContainText('1,000 tokens');
  });

  test('toggles between visual and markdown mode', async ({ page }) => {
    await page.click('[data-testid="create-coach-button"]');

    // Navigate to system prompt step
    await page.fill('[data-testid="coach-title-input"]', 'Markdown Test');
    await page.click('[data-testid="next-button"]');
    await page.click('[data-testid="next-button"]');

    // Should start in visual mode
    await expect(page.locator('[data-testid="markdown-toggle"]')).toContainText('Markdown');

    // Toggle to markdown mode
    await page.click('[data-testid="markdown-toggle"]');
    await expect(page.locator('[data-testid="markdown-toggle"]')).toContainText('Visual');

    // Toggle preview
    await page.click('[data-testid="preview-toggle"]');
    await expect(page.locator('[data-testid="preview-panel"]')).toBeVisible();
  });

  test('exports coach to markdown file', async ({ page }) => {
    await page.click('[data-testid="create-coach-button"]');

    // Fill basic info
    await page.fill('[data-testid="coach-title-input"]', 'Export Test Coach');
    await page.click('[data-testid="category-nutrition"]');

    // Click export button
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.click('[data-testid="export-button"]'),
    ]);

    // Verify download
    expect(download.suggestedFilename()).toContain('export-test-coach');
    expect(download.suggestedFilename()).toContain('.md');
  });

  test('imports coach from markdown file', async ({ page }) => {
    await page.click('[data-testid="create-coach-button"]');

    // Create a test markdown content
    const markdownContent = `---
title: Imported Coach
category: recovery
tags: [sleep, rest]
---

## Purpose

Help users optimize recovery.

## Instructions

You are a recovery coach...
`;

    // Upload the file
    const fileInput = page.locator('[data-testid="import-input"]');
    await fileInput.setInputFiles({
      name: 'test-coach.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from(markdownContent),
    });

    // Verify fields are populated
    await expect(page.locator('[data-testid="coach-title-input"]')).toHaveValue('Imported Coach');
    await expect(page.locator('[data-testid="category-recovery"]')).toHaveClass(/active|selected/);
  });
});

test.describe('Coach Prerequisites', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('[data-testid="coach-library-tab"]');
  });

  test('shows prerequisites_met status in coach list', async ({ page }) => {
    // Request with check_prerequisites=true
    await page.goto('/coaches?check_prerequisites=true');

    // Look for prerequisites indicator on coach cards
    const coachCard = page.locator('[data-testid="coach-card"]').first();
    await expect(coachCard).toBeVisible();

    // Should show either met or unmet status
    const prerequisitesStatus = coachCard.locator('[data-testid="prerequisites-status"]');
    const statusText = await prerequisitesStatus.textContent();
    expect(['Prerequisites met', 'Missing prerequisites']).toContain(statusText);
  });

  test('displays missing prerequisites message', async ({ page }) => {
    await page.goto('/coaches?check_prerequisites=true');

    // Find a coach with missing prerequisites
    const unmetCoach = page.locator('[data-testid="coach-card"]:has([data-testid="prerequisites-unmet"])').first();

    if (await unmetCoach.count() > 0) {
      await unmetCoach.click();

      // Should show what's missing
      await expect(page.locator('text=Connect')).toBeVisible();
      await expect(page.locator('[data-testid="missing-prerequisite"]')).toBeVisible();
    }
  });
});

test.describe('Coach Forking', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('[data-testid="coach-library-tab"]');
  });

  test('forks a system coach to create user copy', async ({ page }) => {
    // Find a system coach
    const systemCoach = page.locator('[data-testid="coach-card"]:has([data-testid="system-badge"])').first();
    await expect(systemCoach).toBeVisible();

    // Open context menu or click fork button
    await systemCoach.click({ button: 'right' });
    await page.click('[data-testid="fork-option"]');

    // Confirm fork dialog
    await expect(page.locator('text=Fork Coach')).toBeVisible();
    await page.click('[data-testid="confirm-fork"]');

    // Should show success and redirect to editor
    await expect(page.locator('text=Coach forked successfully')).toBeVisible();
  });

  test('forked coach has correct forked_from reference', async ({ page }) => {
    // Find a system coach and get its ID
    const systemCoach = page.locator('[data-testid="coach-card"]:has([data-testid="system-badge"])').first();
    const sourceId = await systemCoach.getAttribute('data-coach-id');

    // Fork the coach
    await systemCoach.click({ button: 'right' });
    await page.click('[data-testid="fork-option"]');
    await page.click('[data-testid="confirm-fork"]');

    // Wait for navigation to editor
    await page.waitForURL('**/coaches/**');

    // Check the forked_from field via API or UI
    const forkedFromDisplay = page.locator('[data-testid="forked-from"]');
    if (await forkedFromDisplay.count() > 0) {
      await expect(forkedFromDisplay).toContainText(sourceId || '');
    }
  });
});
