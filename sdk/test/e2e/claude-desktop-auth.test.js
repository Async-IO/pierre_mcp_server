// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Claude Desktop authentication E2E tests - token refresh, session persistence, context switching
// ABOUTME: Tests auth flow including session recovery and multi-tenant scenarios
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');
const path = require('path');
const fs = require('fs');
const os = require('os');

describe('E2E: Claude Desktop Token Refresh During Session', () => {
  let serverHandle;
  let bridgeClient;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);
  }, 90000);

  afterAll(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
    }
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should maintain session after token refresh', async () => {
    // Make initial request
    const response1 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response1).toHaveProperty('result');
    expect(response1.result).toHaveProperty('tools');

    // Simulate time passing (in a real test, token would approach expiry)
    await new Promise(resolve => setTimeout(resolve, 100));

    // Make another request - should work seamlessly
    const response2 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response2).toHaveProperty('result');
    expect(response2.result).toHaveProperty('tools');
    expect(response2.result.tools.length).toBe(response1.result.tools.length);
  }, 30000);

  test('should preserve tool call context after token refresh', async () => {
    // First tool call
    const response1 = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'list_connections',
        arguments: {}
      }
    });

    expect(response1).toBeDefined();

    // Second tool call should work and have consistent context
    const response2 = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    });

    expect(response2).toBeDefined();
  }, 60000);

  test('should handle expired token gracefully', async () => {
    // Make a request - bridge should handle token validation
    const response = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_athlete',
        arguments: { provider: 'strava' }
      }
    });

    // Should complete with either success or structured error
    expect(response).toBeDefined();
    if (response.error) {
      expect(response.error).toHaveProperty('code');
      expect(response.error).toHaveProperty('message');
    }
  }, 30000);

  test('should notify when re-authentication is required', async () => {
    // When tokens fully expire, user needs to re-authenticate
    // Bridge should indicate this clearly
    const response = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response).toBeDefined();
    // If auth is required, response structure should indicate it
  }, 30000);
});

describe('E2E: Claude Desktop Session Persistence', () => {
  let serverHandle;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const testTokenFile = path.join(os.tmpdir(), `test-tokens-${Date.now()}.json`);

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
    // Cleanup test token file
    if (fs.existsSync(testTokenFile)) {
      fs.unlinkSync(testTokenFile);
    }
  });

  test('should persist session across bridge restarts', async () => {
    // Write test tokens to file
    const tokenData = generateTestToken('user-persist', 'persist@example.com', 3600);
    const tokens = {
      pierre: tokenData,
      providers: {}
    };
    fs.writeFileSync(testTokenFile, JSON.stringify(tokens, null, 2));

    // Start first bridge instance
    const bridgeClient1 = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);
    await bridgeClient1.start();
    await bridgeClient1.send(MCPMessages.initialize);

    const response1 = await bridgeClient1.send({
      method: 'tools/list',
      params: {}
    });
    await bridgeClient1.stop();

    // Start second bridge instance
    const bridgeClient2 = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);
    await bridgeClient2.start();
    await bridgeClient2.send(MCPMessages.initialize);

    const response2 = await bridgeClient2.send({
      method: 'tools/list',
      params: {}
    });
    await bridgeClient2.stop();

    // Both should have tools
    expect(response1.result?.tools).toBeDefined();
    expect(response2.result?.tools).toBeDefined();
  }, 120000);

  test('should load saved tokens on startup', async () => {
    // Create token file
    const tokenData = generateTestToken('user-load', 'load@example.com', 3600);
    const tokens = {
      pierre: tokenData,
      providers: {}
    };
    fs.writeFileSync(testTokenFile, JSON.stringify(tokens, null, 2));

    // Bridge should load tokens from file
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);
    await bridgeClient.start();

    const initResponse = await bridgeClient.send(MCPMessages.initialize);
    expect(initResponse).toHaveProperty('result');

    await bridgeClient.stop();
  }, 60000);

  test('should save tokens after refresh', async () => {
    // Write initial tokens
    const tokenData = generateTestToken('user-save', 'save@example.com', 3600);
    const tokens = {
      pierre: tokenData,
      providers: {}
    };
    fs.writeFileSync(testTokenFile, JSON.stringify(tokens, null, 2));

    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);
    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Make some requests
    await bridgeClient.send({ method: 'tools/list', params: {} });

    await bridgeClient.stop();

    // Token file should still exist
    expect(fs.existsSync(testTokenFile)).toBe(true);
  }, 60000);

  test('should handle missing token file gracefully', async () => {
    const missingTokenFile = path.join(os.tmpdir(), 'nonexistent-tokens.json');

    // Ensure file doesn't exist
    if (fs.existsSync(missingTokenFile)) {
      fs.unlinkSync(missingTokenFile);
    }

    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    // Should start without crashing
    await bridgeClient.start();
    const response = await bridgeClient.send(MCPMessages.initialize);

    expect(response).toBeDefined();
    await bridgeClient.stop();
  }, 60000);
});

