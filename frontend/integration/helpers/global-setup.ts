// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Global setup for Playwright integration tests.
// ABOUTME: Initializes test database and ensures servers are ready before tests run.

import { cleanupTestDatabase, ensureDataDirectory, createTestAdminUser } from './db-setup';
import { waitForServersReady } from './server-manager';
import { testUsers } from '../fixtures/test-data';

async function globalSetup(): Promise<void> {
  console.log('\n[Integration Tests] Starting global setup...');

  ensureDataDirectory();

  if (process.env.CI) {
    console.log('[Integration Tests] Cleaning up test database for fresh run...');
    cleanupTestDatabase();
  }

  console.log('[Integration Tests] Waiting for servers to be ready...');
  const serversReady = await waitForServersReady();

  if (!serversReady.backend) {
    throw new Error('Backend server failed to start or is not healthy');
  }

  if (!serversReady.frontend) {
    throw new Error('Frontend dev server failed to start');
  }

  console.log('[Integration Tests] Servers are ready');

  console.log('[Integration Tests] Creating default test admin user...');
  const adminResult = await createTestAdminUser(testUsers.admin);
  if (!adminResult.success) {
    console.warn(`[Integration Tests] Warning: Could not create admin user: ${adminResult.error}`);
  }

  console.log('[Integration Tests] Global setup complete\n');
}

export default globalSetup;
