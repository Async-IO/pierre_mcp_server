// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Global teardown for Playwright integration tests.
// ABOUTME: Cleans up test database and resources after all tests complete.

import { cleanupTestDatabase } from './db-setup';

async function globalTeardown(): Promise<void> {
  console.log('\n[Integration Tests] Starting global teardown...');

  if (process.env.CI || process.env.CLEANUP_DB) {
    console.log('[Integration Tests] Cleaning up test database...');
    cleanupTestDatabase();
  } else {
    console.log('[Integration Tests] Preserving test database for inspection (set CLEANUP_DB=1 to remove)');
  }

  console.log('[Integration Tests] Global teardown complete\n');
}

export default globalTeardown;