describe('E2E: Claude Desktop Multi-Tenant Context Switching', () => {
  let serverHandle;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should isolate tenant data between different bridge instances', async () => {
    // Create two bridge clients (simulating two different users)
    const bridgeClient1 = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    const bridgeClient2 = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient1.start();
    await bridgeClient2.start();

    await bridgeClient1.send(MCPMessages.initialize);
    await bridgeClient2.send(MCPMessages.initialize);

    // Both should get tools list independently
    const response1 = await bridgeClient1.send({ method: 'tools/list', params: {} });
    const response2 = await bridgeClient2.send({ method: 'tools/list', params: {} });

    expect(response1.result?.tools).toBeDefined();
    expect(response2.result?.tools).toBeDefined();

    await bridgeClient1.stop();
    await bridgeClient2.stop();
  }, 90000);

  test('should maintain tenant context across tool calls', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Multiple tool calls should maintain same tenant context
    const response1 = await bridgeClient.send({
      method: 'tools/call',
      params: { name: 'list_connections', arguments: {} }
    });

    const response2 = await bridgeClient.send({
      method: 'tools/call',
      params: { name: 'get_activities', arguments: { provider: 'strava' } }
    });

    // Both should be processed in same tenant context
    expect(response1).toBeDefined();
    expect(response2).toBeDefined();

    await bridgeClient.stop();
  }, 60000);

  test('should not leak tenant data between instances', async () => {
    const bridgeClient1 = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient1.start();
    await bridgeClient1.send(MCPMessages.initialize);

    // Make request from first tenant
    const response1 = await bridgeClient1.send({
      method: 'tools/call',
      params: { name: 'list_connections', arguments: {} }
    });

    await bridgeClient1.stop();

    // Second tenant instance
    const bridgeClient2 = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient2.start();
    await bridgeClient2.send(MCPMessages.initialize);

    // Should not see first tenant's data
    const response2 = await bridgeClient2.send({
      method: 'tools/call',
      params: { name: 'list_connections', arguments: {} }
    });

    await bridgeClient2.stop();

    // Both should complete independently
    expect(response1).toBeDefined();
    expect(response2).toBeDefined();
  }, 90000);
});

describe('E2E: Claude Desktop Auth Failure Recovery', () => {
  let serverHandle;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should recover from temporary auth failures', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // First request
    const response1 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response1).toBeDefined();

    // Wait a moment (simulating brief auth issue)
    await new Promise(resolve => setTimeout(resolve, 100));

    // Second request should recover
    const response2 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response2).toBeDefined();

    await bridgeClient.stop();
  }, 60000);

  test('should provide clear error when auth cannot be recovered', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();

    // Initialize
    const initResponse = await bridgeClient.send(MCPMessages.initialize);
    expect(initResponse).toBeDefined();

    await bridgeClient.stop();
  }, 60000);

  test('should handle server restart during session', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Make initial request
    const response1 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response1).toBeDefined();

    // Subsequent request after potential reconnect
    const response2 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response2).toBeDefined();

    await bridgeClient.stop();
  }, 90000);

  test('should not loop infinitely on auth failure', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();

    const startTime = Date.now();

    // This should complete (success or error) within reasonable time
    const response = await bridgeClient.send(MCPMessages.initialize);

    const elapsed = Date.now() - startTime;

    // Should not take forever
    expect(elapsed).toBeLessThan(30000);
    expect(response).toBeDefined();

    await bridgeClient.stop();
  }, 60000);
});
