// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright configuration for integration tests with real server.
// ABOUTME: Configures both backend and frontend servers, longer timeouts, and sequential execution.

import { defineConfig, devices } from '@playwright/test';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(currentDir, '../../..');
const frontendRoot = path.resolve(currentDir, '../..');

// Log paths for debugging in CI
console.log(`[Playwright Config] currentDir: ${currentDir}`);
console.log(`[Playwright Config] projectRoot: ${projectRoot}`);
console.log(`[Playwright Config] frontendRoot: ${frontendRoot}`);
console.log(`[Playwright Config] DATABASE_URL will be: sqlite:${projectRoot}/data/integration-test.db`);

// Configurable ports for CI/CD flexibility
const backendPort = process.env.BACKEND_PORT || '8081';
const frontendPort = process.env.FRONTEND_PORT || '5173';
const backendUrl = process.env.BACKEND_URL || `http://localhost:${backendPort}`;
const frontendUrl = process.env.FRONTEND_URL || `http://localhost:${frontendPort}`;

// Detect pre-built binary (CI builds release, local dev builds debug)
const serverBinaryRelease = path.join(projectRoot, 'target', 'release', 'pierre-mcp-server');
const serverBinaryDebug = path.join(projectRoot, 'target', 'debug', 'pierre-mcp-server');

function getServerCommand(): string {
  // Use pre-built binary if available (much faster startup)
  if (fs.existsSync(serverBinaryRelease)) {
    console.log(`[Playwright Config] Using release binary: ${serverBinaryRelease}`);
    return serverBinaryRelease;
  }
  if (fs.existsSync(serverBinaryDebug)) {
    console.log(`[Playwright Config] Using debug binary: ${serverBinaryDebug}`);
    return serverBinaryDebug;
  }
  // Fall back to cargo run (will compile if needed)
  console.log('[Playwright Config] Using cargo run (no pre-built binary found)');
  return `cargo run --bin pierre-mcp-server --`;
}

// Chrome flags for containerized environments
// CI with playwright install --with-deps has proper Chrome support
// PLAYWRIGHT_SINGLE_PROCESS=true forces single-process mode (for constrained containers)
const useSingleProcess = process.env.PLAYWRIGHT_SINGLE_PROCESS === 'true';
const chromeArgs = process.env.CI
  ? ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage', '--disable-gpu']
  : useSingleProcess
    ? ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--single-process', '--no-zygote']
    : [];

export default defineConfig({
  testDir: '../specs',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? 'github' : 'html',
  timeout: 60000,
  expect: {
    timeout: 10000,
  },
  outputDir: path.join(frontendRoot, 'test-results', 'integration'),

  use: {
    baseURL: frontendUrl,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'on-first-retry',
  },

  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          args: chromeArgs,
        },
      },
    },
  ],

  globalSetup: path.join(currentDir, '../helpers/global-setup.ts'),
  globalTeardown: path.join(currentDir, '../helpers/global-teardown.ts'),

  webServer: [
    {
      command: `cd ${projectRoot} && mkdir -p data && ${getServerCommand()}`,
      url: `${backendUrl}/health`,
      timeout: 180000,
      reuseExistingServer: !process.env.CI,
      env: {
        DATABASE_URL: `sqlite:${projectRoot}/data/integration-test.db`,
        PIERRE_MASTER_ENCRYPTION_KEY: process.env.PIERRE_MASTER_ENCRYPTION_KEY || 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
        PIERRE_RSA_KEY_SIZE: process.env.PIERRE_RSA_KEY_SIZE || '2048',
        HTTP_PORT: backendPort,
        RUST_LOG: process.env.RUST_LOG || 'warn',
        STRAVA_CLIENT_ID: process.env.STRAVA_CLIENT_ID || 'test_client_id_integration',
        STRAVA_CLIENT_SECRET: process.env.STRAVA_CLIENT_SECRET || 'test_client_secret_integration',
        STRAVA_REDIRECT_URI: process.env.STRAVA_REDIRECT_URI || 'http://localhost:8080/auth/strava/callback',
      },
    },
    {
      command: `cd ${frontendRoot} && bun run dev -- --port ${frontendPort}`,
      url: frontendUrl,
      timeout: 60000,
      reuseExistingServer: !process.env.CI,
      env: {
        VITE_BACKEND_URL: backendUrl,
      },
    },
  ],
});

// Export for use in test files to check if single-process mode is active
export const isSingleProcessMode = useSingleProcess;
