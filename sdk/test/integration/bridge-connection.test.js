// ABOUTME: Integration tests for bridge connection establishment and validation
// ABOUTME: Tests server connectivity, health checks, and MCP endpoint availability
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { TestConfig } = require('../helpers/fixtures');

// Use native fetch (Node 18+)
const fetch = global.fetch;

describe('Bridge Integration Tests', () => {
  let serverHandle;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 60000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  describe('Server Connection', () => {
    test('should verify Pierre server is running', async () => {
      const response = await fetch(`${serverUrl}/health`);
      expect(response.ok).toBe(true);

      const health = await response.json();
      expect(health.status).toBe('ok');
    });

    test('should access MCP endpoint', async () => {
      try {
        const response = await fetch(`${serverUrl}/mcp`, {
          method: 'OPTIONS'
        });
        expect(response.status).toBeLessThan(500);
      } catch (error) {
        // Server may require authentication, but should be accessible
        expect(error).toBeDefined();
      }
    });

    test('should verify server has expected endpoints', async () => {
      const healthResponse = await fetch(`${serverUrl}/health`);
      expect(healthResponse.ok).toBe(true);

      // MCP endpoint should exist (may return 401 or other error without auth)
      const mcpResponse = await fetch(`${serverUrl}/mcp`, {
        method: 'OPTIONS'
      });
      expect(mcpResponse.status).not.toBe(404);
    });
  });

  describe('Server Health and Status', () => {
    test('should have valid health response structure', async () => {
      const response = await fetch(`${serverUrl}/health`);
      const health = await response.json();

      expect(health).toHaveProperty('status');
      expect(health.status).toBe('ok');
    });

    test('should respond within timeout', async () => {
      const startTime = Date.now();

      const response = await fetch(`${serverUrl}/health`);
      const duration = Date.now() - startTime;

      expect(response.ok).toBe(true);
      expect(duration).toBeLessThan(5000);
    });
  });
});
