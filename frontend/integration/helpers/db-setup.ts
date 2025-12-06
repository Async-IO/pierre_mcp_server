// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Database setup utilities for integration tests.
// ABOUTME: Handles test user creation and database cleanup via admin-setup binary.

import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(currentDir, '../../..');
const DB_PATH = path.join(PROJECT_ROOT, 'data', 'integration-test.db');

// Binary paths - check release first (CI builds release), then debug (local dev)
const ADMIN_SETUP_RELEASE = path.join(PROJECT_ROOT, 'target', 'release', 'admin-setup');
const ADMIN_SETUP_DEBUG = path.join(PROJECT_ROOT, 'target', 'debug', 'admin-setup');

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
 * Environment variables required for running admin-setup commands.
 */
function getAdminSetupEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    DATABASE_URL: `sqlite:${DB_PATH}`,
    PIERRE_MASTER_ENCRYPTION_KEY: process.env.PIERRE_MASTER_ENCRYPTION_KEY || 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
    PIERRE_RSA_KEY_SIZE: process.env.PIERRE_RSA_KEY_SIZE || '2048',
    RUST_LOG: 'warn',
    STRAVA_CLIENT_ID: process.env.STRAVA_CLIENT_ID || 'test_client_id_integration',
    STRAVA_CLIENT_SECRET: process.env.STRAVA_CLIENT_SECRET || 'test_client_secret_integration',
    STRAVA_REDIRECT_URI: process.env.STRAVA_REDIRECT_URI || 'http://localhost:8080/auth/strava/callback',
  };
}

/**
 * Get the command to run admin-setup.
 * Prefers pre-built binary (release first, then debug) for speed, falls back to cargo run.
 */
function getAdminSetupCommand(args: string): string {
  // Check release binary first (CI builds with --release)
  if (fs.existsSync(ADMIN_SETUP_RELEASE)) {
    console.log(`[DB Setup] Using release binary: ${ADMIN_SETUP_RELEASE}`);
    return `${ADMIN_SETUP_RELEASE} ${args}`;
  }
  // Check debug binary (local development)
  if (fs.existsSync(ADMIN_SETUP_DEBUG)) {
    console.log(`[DB Setup] Using debug binary: ${ADMIN_SETUP_DEBUG}`);
    return `${ADMIN_SETUP_DEBUG} ${args}`;
  }
  // Fall back to cargo run (will compile if needed)
  console.log('[DB Setup] Using cargo run (no pre-built binary found)');
  return `cargo run --bin admin-setup -- ${args}`;
}

/**
 * Create an admin user using the admin-setup binary.
 * This is the primary method for seeding test users.
 */
export async function createTestAdminUser(user: TestUser): Promise<CreateUserResult> {
  try {
    console.log(`[DB Setup] Creating admin user: ${user.email}`);
    console.log(`[DB Setup] Database path: ${DB_PATH}`);
    console.log(`[DB Setup] Project root: ${PROJECT_ROOT}`);

    const command = getAdminSetupCommand(`create-admin-user --email "${user.email}" --password "${user.password}" --force`);
    console.log(`[DB Setup] Running command: ${command}`);

    const output = execSync(command, {
      cwd: PROJECT_ROOT,
      env: getAdminSetupEnv(),
      stdio: 'pipe',
      timeout: 120000, // 2 minutes for cargo compile if needed
    });

    const outputStr = output.toString();
    console.log(`[DB Setup] Command output: ${outputStr}`);
    console.log(`[DB Setup] User created successfully: ${user.email}`);

    return { success: true };
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    const stderr = (error as { stderr?: Buffer })?.stderr?.toString() || '';

    console.log(`[DB Setup] User creation error: ${errorMessage}`);
    if (stderr) {
      console.log(`[DB Setup] Stderr: ${stderr}`);
    }

    // User already exists is not an error for our purposes
    if (errorMessage.includes('already exists') || stderr.includes('already exists')) {
      console.log(`[DB Setup] User already exists, treating as success: ${user.email}`);
      return { success: true };
    }

    return {
      success: false,
      error: `Failed to create admin user: ${errorMessage}`,
    };
  }
}

/**
 * Generate an API token for a service using admin-setup.
 */
export async function generateApiToken(
  service: string,
  expiresDays: number = 30
): Promise<{ success: boolean; token?: string; error?: string }> {
  try {
    const command = getAdminSetupCommand(`generate-token --service "${service}" --expires-days ${expiresDays}`);

    const output = execSync(command, {
      cwd: PROJECT_ROOT,
      env: getAdminSetupEnv(),
      stdio: 'pipe',
      timeout: 120000,
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
      error: `Failed to generate API token: ${error}`,
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
