// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Playwright configuration for integration tests with real server.
// ABOUTME: Configures both backend and frontend servers, longer timeouts, and sequential execution.

import { defineConfig, devices } from '@playwright/test';
import path from 'path';

const projectRoot = path.resolve(__dirname, '../../..');
const frontendRoot = path.resolve(__dirname, '../..');

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
    baseURL: 'http://localhost:5173',
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
          args: process.env.CI
            ? ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
            : [],
        },
      },
    },
  ],

  globalSetup: path.join(__dirname, '../helpers/global-setup.ts'),
  globalTeardown: path.join(__dirname, '../helpers/global-teardown.ts'),

  webServer: [
    {
      command: `cd ${projectRoot} && DATABASE_URL="sqlite:${projectRoot}/data/integration-test.db" RUST_LOG=warn cargo run --bin pierre-mcp-server`,
      url: 'http://localhost:8081/health',
      timeout: 180000,
      reuseExistingServer: !process.env.CI,
      env: {
        DATABASE_URL: `sqlite:${projectRoot}/data/integration-test.db`,
        PIERRE_MASTER_ENCRYPTION_KEY: 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
        PIERRE_RSA_KEY_SIZE: '2048',
        HTTP_PORT: '8081',
        RUST_LOG: 'warn',
        STRAVA_CLIENT_ID: 'test_client_id_integration',
        STRAVA_CLIENT_SECRET: 'test_client_secret_integration',
        STRAVA_REDIRECT_URI: 'http://localhost:8080/auth/strava/callback',
      },
    },
    {
      command: `cd ${frontendRoot} && npm run dev`,
      url: 'http://localhost:5173',
      timeout: 30000,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
