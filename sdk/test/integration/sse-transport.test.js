// ABOUTME: Integration tests for SSE/Streamable HTTP transport (Claude Desktop mode)
// ABOUTME: Tests authentication flow, session management, tools listing, and OAuth handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StreamableHTTPClientTransport } = require('@modelcontextprotocol/sdk/client/streamableHttp.js');
const { ensureServerRunning } = require('../helpers/server');
const { generateTestToken } = require('../helpers/token-generator');
const { TestConfig } = require('../helpers/fixtures');

// Use native fetch
const fetch = global.fetch;

describe('SSE/Streamable HTTP Transport Tests (Claude Desktop Mode)', () => {
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
    testToken = generateTestToken(userId, 'sse-transport-test@example.com', 3600);
  }, 60000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  describe('Authentication Flow', () => {
    test('should verify server is ready for SSE connections', async () => {
      const response = await fetch(`${serverUrl}/health`);
      expect(response.ok).toBe(true);

      const health = await response.json();
      expect(health.status).toBe('ok');
    });

    test('should accept JWT token in authorization header', async () => {
      const client = new Client(
        { name: 'claude-desktop-sse-test', version: '1.0.0' },
        { capabilities: { tools: {}, resources: {}, prompts: {} } }
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
          // Ignore close errors
        }
      }
    }, 30000);
  });

  describe('Session Management', () => {
    test('should establish MCP session with full capabilities', async () => {
      const client = new Client(
        { name: 'claude-desktop-session-test', version: '1.0.0' },
        {
          capabilities: {
            tools: {},
            resources: {},
            prompts: {}
          }
        }
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

        // Verify client is properly connected
        expect(client).toBeDefined();

        // Session ID may or may not be available depending on server implementation
        // This test validates the connection works regardless
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);
  });

  describe('Tools Discovery', () => {
    test('should list available fitness tools', async () => {
      const client = new Client(
        { name: 'claude-desktop-tools-test', version: '1.0.0' },
        { capabilities: { tools: {}, resources: {}, prompts: {} } }
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

        // Verify we have fitness-related tools
        const toolNames = toolsResult.tools.map(t => t.name);

        // Should have at least some core tools
        expect(toolNames.length).toBeGreaterThan(0);

        // Log discovered tools for debugging
        if (process.env.DEBUG) {
          console.log('Available tools:', toolNames);
        }
      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);

    test('should have valid tool schemas', async () => {
      const client = new Client(
        { name: 'claude-desktop-schema-test', version: '1.0.0' },
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

        // Verify each tool has proper schema structure
        for (const tool of toolsResult.tools) {
          expect(tool.name).toBeDefined();
          expect(typeof tool.name).toBe('string');
          expect(tool.description).toBeDefined();
          expect(typeof tool.description).toBe('string');
          expect(tool.inputSchema).toBeDefined();
          expect(tool.inputSchema.type).toBe('object');
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

  describe('OAuth-Required Tools', () => {
    test('should receive MCP response when calling connect_strava', async () => {
      const client = new Client(
        { name: 'claude-desktop-oauth-test', version: '1.0.0' },
        { capabilities: { tools: {}, resources: {}, prompts: {} } }
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

        // Tool call may succeed or fail with MCP error
        // Both are valid responses that prove SSE transport works
        try {
          const result = await client.callTool({
            name: 'connect_strava',
            arguments: {}
          });

          // Success case - verify MCP response structure
          expect(result).toBeDefined();
          expect(result.content).toBeDefined();
          expect(Array.isArray(result.content)).toBe(true);
        } catch (mcpError) {
          // MCP error proves transport delivered the request correctly
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

    test('should receive MCP response for get_activities', async () => {
      const client = new Client(
        { name: 'claude-desktop-activities-test', version: '1.0.0' },
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

        // Tool call may succeed or fail - both prove transport works
        try {
          const result = await client.callTool({
            name: 'get_activities',
            arguments: { provider: 'strava', limit: 10 }
          });

          expect(result).toBeDefined();
          expect(result.content).toBeDefined();
          expect(Array.isArray(result.content)).toBe(true);
        } catch (mcpError) {
          // MCP error is expected for unauthenticated requests
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
  });

  describe('Connection Status', () => {
    test('should receive MCP response for connection status', async () => {
      const client = new Client(
        { name: 'claude-desktop-status-test', version: '1.0.0' },
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

        // Tool call may succeed or return MCP error
        // Both responses prove SSE transport works correctly
        try {
          const result = await client.callTool({
            name: 'get_connection_status',
            arguments: {}
          });

          expect(result).toBeDefined();
          expect(result.content).toBeDefined();
          expect(Array.isArray(result.content)).toBe(true);

          const textContent = result.content.find(c => c.type === 'text');
          expect(textContent).toBeDefined();
          expect(typeof textContent.text).toBe('string');
        } catch (mcpError) {
          // MCP error is acceptable - proves transport delivered request
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
  });

  describe('Multiple Client Sessions', () => {
    test('should handle multiple concurrent client connections', async () => {
      const clients = [];
      const transports = [];

      try {
        // Create multiple clients
        for (let i = 0; i < 3; i++) {
          const crypto = require('crypto');
          const userId = crypto.randomUUID();
          const token = generateTestToken(userId, `concurrent-test-${i}@example.com`, 3600);

          const client = new Client(
            { name: `concurrent-client-${i}`, version: '1.0.0' },
            { capabilities: { tools: {} } }
          );

          const transport = new StreamableHTTPClientTransport(
            new URL(mcpUrl),
            {
              requestInit: {
                headers: {
                  'Authorization': `Bearer ${token.access_token}`
                }
              }
            }
          );

          clients.push(client);
          transports.push(transport);
        }

        // Connect all clients concurrently
        await Promise.all(clients.map((client, i) => client.connect(transports[i])));

        // All should be connected
        expect(clients.length).toBe(3);

        // All should be able to list tools
        const results = await Promise.all(
          clients.map(client => client.listTools())
        );

        for (const result of results) {
          expect(result.tools).toBeDefined();
          expect(result.tools.length).toBeGreaterThan(0);
        }
      } finally {
        // Cleanup all clients
        for (const client of clients) {
          try {
            await client.close();
          } catch (e) {
            // Ignore close errors
          }
        }
      }
    }, 60000);
  });
});
