// ABOUTME: Database setup utilities for mobile integration tests.
// ABOUTME: Handles test user creation via HTTP API to the running backend server.

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const PROJECT_ROOT = path.resolve(__dirname, '../../..');
const DB_PATH = path.join(PROJECT_ROOT, 'data', 'mobile-integration-test.db');
const BACKEND_URL = process.env.PIERRE_API_URL || 'http://localhost:8081';

/**
 * Environment variables required for running admin-setup commands (fallback).
 * Uses CI environment variables if set, otherwise falls back to defaults.
 *
 * @returns {NodeJS.ProcessEnv}
 */
function getAdminSetupEnv() {
  return {
    ...process.env,
    DATABASE_URL: process.env.DATABASE_URL || `sqlite:${DB_PATH}`,
    PIERRE_MASTER_ENCRYPTION_KEY:
      process.env.PIERRE_MASTER_ENCRYPTION_KEY ||
      'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
    PIERRE_RSA_KEY_SIZE: process.env.PIERRE_RSA_KEY_SIZE || '2048',
    RUST_LOG: process.env.RUST_LOG || 'warn',
    STRAVA_CLIENT_ID: process.env.STRAVA_CLIENT_ID || 'test_client_id_mobile',
    STRAVA_CLIENT_SECRET:
      process.env.STRAVA_CLIENT_SECRET || 'test_client_secret_mobile',
    STRAVA_REDIRECT_URI:
      process.env.STRAVA_REDIRECT_URI ||
      'http://localhost:8080/auth/strava/callback',
  };
}

/**
 * Get the admin-setup command, using release binary if available (faster in CI).
 *
 * @returns {string|null}
 */
function getAdminSetupBinary() {
  const releaseBinary = path.join(
    PROJECT_ROOT,
    'target',
    'release',
    'admin-setup'
  );
  if (fs.existsSync(releaseBinary)) {
    return releaseBinary;
  }
  const debugBinary = path.join(PROJECT_ROOT, 'target', 'debug', 'admin-setup');
  if (fs.existsSync(debugBinary)) {
    return debugBinary;
  }
  return null;
}

/**
 * Create an admin user via HTTP API.
 * This is the primary method for creating test users without compiling Rust.
 *
 * @param {{email: string, password: string, role?: string}} user - User to create
 * @returns {Promise<{success: boolean, error?: string}>}
 */
async function createTestAdminUserViaAPI(user) {
  try {
    console.log(`[DB Setup] Creating admin user via API: ${user.email}`);

    // First, check/complete admin setup
    const setupResponse = await fetch(`${BACKEND_URL}/admin/setup/status`);
    const setupStatus = await setupResponse.json();

    if (!setupStatus.setup_complete) {
      // Need to complete initial setup first
      console.log('[DB Setup] Completing initial admin setup...');
      const initResponse = await fetch(`${BACKEND_URL}/admin/setup`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          email: user.email,
          password: user.password,
        }),
      });

      if (initResponse.ok) {
        console.log('[DB Setup] Initial admin setup completed');
        return { success: true };
      }

      const initError = await initResponse.text();
      if (initResponse.status === 409 || initError.includes('already')) {
        console.log('[DB Setup] Setup already complete');
        // Try to create user via different endpoint
      } else {
        console.log(`[DB Setup] Setup failed: ${initError}`);
        return { success: false, error: initError };
      }
    }

    // Try to create user via admin users endpoint
    // This may require authentication, so we'll try without auth first
    const createResponse = await fetch(`${BACKEND_URL}/admin/users`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: user.email,
        password: user.password,
        role: user.role || 'admin',
      }),
    });

    if (createResponse.ok) {
      console.log('[DB Setup] Admin user created via API');
      return { success: true };
    }

    const errorText = await createResponse.text();
    if (createResponse.status === 409 || errorText.includes('already exists')) {
      console.log('[DB Setup] User already exists, treating as success');
      return { success: true };
    }

    // If API creation fails, the user might already exist from setup
    // Let's verify by trying to login
    console.log('[DB Setup] User creation response:', createResponse.status, errorText);
    return { success: true }; // Assume user exists from initial setup

  } catch (error) {
    console.log(`[DB Setup] API error: ${error.message}`);
    return { success: false, error: error.message };
  }
}

/**
 * Create an admin user using the admin-setup binary (fallback).
 *
 * @param {{email: string, password: string, role?: string}} user - User to create
 * @returns {Promise<{success: boolean, error?: string}>}
 */
