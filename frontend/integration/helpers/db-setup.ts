// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Database setup utilities for integration tests.
// ABOUTME: Handles test user creation and database cleanup via pierre-cli binary.

import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PROJECT_ROOT = path.resolve(__dirname, '../../..');
const DB_PATH = path.join(PROJECT_ROOT, 'data', 'integration-test.db');

export interface TestUser {
  email: string;
  password: string;
  role?: 'admin' | 'super_admin' | 'user';
}

export interface CreateUserResult {
  success: boolean;
  error?: string;
}

/**
 * Environment variables required for running pierre-cli commands.
 * Uses CI environment variables if set, otherwise falls back to defaults.
 */
function getAdminSetupEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    DATABASE_URL: process.env.DATABASE_URL || `sqlite:${DB_PATH}`,
    PIERRE_MASTER_ENCRYPTION_KEY: process.env.PIERRE_MASTER_ENCRYPTION_KEY || 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
    PIERRE_RSA_KEY_SIZE: process.env.PIERRE_RSA_KEY_SIZE || '2048',
    RUST_LOG: process.env.RUST_LOG || 'warn',
    STRAVA_CLIENT_ID: process.env.STRAVA_CLIENT_ID || 'test_client_id_integration',
    STRAVA_CLIENT_SECRET: process.env.STRAVA_CLIENT_SECRET || 'test_client_secret_integration',
    STRAVA_REDIRECT_URI: process.env.STRAVA_REDIRECT_URI || 'http://localhost:8080/auth/strava/callback',
  };
}

/**
 * Get the pierre-cli command, using release binary if available (faster in CI).
 */
function getAdminSetupCommand(): string {
  const releaseBinary = path.join(PROJECT_ROOT, 'target', 'release', 'pierre-cli');
  if (fs.existsSync(releaseBinary)) {
    return releaseBinary;
  }
  return 'cargo run --release --bin pierre-cli --';
}

/**
 * Create an admin user using the pierre-cli binary.
 * This is the primary method for seeding test users.
 */
export async function createTestAdminUser(user: TestUser): Promise<CreateUserResult> {
  try {
    const adminSetup = getAdminSetupCommand();
    // Security: Don't log the full command with password - it's sensitive
    // Security: Validate email format to prevent command injection
    if (!/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(user.email)) {
      return { success: false, error: 'Invalid email format' };
    }
    // Security: Validate password doesn't contain shell metacharacters
    if (/[;&|`$"'\\<>]/.test(user.password)) {
      return { success: false, error: 'Password contains invalid characters' };
    }
    const command = `${adminSetup} user create --email "${user.email}" --password "${user.password}" --force`;
    // Security: Don't log the full command or DATABASE_URL as they may contain credentials
    console.log(`[DB Setup] Creating admin user: ${user.email}`);

    execSync(command, {
      cwd: PROJECT_ROOT,
      env: getAdminSetupEnv(),
      stdio: 'pipe',
      timeout: 60000,
    });

    // Security: Only log success status, not the full output which may contain sensitive info
    console.log(`[DB Setup] Admin user created successfully`);
    return { success: true };
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    // Security: Sanitize error message to avoid leaking sensitive info
    const sanitizedError = errorMessage.replace(/--password\s+"[^"]*"/g, '--password "[REDACTED]"');
    console.log(`[DB Setup] Command error: ${sanitizedError}`);

    if (errorMessage.includes('already exists')) {
      console.log(`[DB Setup] User already exists, treating as success`);
      return { success: true };
    }

    return {
      success: false,
      error: `Failed to create admin user: ${errorMessage}`,
    };
  }
}

/**
 * Generate an API token for a service using pierre-cli.
 */
export async function generateApiToken(
  service: string,
  expiresDays: number = 30
): Promise<{ success: boolean; token?: string; error?: string }> {
  try {
    const adminSetup = getAdminSetupCommand();
    // Security: Validate service name to prevent command injection
    if (!/^[a-zA-Z0-9_-]+$/.test(service)) {
      return { success: false, error: 'Invalid service name format' };
    }
    // Security: Validate expiresDays is a reasonable positive integer
    if (!Number.isInteger(expiresDays) || expiresDays < 1 || expiresDays > 365) {
      return { success: false, error: 'Invalid expiry days (must be 1-365)' };
    }
    const command = `${adminSetup} token generate --service "${service}" --expires-days ${expiresDays}`;

    const output = execSync(command, {
      cwd: PROJECT_ROOT,
      env: getAdminSetupEnv(),
      stdio: 'pipe',
      timeout: 60000,
    });

    const outputStr = output.toString();
    const tokenMatch = outputStr.match(/Token:\s*(\S+)/);

    if (tokenMatch) {
      return { success: true, token: tokenMatch[1] };
    }

    return { success: true };
  } catch {
    // Security: Don't expose raw error details
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
export function cleanupTestDatabase(): void {
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
  } catch (error) {
    console.warn(`Warning: Could not clean up test database: ${error}`);
  }
}

/**
 * Ensure the data directory exists.
 */
export function ensureDataDirectory(): void {
  const dataDir = path.dirname(DB_PATH);
  if (!fs.existsSync(dataDir)) {
    fs.mkdirSync(dataDir, { recursive: true });
  }
}

/**
 * Get the path to the test database.
 */
export function getTestDatabasePath(): string {
  return DB_PATH;
}

/**
 * Check if the test database exists.
 */
export function testDatabaseExists(): boolean {
  return fs.existsSync(DB_PATH);
}
