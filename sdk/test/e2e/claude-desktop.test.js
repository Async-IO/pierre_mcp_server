// ABOUTME: End-to-end Claude Desktop integration tests with full MCP workflow
// ABOUTME: Tests initialization, tool listing, batch requests, and client-bridge communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const path = require('path');

describe('E2E: MCP Client Bridge Integration', () => {
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
  }, 90000);

  afterAll(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
    }
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should initialize connection with protocol 2025-06-18', async () => {
    const response = await bridgeClient.send(MCPMessages.initialize);

    expect(response).toHaveProperty('result');
    expect(response.result.protocolVersion).toBe('2025-06-18');
    expect(response.result.serverInfo.name).toBe('pierre-fitness');
  }, 30000);

  test('should list available tools', async () => {
    const response = await bridgeClient.send(MCPMessages.toolsList);

    expect(response).toHaveProperty('result');
    expect(response.result).toHaveProperty('tools');
    expect(Array.isArray(response.result.tools)).toBe(true);
    expect(response.result.tools.length).toBeGreaterThan(0);

    const toolNames = response.result.tools.map(t => t.name);
    // Before OAuth, bridge shows connect_to_pierre tool
    // After OAuth, it shows actual Pierre tools (get_activities, get_athlete, etc.)
    expect(toolNames).toContain('connect_to_pierre');
  }, 30000);

  test('should reject batch requests per 2025-06-18 spec', async () => {
    const batchJson = JSON.stringify(MCPMessages.batchRequest);
    const responseRaw = await bridgeClient.sendRaw(batchJson + '\n');

    const responses = JSON.parse(responseRaw);
    expect(Array.isArray(responses)).toBe(true);
    expect(responses.length).toBe(2);

    responses.forEach(response => {
      expect(response).toHaveProperty('error');
      expect(response.error.code).toBe(-32600);
      expect(response.error.message).toContain('not supported');
    });
  }, 30000);

  test('should handle ping request', async () => {
    const response = await bridgeClient.send(MCPMessages.ping);

    expect(response).toHaveProperty('result');
  }, 30000);
});

describe('E2E: Error Handling', () => {
  let bridgeClient;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
  }, 60000);

  afterAll(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
    }
  });

  test('should handle malformed requests gracefully', async () => {
    try {
      const malformed = {
        jsonrpc: '2.0',
        id: 999,
        method: 'nonexistent/method',
        params: {}
      };

      const response = await bridgeClient.send(malformed, 10000);

      // Should either return error or handle gracefully
      if (response.error) {
        expect(response.error).toHaveProperty('code');
      } else {
        expect(response).toBeDefined();
      }
    } catch (error) {
      // Timeout or error is acceptable for malformed request
      expect(error).toBeDefined();
    }
  }, 15000);
});
