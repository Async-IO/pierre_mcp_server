// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Global setup for Playwright integration tests.
// ABOUTME: Initializes test database and ensures servers are ready before tests run.

import { ensureDataDirectory, createTestAdminUser } from './db-setup';
import { waitForServersReady } from './server-manager';
import { testUsers } from '../fixtures/test-data';

async function globalSetup(): Promise<void> {
  console.log('\n[Integration Tests] Starting global setup...');

  ensureDataDirectory();

  // NOTE: Do NOT cleanup database here - the server has already started and connected to it.
  // Deleting the DB file would leave the server with a stale connection.
  // We use --force flag in createTestAdminUser to handle existing users.
  // For a completely fresh start, cleanup should happen BEFORE the server starts.

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
  console.log(`[Integration Tests] User email: ${testUsers.admin.email}`);
  try {
    const adminResult = await createTestAdminUser(testUsers.admin);
    console.log(`[Integration Tests] createTestAdminUser returned: ${JSON.stringify(adminResult)}`);

    if (!adminResult.success) {
      console.error(`[Integration Tests] CRITICAL: Could not create admin user: ${adminResult.error}`);
      // In CI, fail fast if we can't create users - tests will fail anyway
      if (process.env.CI) {
        throw new Error(`Failed to create admin user: ${adminResult.error}`);
      }
    } else {
      console.log('[Integration Tests] Admin user created successfully');
    }
  } catch (setupError) {
    console.error(`[Integration Tests] Exception during user creation: ${setupError}`);
    if (process.env.CI) {
      throw setupError;
    }
  }

  console.log('[Integration Tests] Global setup complete\n');
}

export default globalSetup;
