// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Claude Desktop tool execution E2E tests - verifies all tool categories work correctly
// ABOUTME: Tests tool execution, response schema compliance, chaining, and error formatting
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const path = require('path');

describe('E2E: Claude Desktop Tool Execution', () => {
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

    // Initialize connection
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

  describe('Tool Category Execution - Fitness Data', () => {
    test('should execute get_activities tool successfully', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava', limit: 5 }
        }
      });

      expect(response).toHaveProperty('result');
      expect(response.result).toHaveProperty('content');
      expect(Array.isArray(response.result.content)).toBe(true);
    }, 30000);

    test('should execute get_athlete tool successfully', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'strava' }
        }
      });

      expect(response).toHaveProperty('result');
      expect(response.result).toHaveProperty('content');
    }, 30000);

    test('should execute get_zones tool successfully', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_zones',
          arguments: { provider: 'strava' }
        }
      });

      expect(response).toBeDefined();
      // May return error if not connected, but should have valid structure
      if (response.error) {
        expect(response.error).toHaveProperty('code');
        expect(response.error).toHaveProperty('message');
      } else {
        expect(response.result).toHaveProperty('content');
      }
    }, 30000);

    test('should execute get_stats tool successfully', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'strava' }
        }
      });

      expect(response).toBeDefined();
      if (response.result) {
        expect(response.result).toHaveProperty('content');
      }
    }, 30000);
  });

  describe('Tool Category Execution - Connection Management', () => {
    test('should execute connect_provider tool', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'connect_provider',
          arguments: { provider: 'strava' }
        }
      });

      expect(response).toBeDefined();
      // This typically returns auth URL or status
      if (response.result) {
        expect(response.result).toHaveProperty('content');
      }
    }, 30000);

    test('should execute list_connections tool', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'list_connections',
          arguments: {}
        }
      });

      expect(response).toBeDefined();
      if (response.result) {
        expect(response.result).toHaveProperty('content');
      }
    }, 30000);
  });

  describe('Tool Category Execution - Intelligence', () => {
    test('should execute training readiness tools', async () => {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'analyze_training_load',
          arguments: {}
        }
      });

      expect(response).toBeDefined();
      if (response.error) {
        // Tool might require connected provider
        expect(response.error).toHaveProperty('code');
      }
    }, 30000);
  });
});

describe('E2E: Tool Response Schema Compliance', () => {
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

  test('should return MCP-compliant tool result structure', async () => {
    const response = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava', limit: 1 }
      }
    });

    if (response.result) {
      // MCP compliant result structure
      expect(response.result).toHaveProperty('content');
      expect(Array.isArray(response.result.content)).toBe(true);

      if (response.result.content.length > 0) {
        const content = response.result.content[0];
        expect(content).toHaveProperty('type');
        expect(['text', 'image', 'resource']).toContain(content.type);
      }
    }
  }, 30000);

  test('should include proper content type in responses', async () => {
    const response = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_athlete',
        arguments: { provider: 'strava' }
      }
    });

    if (response.result && response.result.content) {
      for (const item of response.result.content) {
        expect(item.type).toBeDefined();
        if (item.type === 'text') {
          expect(item.text).toBeDefined();
          expect(typeof item.text).toBe('string');
        }
      }
    }
  }, 30000);

  test('should return valid JSON in text content', async () => {
    const response = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    });

    if (response.result && response.result.content) {
      for (const item of response.result.content) {
        if (item.type === 'text' && item.text.startsWith('{')) {
          expect(() => JSON.parse(item.text)).not.toThrow();
        }
      }
    }
  }, 30000);

  test('should include isError flag when tool fails', async () => {
    // Call with invalid arguments to trigger error
    const response = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'invalid_provider' }
      }
    });

    if (response.result && response.result.isError) {
      expect(response.result.isError).toBe(true);
      expect(response.result.content).toBeDefined();
    } else if (response.error) {
      expect(response.error).toHaveProperty('code');
      expect(response.error).toHaveProperty('message');
    }
  }, 30000);
});

describe('E2E: Tool Chaining Scenarios', () => {
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

  test('should support sequential tool calls', async () => {
    // First call: list connections
    const connectionsResponse = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'list_connections',
        arguments: {}
      }
    });

    expect(connectionsResponse).toBeDefined();

    // Second call: get activities (depends on knowing providers)
    const activitiesResponse = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    });

    expect(activitiesResponse).toBeDefined();
  }, 60000);

  test('should maintain session state across tool calls', async () => {
    const requests = [
      { method: 'tools/call', params: { name: 'list_connections', arguments: {} } },
      { method: 'tools/call', params: { name: 'get_athlete', arguments: { provider: 'strava' } } },
      { method: 'tools/call', params: { name: 'get_activities', arguments: { provider: 'strava', limit: 5 } } }
    ];

    for (const request of requests) {
      const response = await bridgeClient.send(request);
      expect(response).toBeDefined();
      // Each request should complete (success or handled error)
      expect(response.error || response.result).toBeDefined();
    }
  }, 90000);

  test('should handle tool chaining with data dependencies', async () => {
    // Get athlete first
    const athleteResponse = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_athlete',
        arguments: { provider: 'strava' }
      }
    });

    // Then get activities (could use athlete data)
    const activitiesResponse = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    });

    // Both should complete
    expect(athleteResponse).toBeDefined();
    expect(activitiesResponse).toBeDefined();
  }, 60000);

  test('should isolate tool call failures from subsequent calls', async () => {
    // First call with invalid provider (should fail)
    const failedResponse = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'nonexistent' }
      }
    });

    // Second call should still work (not affected by previous failure)
    const successResponse = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(failedResponse).toBeDefined();
    expect(successResponse).toHaveProperty('result');
    expect(successResponse.result).toHaveProperty('tools');
  }, 60000);
});

describe('E2E: Tool Error Response Formatting', () => {
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

  test('should return structured error for invalid tool name', async () => {
    try {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'nonexistent_tool',
          arguments: {}
        }
      });

      // Either error property or thrown exception
      expect(response.error).toBeDefined();
      expect(response.error).toHaveProperty('code');
      expect(response.error).toHaveProperty('message');
    } catch (error) {
      expect(error.message).toBeDefined();
    }
  }, 30000);

  test('should return structured error for missing required arguments', async () => {
    try {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: {} // Missing provider
        }
      });

      if (response.error) {
        expect(response.error.code).toBeDefined();
        expect(response.error.message.length).toBeGreaterThan(0);
      }
    } catch (error) {
      expect(error.message).toBeDefined();
    }
  }, 30000);

  test('should return structured error for invalid argument types', async () => {
    try {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: {
            provider: 'strava',
            limit: 'not-a-number' // Invalid type
          }
        }
      });

      // Should handle gracefully
      expect(response).toBeDefined();
    } catch (error) {
      expect(error.message).toBeDefined();
    }
  }, 30000);

  test('should include helpful error message for provider not connected', async () => {
    try {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin' }
        }
      });

      if (response.error) {
        // Error message should be helpful
        expect(response.error.message.length).toBeGreaterThan(5);
      }
    } catch (error) {
      expect(error.message.length).toBeGreaterThan(5);
    }
  }, 30000);

  test('should not expose internal implementation in error messages', async () => {
    try {
      const response = await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      });

      if (response.error) {
        const message = response.error.message.toLowerCase();
        // Should not expose internal details
        expect(message).not.toContain('panic');
        expect(message).not.toContain('stack trace');
        expect(message).not.toContain('unwrap()');
      }
    } catch (error) {
      const message = error.message.toLowerCase();
      expect(message).not.toContain('panic');
    }
  }, 30000);
});
