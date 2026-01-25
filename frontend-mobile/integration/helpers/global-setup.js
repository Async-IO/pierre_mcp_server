// ABOUTME: Global setup for mobile integration tests.
// ABOUTME: Initializes test database and ensures backend server is ready before tests run.

const { ensureDataDirectory, createTestAdminUser } = require('./db-setup');
const { waitForBackendHealth } = require('./server-manager');
const { testUsers } = require('../fixtures/test-data');

/**
 * Global setup function that runs once before all tests.
 * Ensures the backend server is healthy and creates the default test user.
 */
async function globalSetup() {
  console.log('\n[Mobile Integration Tests] Starting global setup...');

  // Ensure data directory exists for the test database
  ensureDataDirectory();

  // Wait for the backend server to be ready
  console.log('[Mobile Integration Tests] Waiting for backend server...');
  const healthResult = await waitForBackendHealth();

  if (!healthResult.healthy) {
    throw new Error(
      `Backend server is not healthy: ${healthResult.error}\n` +
        'Make sure the Pierre server is running on port 8081.\n' +
        'Start it with: ./bin/start-server.sh'
    );
  }

  console.log(
    `[Mobile Integration Tests] Backend healthy (version: ${healthResult.version || 'unknown'})`
  );

  // Create the default test admin user
  console.log('[Mobile Integration Tests] Creating default test admin user...');
  const adminResult = await createTestAdminUser(testUsers.admin);

  if (!adminResult.success) {
    console.warn(
      `[Mobile Integration Tests] Warning: Could not create admin user: ${adminResult.error}`
    );
    // Don't fail - user might already exist
  }

  console.log('[Mobile Integration Tests] Global setup complete\n');
}

module.exports = globalSetup;
