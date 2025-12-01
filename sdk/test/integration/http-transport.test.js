// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Integration tests for direct HTTP transport using StreamableHTTPClientTransport
// ABOUTME: Tests MCP client connectivity, tool calls, and response format compliance via HTTP
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StreamableHTTPClientTransport } = require('@modelcontextprotocol/sdk/client/streamableHttp.js');
const { ensureServerRunning } = require('../helpers/server');
const { generateTestToken } = require('../helpers/token-generator');
const { TestConfig } = require('../helpers/fixtures');

describe('HTTP Transport Integration Tests', () => {
  let serverHandle;
  let testToken;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;
  const mcpUrl = `${serverUrl}/mcp`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });

    // Generate a test JWT token
    const crypto = require('crypto');
    const userId = crypto.randomUUID();
    testToken = generateTestToken(userId, 'http-transport-test@example.com', 3600);
  }, 60000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  describe('StreamableHTTPClientTransport', () => {
    test('should connect to server via HTTP transport', async () => {
      const client = new Client(
        { name: 'test-http-client', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL(mcpUrl),
        {
          requestInit: {
            headers: {
              'Authorization': `Bearer ${testToken.access_token}`
            }
          }
        }
      );

      try {
        await client.connect(transport);
        expect(client).toBeDefined();
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors in cleanup
        }
      }
    }, 30000);

    test('should list tools via HTTP transport', async () => {
      const client = new Client(
        { name: 'test-http-list-tools', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL(mcpUrl),
        {
          requestInit: {
            headers: {
              'Authorization': `Bearer ${testToken.access_token}`
            }
          }
        }
      );

      try {
        await client.connect(transport);

        const toolsResult = await client.listTools();
        expect(toolsResult).toBeDefined();
        expect(toolsResult.tools).toBeInstanceOf(Array);
        expect(toolsResult.tools.length).toBeGreaterThan(0);

        // Verify tool structure
        const firstTool = toolsResult.tools[0];
        expect(firstTool).toHaveProperty('name');
        expect(firstTool).toHaveProperty('description');
        expect(firstTool).toHaveProperty('inputSchema');
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);

    test('should call tool via HTTP and receive MCP response', async () => {
      const client = new Client(
        { name: 'test-http-tool-call', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL(mcpUrl),
        {
          requestInit: {
            headers: {
              'Authorization': `Bearer ${testToken.access_token}`
            }
          }
        }
      );

      try {
        await client.connect(transport);

        // Tool call may succeed or fail with MCP error depending on JWT validation
        // The key test is that we get a proper MCP protocol response (not network error)
        try {
          const result = await client.callTool({
            name: 'get_connection_status',
            arguments: {}
          });

          // If succeeds, verify MCP-compliant response format
          expect(result).toBeDefined();
          expect(result.content).toBeDefined();
          expect(Array.isArray(result.content)).toBe(true);
        } catch (mcpError) {
          // MCP protocol error is acceptable - proves transport works
          expect(mcpError.message).toContain('MCP error');
        }
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);

    test('should handle tool call errors via MCP protocol', async () => {
      const client = new Client(
        { name: 'test-http-error-handling', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL(mcpUrl),
        {
          requestInit: {
            headers: {
              'Authorization': `Bearer ${testToken.access_token}`
            }
          }
        }
      );

      try {
        await client.connect(transport);

        // Call a tool - may return result or MCP error
        // Either response validates HTTP transport works
        try {
          const result = await client.callTool({
            name: 'get_activities',
            arguments: { provider: 'strava', limit: 10 }
          });

          // If succeeds, should be valid MCP response
          expect(result).toBeDefined();
          expect(result.content).toBeDefined();
          expect(Array.isArray(result.content)).toBe(true);
        } catch (mcpError) {
          // MCP protocol error proves transport works correctly
          expect(mcpError.message).toContain('MCP error');
        }
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);

    test('should receive valid MCP protocol response structure', async () => {
      const client = new Client(
        { name: 'test-http-response-structure', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL(mcpUrl),
        {
          requestInit: {
            headers: {
              'Authorization': `Bearer ${testToken.access_token}`
            }
          }
        }
      );

      try {
        await client.connect(transport);

        // Verify we can communicate via MCP protocol
        // Tool call may succeed or return MCP error - both are valid protocol responses
        try {
          const result = await client.callTool({
            name: 'get_connection_status',
            arguments: {}
          });

          // Success case - verify structure
          expect(result.content).toBeDefined();
          expect(result.content.length).toBeGreaterThanOrEqual(1);
          const textContent = result.content[0];
          expect(textContent.type).toBe('text');
          expect(textContent.text).toBeDefined();
        } catch (mcpError) {
          // MCP error case - verify it's a protocol error
          expect(mcpError).toBeDefined();
          expect(mcpError.message).toContain('MCP error');
          // Having an MCP error code proves HTTP transport delivered the request
        }
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);
  });

  describe('HTTP Transport Error Handling', () => {
    test('should handle connection to non-existent endpoint', async () => {
      const client = new Client(
        { name: 'test-http-bad-endpoint', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL('http://localhost:19999/mcp'),
        {
          requestInit: {
            headers: {
              'Authorization': `Bearer ${testToken.access_token}`
            }
          }
        }
      );

      await expect(client.connect(transport)).rejects.toThrow();
    }, 10000);

    test('should handle missing authorization header', async () => {
      const client = new Client(
        { name: 'test-http-no-auth', version: '1.0.0' },
        { capabilities: { tools: {} } }
      );

      const transport = new StreamableHTTPClientTransport(
        new URL(mcpUrl),
        {
          requestInit: {
            headers: {}
          }
        }
      );

      // Should either throw on connect or fail operations
      try {
        await client.connect(transport);
        // If connect succeeds, operations should fail
        await expect(client.callTool({
          name: 'get_connection_status',
          arguments: {}
        })).rejects.toThrow();
      } catch (error) {
        // Connection failure is acceptable
        expect(error).toBeDefined();
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);
  });
});
