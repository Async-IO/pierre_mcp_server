// ABOUTME: Server lifecycle management for bridge tests
// ABOUTME: Handles starting/stopping Pierre MCP server with health checks

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
  const port = config.port || process.env.PIERRE_SERVER_PORT || 8888;
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
      return null;
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
  const port = config.port || 8888;

  // Try multiple possible locations for the server binary
  const possiblePaths = [
    config.binaryPath,
    path.join(__dirname, '../../../target/debug/pierre-mcp-server'),
    '/Users/jeanfrancoisarcand/workspace/strava_ai/pierre_mcp_server/target/debug/pierre-mcp-server',
    path.join(__dirname, '../../../../../target/debug/pierre-mcp-server')
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

  return {
    process: serverProcess,
    port,
    logs,
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
  waitForHealth,
  sleep
};