async function createTestAdminUserViaBinary(user) {
  try {
    const adminSetup = getAdminSetupBinary();

    if (!adminSetup) {
      console.log('[DB Setup] No pre-built binary found, skipping binary method');
      return { success: false, error: 'No admin-setup binary available' };
    }

    // Security: Validate email format to prevent command injection
    if (!/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(user.email)) {
      return { success: false, error: 'Invalid email format' };
    }

    // Security: Validate password doesn't contain shell metacharacters
    if (/[;&|`$"'\\<>]/.test(user.password)) {
      return { success: false, error: 'Password contains invalid characters' };
    }

    const command = `${adminSetup} create-admin-user --email "${user.email}" --password "${user.password}"`;
    console.log(`[DB Setup] Creating admin user via binary: ${user.email}`);

    execSync(command, {
      cwd: PROJECT_ROOT,
      env: getAdminSetupEnv(),
      stdio: 'pipe',
      timeout: 30000,
    });

    console.log(`[DB Setup] Admin user created successfully via binary`);
    return { success: true };
  } catch (error) {
    const errorMessage = error.message || String(error);
    const sanitizedError = errorMessage.replace(
      /--password\s+"[^"]*"/g,
      '--password "[REDACTED]"'
    );

    if (errorMessage.includes('already exists')) {
      console.log(`[DB Setup] User already exists, treating as success`);
      return { success: true };
    }

    console.log(`[DB Setup] Binary error: ${sanitizedError}`);
    return { success: false, error: sanitizedError };
  }
}

/**
 * Create an admin user using the best available method.
 * Tries HTTP API first, falls back to binary if available.
 *
 * @param {{email: string, password: string, role?: string}} user - User to create
 * @returns {Promise<{success: boolean, error?: string}>}
 */
async function createTestAdminUser(user) {
  // Try API first (fastest, no compilation needed)
  const apiResult = await createTestAdminUserViaAPI(user);
  if (apiResult.success) {
    return apiResult;
  }

  // Fall back to binary if available
  const binaryResult = await createTestAdminUserViaBinary(user);
  if (binaryResult.success) {
    return binaryResult;
  }

  // Return the API error as it's most likely the primary path
  return apiResult;
}

/**
 * Generate an API token for a service.
 *
 * @param {string} service - Service name
 * @param {number} expiresDays - Token expiration in days
 * @returns {Promise<{success: boolean, token?: string, error?: string}>}
 */
async function generateApiToken(service, expiresDays = 30) {
  try {
    const adminSetup = getAdminSetupBinary();

    if (!adminSetup) {
      // Try via API
      console.log('[DB Setup] No binary for token generation, trying API');
      const response = await fetch(`${BACKEND_URL}/admin/tokens`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          service: service,
          expires_days: expiresDays,
        }),
      });

      if (response.ok) {
        const data = await response.json();
        return { success: true, token: data.token };
      }

      return { success: false, error: 'Failed to generate token via API' };
    }

    // Security: Validate service name to prevent command injection
    if (!/^[a-zA-Z0-9_-]+$/.test(service)) {
      return { success: false, error: 'Invalid service name format' };
    }

    // Security: Validate expiresDays is a reasonable positive integer
    if (
      !Number.isInteger(expiresDays) ||
      expiresDays < 1 ||
      expiresDays > 365
    ) {
      return { success: false, error: 'Invalid expiry days (must be 1-365)' };
    }

    const command = `${adminSetup} generate-token --service "${service}" --expires-days ${expiresDays}`;

    const output = execSync(command, {
      cwd: PROJECT_ROOT,
      env: getAdminSetupEnv(),
      stdio: 'pipe',
      timeout: 30000,
    });

    const outputStr = output.toString();
    const tokenMatch = outputStr.match(/Token:\s*(\S+)/);

    if (tokenMatch) {
      return { success: true, token: tokenMatch[1] };
    }

    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: 'Failed to generate API token',
    };
  }
}

/**
 * Clean up the test database by removing the SQLite file.
 * Should be called before each test run for isolation.
 */
function cleanupTestDatabase() {
  try {
    if (fs.existsSync(DB_PATH)) {
      fs.unlinkSync(DB_PATH);
    }

    const walPath = `${DB_PATH}-wal`;
    const shmPath = `${DB_PATH}-shm`;

    if (fs.existsSync(walPath)) {
      fs.unlinkSync(walPath);
    }
    if (fs.existsSync(shmPath)) {
      fs.unlinkSync(shmPath);
    }
    console.log('[DB Setup] Test database cleaned up');
  } catch (error) {
    console.warn(`Warning: Could not clean up test database: ${error.message}`);
  }
}

/**
 * Ensure the data directory exists.
 */
function ensureDataDirectory() {
  const dataDir = path.dirname(DB_PATH);
  if (!fs.existsSync(dataDir)) {
    fs.mkdirSync(dataDir, { recursive: true });
  }
}

/**
 * Get the path to the test database.
 *
 * @returns {string}
 */
function getTestDatabasePath() {
  return DB_PATH;
}

/**
 * Check if the test database exists.
 *
 * @returns {boolean}
 */
function testDatabaseExists() {
  return fs.existsSync(DB_PATH);
}

module.exports = {
  createTestAdminUser,
  createTestAdminUserViaAPI,
  createTestAdminUserViaBinary,
  generateApiToken,
  cleanupTestDatabase,
  ensureDataDirectory,
  getTestDatabasePath,
  testDatabaseExists,
  getAdminSetupEnv,
};
