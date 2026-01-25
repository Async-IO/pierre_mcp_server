// ABOUTME: Global teardown for mobile integration tests.
// ABOUTME: Cleans up test data after all tests complete.

const { cleanupTestDatabase } = require('./db-setup');

/**
 * Global teardown function that runs once after all tests.
 * Cleans up the test database in CI or when CLEANUP_DB is set.
 */
async function globalTeardown() {
  console.log('\n[Mobile Integration Tests] Starting global teardown...');

  // Only clean up database in CI or when explicitly requested
  // This preserves local debugging capability
  if (process.env.CI || process.env.CLEANUP_DB === '1') {
    console.log('[Mobile Integration Tests] Cleaning up test database...');
    cleanupTestDatabase();
  } else {
    console.log(
      '[Mobile Integration Tests] Skipping database cleanup (local dev mode)'
    );
    console.log('Set CLEANUP_DB=1 to force cleanup');
  }

  console.log('[Mobile Integration Tests] Global teardown complete\n');
}

module.exports = globalTeardown;
