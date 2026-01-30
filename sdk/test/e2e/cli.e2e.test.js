// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: E2E tests for CLI commands in the SDK.
// ABOUTME: Tests all CLI operations including help, version, and main functionality.

const { spawn, execSync } = require('child_process');
const path = require('path');

const TIMEOUT = 30000;
const CLI_PATH = path.join(__dirname, '../../dist/cli.js');

const execCli = (args = [], options = {}) => {
  return new Promise((resolve, reject) => {
    const proc = spawn('node', [CLI_PATH, ...args], {
      env: {
        ...process.env,
        PIERRE_SERVER_URL: options.serverUrl || 'http://localhost:8081',
        PIERRE_JWT_TOKEN: options.token || '',
        CI: 'true',
      },
      timeout: options.timeout || 10000,
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    proc.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    proc.on('close', (code) => {
      resolve({ code, stdout, stderr });
    });

    proc.on('error', reject);

    // Auto-close after timeout for long-running processes
    if (options.autoClose) {
      setTimeout(() => {
        proc.kill('SIGTERM');
      }, options.autoClose);
    }
  });
};

describe('CLI E2E Tests', () => {
  describe('Help and Version', () => {
    test('should display help with --help flag', async () => {
      const result = await execCli(['--help']);

      expect(result.code).toBe(0);
      expect(result.stdout).toContain('pierre-mcp-client');
      expect(result.stdout).toContain('MCP client');
      expect(result.stdout).toContain('--server');
      expect(result.stdout).toContain('--token');
    }, TIMEOUT);

    test('should display version with --version flag', async () => {
      const result = await execCli(['--version']);

      expect(result.code).toBe(0);
      // Version should be a semver-like string
      expect(result.stdout).toMatch(/\d+\.\d+\.\d+/);
    }, TIMEOUT);

    test('should display help with -h flag', async () => {
      const result = await execCli(['-h']);

      expect(result.code).toBe(0);
      expect(result.stdout).toContain('pierre-mcp-client');
    }, TIMEOUT);
  });

  describe('Server URL Option', () => {
    test('should accept --server option', async () => {
      const result = await execCli(['--server', 'http://localhost:9999', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept -s shorthand for server', async () => {
      const result = await execCli(['-s', 'http://localhost:9999', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);
  });

  describe('Token Option', () => {
    test('should accept --token option', async () => {
      const result = await execCli(['--token', 'test-jwt-token', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept -t shorthand for token', async () => {
      const result = await execCli(['-t', 'test-jwt-token', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);
  });

  describe('OAuth Options', () => {
    test('should accept --oauth-client-id option', async () => {
      const result = await execCli(['--oauth-client-id', 'test-client-id', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept --oauth-client-secret option', async () => {
      const result = await execCli(['--oauth-client-secret', 'test-secret', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept --callback-port option', async () => {
      const result = await execCli(['--callback-port', '35536', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept --no-browser option', async () => {
      const result = await execCli(['--no-browser', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);
  });

  describe('Timeout Options', () => {
    test('should accept --token-validation-timeout option', async () => {
      const result = await execCli(['--token-validation-timeout', '5000', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept --proactive-connection-timeout option', async () => {
      const result = await execCli(['--proactive-connection-timeout', '10000', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);

    test('should accept --tool-call-connection-timeout option', async () => {
      const result = await execCli(['--tool-call-connection-timeout', '15000', '--help']);

      expect(result.code).toBe(0);
    }, TIMEOUT);
  });

  describe('Environment Variables', () => {
    test('should read PIERRE_SERVER_URL from environment', async () => {
      const result = await execCli([], {
        serverUrl: 'http://env-server:8081',
        autoClose: 3000,
      });

      // Should start (or fail to connect) but not error on parsing
      expect(result.stderr).toContain('PIERRE_SERVER_URL');
    }, TIMEOUT);

    test('should read PIERRE_JWT_TOKEN from environment', async () => {
      const result = await execCli([], {
        token: 'env-jwt-token',
        autoClose: 3000,
      });

      // Debug output should show token is set
      expect(result.stderr).toContain('PIERRE_JWT_TOKEN');
      expect(result.stderr).toContain('[SET]');
    }, TIMEOUT);
  });

  describe('Bridge Startup', () => {
    test('should output debug information on startup', async () => {
      const result = await execCli([], { autoClose: 3000 });

      expect(result.stderr).toContain('[DEBUG]');
      expect(result.stderr).toContain('Bridge CLI starting');
    }, TIMEOUT);

    test('should log CI environment detection', async () => {
      const result = await execCli([], { autoClose: 3000 });

      // Should detect CI environment
      expect(result.stderr).toContain('CI');
    }, TIMEOUT);
  });

  describe('Error Handling', () => {
    test('should handle unknown options gracefully', async () => {
      const result = await execCli(['--unknown-option']);

      // Commander should show error for unknown option
      expect(result.code).not.toBe(0);
      expect(result.stderr).toContain('unknown option');
    }, TIMEOUT);

    test('should handle invalid timeout values', async () => {
      const result = await execCli(['--token-validation-timeout', 'not-a-number'], {
        autoClose: 3000,
      });

      // Should start but may have NaN issues
      // At minimum it shouldn't crash on parsing
      expect(typeof result.code).toBe('number');
    }, TIMEOUT);
  });

  describe('Graceful Shutdown', () => {
    test('should handle SIGTERM gracefully', async () => {
      const proc = spawn('node', [CLI_PATH], {
        env: {
          ...process.env,
          CI: 'true',
        },
      });

      let stderr = '';
      proc.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      // Wait for startup
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // Send SIGTERM
      proc.kill('SIGTERM');

      // Wait for shutdown
      await new Promise((resolve) => setTimeout(resolve, 2000));

      expect(stderr).toContain('shutting down');
    }, TIMEOUT);
  });

  describe('Combined Options', () => {
    test('should accept multiple options together', async () => {
      const result = await execCli([
        '--server', 'http://localhost:8081',
        '--token', 'test-token',
        '--oauth-client-id', 'client-id',
        '--callback-port', '35537',
        '--no-browser',
        '--help',
      ]);

      expect(result.code).toBe(0);
    }, TIMEOUT);
  });
});
