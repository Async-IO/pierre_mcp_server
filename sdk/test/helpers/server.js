// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Server lifecycle management helper for integration tests
// ABOUTME: Starts, monitors health, and cleanly shuts down Pierre MCP server instances
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { spawn } = require('child_process');
const path = require('path');

// Use native fetch (Node 18+) or dynamic import for node-fetch
const fetch = global.fetch || (async (...args) => {
  const nodeFetch = await import('node-fetch');
  return nodeFetch.default(...args);
});

/**
 * Ensures Pierre MCP server is running
 * - In CI: Always starts fresh server
 * - Locally: Uses existing server if available
 */
async function ensureServerRunning(config = {}) {
  const isCI = process.env.CI === 'true';
  const port = config.port || process.env.PIERRE_SERVER_PORT || process.env.HTTP_PORT || process.env.MCP_PORT || 8081;
  const healthUrl = `http://localhost:${port}/health`;

  if (isCI) {
    console.log('🤖 CI environment - starting fresh server');
    return await startServer({ port, ...config });
  }

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);
    const response = await fetch(healthUrl, { signal: controller.signal });
    clearTimeout(timeout);

    if (response.ok) {
      console.log('✅ Using existing Pierre server');
      const testToken = await registerAndGetToken(port);
      return { process: null, port, logs: [], testToken, cleanup: null };
    }
  } catch (error) {
    // Server not running
  }

  console.log('🚀 Starting Pierre server for tests...');
  return await startServer({ port, ...config });
}

/**
 * Start Pierre MCP server for testing
 */
async function startServer(config) {
  const port = config.port || process.env.HTTP_PORT || process.env.MCP_PORT || 8081;

  // Try multiple possible locations for the server binary
  // Supports: explicit config, relative paths from sdk/test/helpers/, and PIERRE_SERVER_BINARY env var
  const possiblePaths = [
    config.binaryPath,
    process.env.PIERRE_SERVER_BINARY,
    path.join(__dirname, '../../../target/debug/pierre-mcp-server'),
    path.join(__dirname, '../../../target/release/pierre-mcp-server')
  ].filter(Boolean);

  let serverPath = null;
  const fs = require('fs');
  for (const testPath of possiblePaths) {
    if (fs.existsSync(testPath)) {
      serverPath = testPath;
      break;
    }
  }

  if (!serverPath) {
    throw new Error('Pierre server binary not found. Please run: cargo build --bin pierre-mcp-server');
  }

  const env = {
    ...process.env,
    HTTP_PORT: port.toString(),
    DATABASE_URL: config.database || 'sqlite::memory:',
    PIERRE_MASTER_ENCRYPTION_KEY: config.encryptionKey || 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
    PIERRE_JWT_SECRET: config.jwtSecret || 'test_jwt_secret_for_automated_tests_only',
    PIERRE_RSA_KEY_SIZE: '2048', // Use smaller key size for faster test startup
    RUST_LOG: config.logLevel || 'info'
  };

  const serverProcess = spawn(serverPath, [], {
    env,
    stdio: process.env.DEBUG ? 'inherit' : 'pipe',
    detached: false
  });

  const logs = [];
  if (serverProcess.stdout) {
    serverProcess.stdout.on('data', (data) => {
      logs.push(data.toString());
      if (process.env.DEBUG) {
        console.log(`[Server]: ${data}`);
      }
    });
  }

  if (serverProcess.stderr) {
    serverProcess.stderr.on('data', (data) => {
      logs.push(data.toString());
      if (process.env.DEBUG) {
        console.error(`[Server Error]: ${data}`);
      }
    });
  }

  serverProcess.on('error', (error) => {
    console.error(`❌ Failed to start server: ${error.message}`);
  });

  try {
    await waitForHealth(`http://localhost:${port}/health`, 30000);
  } catch (error) {
    serverProcess.kill('SIGKILL');
    console.error('Server logs:', logs.join('\n'));
    throw error;
  }

  // Register a test user and get a real RS256 JWT token for authenticated tests
  const testToken = await registerAndGetToken(port);

  return {
    process: serverProcess,
    port,
    logs,
    testToken,
    cleanup: async () => {
      return new Promise((resolve) => {
        serverProcess.on('exit', resolve);
        serverProcess.kill('SIGTERM');
        setTimeout(() => {
          if (!serverProcess.killed) {
            serverProcess.kill('SIGKILL');
          }
          resolve();
        }, 5000);
      });
    }
  };
}

/**
 * Register a test user and login to get a real RS256 JWT token.
 * The server uses RS256 (RSA) JWT validation, so test tokens must come from
 * the actual server login endpoint rather than being locally generated.
 */
async function registerAndGetToken(port) {
  const baseUrl = `http://localhost:${port}`;
  const testEmail = `sdk-test-${Date.now()}@example.com`;
  const testPassword = 'SdkTestPassword123!';

  // Register user (may be auto-approved or pending depending on config)
  try {
    const registerResponse = await fetch(`${baseUrl}/api/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: testEmail,
        password: testPassword,
        display_name: 'SDK Test User'
      })
    });

    if (!registerResponse.ok) {
      const errorText = await registerResponse.text();
      console.warn(`⚠️ Registration returned ${registerResponse.status}: ${errorText}`);
    }
  } catch (error) {
    console.warn(`⚠️ Registration failed: ${error.message}`);
  }

  // Login via OAuth2 ROPC (RFC 6749 §4.3) to get RS256 JWT
  try {
    const loginResponse = await fetch(`${baseUrl}/oauth/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'password',
        username: testEmail,
        password: testPassword
      }).toString()
    });

    if (loginResponse.ok) {
      const tokenData = await loginResponse.json();
      console.log('✅ Test user authenticated with RS256 JWT');
      return {
        access_token: tokenData.access_token,
        token_type: tokenData.token_type || 'Bearer',
        expires_in: tokenData.expires_in || 86400,
        scope: tokenData.scope || 'read:fitness write:fitness',
        saved_at: Math.floor(Date.now() / 1000)
      };
    }

    const errorText = await loginResponse.text();
    console.warn(`⚠️ Login returned ${loginResponse.status}: ${errorText}`);
  } catch (error) {
    console.warn(`⚠️ Login failed: ${error.message}`);
  }

  // Fallback: return a placeholder token (tests that require auth will fail with clear errors)
  console.warn('⚠️ Could not get RS256 token - tests requiring auth will fail');
  return {
    access_token: 'INVALID_NO_RS256_TOKEN_AVAILABLE',
    token_type: 'Bearer',
    expires_in: 3600,
    scope: 'read:fitness write:fitness',
    saved_at: Math.floor(Date.now() / 1000)
  };
}

/**
 * Wait for server health check to pass
 */
async function waitForHealth(url, timeout = 30000) {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      const controller = new AbortController();
      const fetchTimeout = setTimeout(() => controller.abort(), 1000);
      const response = await fetch(url, { signal: controller.signal });
      clearTimeout(fetchTimeout);

      if (response.ok) {
        console.log('✅ Server health check passed');
        return;
      }
    } catch (error) {
      // Not ready yet
    }
    await sleep(500);
  }

  throw new Error(`Server health check failed after ${timeout}ms`);
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

module.exports = {
  ensureServerRunning,
  startServer,
  registerAndGetToken,
  waitForHealth,
  sleep
};
