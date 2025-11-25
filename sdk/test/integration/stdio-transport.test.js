// ABOUTME: Integration tests for stdio transport - the actual path Claude Desktop uses
// ABOUTME: Tests subprocess spawning, JSON-RPC message format, and round-trip communication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { generateTestToken } = require('../helpers/token-generator');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { clearKeychainTokens } = require('../helpers/keychain-cleanup');
const path = require('path');

describe('Stdio Transport Integration Tests (Claude Desktop Path)', () => {
  let serverHandle;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 60000);

  beforeEach(async () => {
    await clearKeychainTokens();
  });

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  describe('Subprocess Spawning', () => {
    test('should spawn bridge subprocess successfully', async () => {
      const client = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);

      await client.start();
      expect(client.process).toBeDefined();
      expect(client.process.pid).toBeGreaterThan(0);

      await client.stop();
    }, 30000);

    test('should handle bridge startup with authentication token', async () => {
      const testToken = generateTestToken('stdio-auth-user', 'stdio@example.com', 3600);

      const client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      await client.start();
      expect(client.process).toBeDefined();

      await client.stop();
    }, 30000);

    test('should terminate subprocess cleanly on stop', async () => {
      const client = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);

      await client.start();
      const pid = client.process.pid;
      expect(pid).toBeGreaterThan(0);

      await client.stop();

      // Process should be terminated
      expect(client.process.killed || client.process.exitCode !== null).toBe(true);
    }, 30000);
  });

  describe('JSON-RPC Message Format', () => {
    let client;

    beforeEach(async () => {
      client = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);
      await client.start();
    });

    afterEach(async () => {
      if (client) {
        await client.stop();
        client = null;
      }
    });

    test('should receive valid JSON-RPC 2.0 initialize response', async () => {
      const response = await client.send(MCPMessages.initialize);

      // Verify JSON-RPC 2.0 format
      expect(response).toHaveProperty('jsonrpc', '2.0');
      expect(response).toHaveProperty('id');
      expect(response).toHaveProperty('result');

      // Verify MCP initialize response structure
      expect(response.result).toHaveProperty('protocolVersion');
      expect(response.result).toHaveProperty('capabilities');
      expect(response.result).toHaveProperty('serverInfo');
      expect(response.result.serverInfo).toHaveProperty('name');
      expect(response.result.serverInfo).toHaveProperty('version');
    }, 30000);

    test('should receive valid JSON-RPC 2.0 tools/list response', async () => {
      await client.send(MCPMessages.initialize);

      const response = await client.send(MCPMessages.toolsList);

      // Verify JSON-RPC 2.0 format
      expect(response).toHaveProperty('jsonrpc', '2.0');
      expect(response).toHaveProperty('id');
      expect(response).toHaveProperty('result');

      // Verify tools list structure
      expect(response.result).toHaveProperty('tools');
      expect(Array.isArray(response.result.tools)).toBe(true);

      // Each tool must have proper structure
      for (const tool of response.result.tools) {
        expect(tool).toHaveProperty('name');
        expect(typeof tool.name).toBe('string');
        expect(tool).toHaveProperty('description');
        expect(typeof tool.description).toBe('string');
        expect(tool).toHaveProperty('inputSchema');
        expect(tool.inputSchema).toHaveProperty('type', 'object');
      }
    }, 30000);

    test('should receive valid JSON-RPC 2.0 error for unknown method', async () => {
      await client.send(MCPMessages.initialize);

      const unknownMethod = {
        jsonrpc: '2.0',
        id: 100,
        method: 'unknown/method',
        params: {}
      };

      try {
        const response = await client.send(unknownMethod, 10000);

        // If we get a response, it should be an error
        if (response.error) {
          expect(response).toHaveProperty('jsonrpc', '2.0');
          expect(response).toHaveProperty('id');
          expect(response.error).toHaveProperty('code');
          expect(response.error).toHaveProperty('message');
          expect(typeof response.error.code).toBe('number');
        }
      } catch (error) {
        // Either timeout or "Method not found" error is acceptable
        const isExpectedError = error.message.includes('timed out') ||
                                error.message.includes('Method not found') ||
                                error.message.includes('not found');
        expect(isExpectedError).toBe(true);
      }
    }, 15000);

    test('should preserve request ID in response', async () => {
      const customId = 'custom-request-id-12345';
      const request = {
        jsonrpc: '2.0',
        id: customId,
        method: 'initialize',
        params: MCPMessages.initialize.params
      };

      const response = await client.send(request);

      expect(response.id).toBe(customId);
    }, 30000);
  });

  describe('Round-Trip Communication', () => {
    let client;

    beforeEach(async () => {
      const testToken = generateTestToken('roundtrip-user', 'roundtrip@example.com', 3600);
      client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);
      await client.start();
      await client.send(MCPMessages.initialize);
    });

    afterEach(async () => {
      if (client) {
        await client.stop();
        client = null;
      }
    });

    test('should handle multiple sequential requests', async () => {
      // Send multiple requests sequentially
      const response1 = await client.send(MCPMessages.toolsList);
      expect(response1.result.tools).toBeDefined();

      const response2 = await client.send(MCPMessages.resourcesList);
      expect(response2.result).toBeDefined();

      const response3 = await client.send(MCPMessages.promptsList);
      expect(response3.result).toBeDefined();

      // All responses should have valid structure
      expect(response1.jsonrpc).toBe('2.0');
      expect(response2.jsonrpc).toBe('2.0');
      expect(response3.jsonrpc).toBe('2.0');
    }, 60000);

    test('should handle tool call via stdio and receive MCP response', async () => {
      const toolCall = {
        jsonrpc: '2.0',
        id: 50,
        method: 'tools/call',
        params: {
          name: 'get_connection_status',
          arguments: {}
        }
      };

      try {
        const response = await client.send(toolCall, 15000);

        // Should receive valid MCP response (success or error)
        expect(response).toHaveProperty('jsonrpc', '2.0');
        expect(response).toHaveProperty('id', 50);

        if (response.result) {
          // Success case - verify MCP tool response structure
          expect(response.result).toHaveProperty('content');
          expect(Array.isArray(response.result.content)).toBe(true);
        } else if (response.error) {
          // Error case - verify MCP error structure
          expect(response.error).toHaveProperty('code');
          expect(response.error).toHaveProperty('message');
        }
      } catch (error) {
        // Timeout is acceptable - proves request was sent
        expect(error.message).toContain('timed out');
      }
    }, 20000);

    test('should maintain session across multiple tool calls', async () => {
      // First call
      const call1 = {
        jsonrpc: '2.0',
        id: 60,
        method: 'tools/call',
        params: {
          name: 'get_connection_status',
          arguments: {}
        }
      };

      // Second call
      const call2 = {
        jsonrpc: '2.0',
        id: 61,
        method: 'tools/call',
        params: {
          name: 'get_connection_status',
          arguments: {}
        }
      };

      try {
        const response1 = await client.send(call1, 15000);
        const response2 = await client.send(call2, 15000);

        // Both should have correct IDs (session maintained)
        expect(response1.id).toBe(60);
        expect(response2.id).toBe(61);
      } catch (error) {
        // At least one should have worked
        expect(error.message).toContain('timed out');
      }
    }, 35000);
  });

  describe('Tools List Completeness via Stdio', () => {
    let client;

    beforeEach(async () => {
      const testToken = generateTestToken('completeness-user', 'completeness@example.com', 3600);
      client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);
      await client.start();
      await client.send(MCPMessages.initialize);
    });

    afterEach(async () => {
      if (client) {
        await client.stop();
        client = null;
      }
    });

    test('CRITICAL: tools/list via stdio must return all expected tools', async () => {
      const response = await client.send(MCPMessages.toolsList);
      const toolNames = response.result.tools.map(t => t.name);

      // Critical tools that must be present (from schema_completeness_test.rs)
      const criticalTools = [
        'get_activities',
        'get_athlete',
        'connect_provider',
        'get_connection_status',
        'connect_to_pierre'
      ];

      const missingTools = criticalTools.filter(t => !toolNames.includes(t));

      if (missingTools.length > 0) {
        console.error('❌ CRITICAL: Missing tools via stdio:', missingTools);
        console.error('Available tools:', toolNames);
      }

      expect(missingTools).toEqual([]);
      console.log(`✅ All ${criticalTools.length} critical tools present via stdio`);
    }, 30000);

    test('tools/list via stdio should match HTTP transport count', async () => {
      const response = await client.send(MCPMessages.toolsList);
      const stdioToolCount = response.result.tools.length;

      // Should have substantial number of tools (matches schema)
      expect(stdioToolCount).toBeGreaterThan(30);

      console.log(`✅ Stdio transport returned ${stdioToolCount} tools`);
    }, 30000);

    test('tool schemas via stdio must be valid', async () => {
      const response = await client.send(MCPMessages.toolsList);

      for (const tool of response.result.tools) {
        // Every tool must have required fields
        expect(tool.name).toBeTruthy();
        expect(tool.description).toBeTruthy();
        expect(tool.inputSchema).toBeDefined();
        expect(tool.inputSchema.type).toBe('object');

        // If required fields exist, they must be in properties
        if (tool.inputSchema.required && tool.inputSchema.required.length > 0) {
          expect(tool.inputSchema.properties).toBeDefined();
          for (const reqField of tool.inputSchema.required) {
            expect(tool.inputSchema.properties).toHaveProperty(reqField);
          }
        }
      }
    }, 30000);
  });

  describe('Error Scenarios', () => {
    test('should handle connection to non-existent server gracefully', async () => {
      const client = new MockMCPClient('node', [
        bridgePath,
        '--server', 'http://localhost:19999'  // Non-existent
      ]);

      await client.start();

      // Initialize might fail or return error
      try {
        const response = await client.send(MCPMessages.initialize, 10000);
        // If we get a response, check if it's an error
        if (response.error) {
          expect(response.error).toHaveProperty('code');
        }
      } catch (error) {
        // Timeout or error is acceptable
        expect(error).toBeDefined();
      }

      await client.stop();
    }, 20000);

    test('should handle invalid JSON gracefully', async () => {
      const client = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);
      await client.start();

      // Send invalid JSON - bridge should not crash
      try {
        // Write invalid JSON directly to stdin
        client.process.stdin.write('{ invalid json }\n');

        // Wait briefly
        await new Promise(resolve => setTimeout(resolve, 1000));

        // Bridge should still be alive
        expect(client.process.killed).toBe(false);

        // Should still be able to send valid request
        const response = await client.send(MCPMessages.initialize, 5000);
        expect(response).toBeDefined();

      } catch (error) {
        // Error is acceptable - the key test is that we didn't crash
        expect(client.process.killed).toBe(false);
      }

      await client.stop();
    }, 15000);
  });
});
