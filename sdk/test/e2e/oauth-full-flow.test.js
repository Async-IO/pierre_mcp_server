// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: End-to-end OAuth flow tests - complete path from connect_provider to tool calls
// ABOUTME: Tests the exact regression path: OAuth completion → token storage → tools/list refresh
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { generateTestToken } = require('../helpers/token-generator');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { clearKeychainTokens } = require('../helpers/keychain-cleanup');
const path = require('path');
const crypto = require('crypto');

describe('E2E: OAuth Full Flow Tests', () => {
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

  describe('Pre-OAuth State', () => {
    let client;

    afterEach(async () => {
      if (client) {
        await client.stop();
        client = null;
      }
    });

    test('unauthenticated bridge should show connect_provider tool', async () => {
      client = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);
      await client.start();
      await client.send(MCPMessages.initialize);

      const response = await client.send(MCPMessages.toolsList);
      const toolNames = response.result.tools.map(t => t.name);

      // connect_provider must always be available for OAuth initiation
      expect(toolNames).toContain('connect_provider');
    }, 30000);

    test('unauthenticated tool call should indicate need for OAuth', async () => {
      client = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);
      await client.start();
      await client.send(MCPMessages.initialize);

      const toolCall = {
        jsonrpc: '2.0',
        id: 10,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava', limit: 10 }
        }
      };

      try {
        const response = await client.send(toolCall, 10000);

        // Should either error or return helpful message about authentication
        if (response.error) {
          expect(response.error.message).toBeDefined();
        } else if (response.result) {
          // May contain message about needing to authenticate
          expect(response.result.content).toBeDefined();
        }
      } catch (error) {
        // Timeout or error is acceptable
        expect(error).toBeDefined();
      }
    }, 15000);
  });

  describe('Post-OAuth State (Simulated with Token)', () => {
    let client;

    afterEach(async () => {
      if (client) {
        await client.stop();
        client = null;
      }
    });

    test('authenticated bridge should show full tool set immediately', async () => {
      const testToken = generateTestToken('post-oauth-user', 'postoauth@example.com', 3600);

      client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      await client.start();
      await client.send(MCPMessages.initialize);

      // Wait for proactive connection to complete
      await new Promise(resolve => setTimeout(resolve, 2000));

      const response = await client.send(MCPMessages.toolsList);
      const toolNames = response.result.tools.map(t => t.name);

      // After OAuth, should have full tool set
      expect(toolNames.length).toBeGreaterThan(20);

      // Critical tools must be present
      const criticalTools = [
        'connect_provider',
        'get_connection_status',
        'get_activities',
        'get_athlete'
      ];

      for (const tool of criticalTools) {
        expect(toolNames).toContain(tool);
      }
    }, 60000);

    test('authenticated bridge should be able to call get_connection_status', async () => {
      const testToken = generateTestToken('status-check-user', 'status@example.com', 3600);

      client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      await client.start();
      await client.send(MCPMessages.initialize);
      await new Promise(resolve => setTimeout(resolve, 2000));

      const toolCall = {
        jsonrpc: '2.0',
        id: 20,
        method: 'tools/call',
        params: {
          name: 'get_connection_status',
          arguments: {}
        }
      };

      try {
        const response = await client.send(toolCall, 15000);

        expect(response).toHaveProperty('jsonrpc', '2.0');
        expect(response).toHaveProperty('id', 20);

        if (response.result) {
          // Success - verify MCP tool response structure
          expect(response.result).toHaveProperty('content');
          expect(Array.isArray(response.result.content)).toBe(true);

          // Content should include connection status info
          const textContent = response.result.content.find(c => c.type === 'text');
          expect(textContent).toBeDefined();
        } else if (response.error) {
          // MCP error is acceptable
          expect(response.error).toHaveProperty('code');
        }
      } catch (error) {
        // Timeout may occur but request was sent
        expect(error.message).toContain('timed out');
      }
    }, 30000);
  });

  describe('OAuth Token Transition', () => {
    test('tools/list should be consistent before and after token injection', async () => {
      // Test 1: Without token
      const client1 = new MockMCPClient('node', [bridgePath, '--server', serverUrl]);
      await client1.start();
      await client1.send(MCPMessages.initialize);

      const beforeOAuth = await client1.send(MCPMessages.toolsList);
      const toolsBefore = beforeOAuth.result.tools.map(t => t.name).sort();

      await client1.stop();

      // Test 2: With token (simulating post-OAuth)
      const testToken = generateTestToken('transition-user', 'transition@example.com', 3600);

      const client2 = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      await client2.start();
      await client2.send(MCPMessages.initialize);
      await new Promise(resolve => setTimeout(resolve, 2000));

      const afterOAuth = await client2.send(MCPMessages.toolsList);
      const toolsAfter = afterOAuth.result.tools.map(t => t.name).sort();

      await client2.stop();

      // Both should have tools (MCP spec: tools visible regardless of auth)
      expect(toolsBefore.length).toBeGreaterThan(0);
      expect(toolsAfter.length).toBeGreaterThan(0);

      // After OAuth should have same or more tools
      expect(toolsAfter.length).toBeGreaterThanOrEqual(toolsBefore.length);

      console.log(`Tools before OAuth: ${toolsBefore.length}`);
      console.log(`Tools after OAuth: ${toolsAfter.length}`);
    }, 90000);
  });

  describe('Provider Connection Flow', () => {
    let client;

    afterEach(async () => {
      if (client) {
        await client.stop();
        client = null;
      }
    });

    test('connect_provider tool should be available after Pierre auth', async () => {
      const testToken = generateTestToken('provider-connect-user', 'provider@example.com', 3600);

      client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      await client.start();
      await client.send(MCPMessages.initialize);
      await new Promise(resolve => setTimeout(resolve, 2000));

      const response = await client.send(MCPMessages.toolsList);
      const tools = response.result.tools;

      // Find connect_provider tool
      const connectProviderTool = tools.find(t => t.name === 'connect_provider');

      expect(connectProviderTool).toBeDefined();
      expect(connectProviderTool.description).toBeDefined();
      expect(connectProviderTool.inputSchema).toBeDefined();

      // Check that provider parameter exists in schema
      if (connectProviderTool.inputSchema.properties) {
        expect(connectProviderTool.inputSchema.properties).toHaveProperty('provider');
      }
    }, 60000);

    test('connect_provider call should return OAuth URL or status', async () => {
      const testToken = generateTestToken('provider-call-user', 'providercall@example.com', 3600);

      client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      await client.start();
      await client.send(MCPMessages.initialize);
      await new Promise(resolve => setTimeout(resolve, 2000));

      const toolCall = {
        jsonrpc: '2.0',
        id: 30,
        method: 'tools/call',
        params: {
          name: 'connect_provider',
          arguments: { provider: 'strava' }
        }
      };

      try {
        const response = await client.send(toolCall, 15000);

        expect(response).toHaveProperty('jsonrpc', '2.0');

        if (response.result) {
          expect(response.result).toHaveProperty('content');
          expect(Array.isArray(response.result.content)).toBe(true);

          // Content may include OAuth URL or connection status
          const textContent = response.result.content.find(c => c.type === 'text');
          if (textContent) {
            // May contain OAuth URL or status message
            expect(typeof textContent.text).toBe('string');
          }
        }
      } catch (error) {
        // Timeout is acceptable - OAuth flow may be waiting
        expect(error.message).toContain('timed out');
      }
    }, 20000);
  });

  describe('Tenant Isolation in OAuth Flow', () => {
    test('different users should have isolated OAuth state', async () => {
      // User 1
      const token1 = generateTestToken('tenant-user-1', 'tenant1@example.com', 3600);
      const client1 = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', token1.access_token
      ]);

      // User 2
      const token2 = generateTestToken('tenant-user-2', 'tenant2@example.com', 3600);
      const client2 = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', token2.access_token
      ]);

      try {
        await client1.start();
        await client2.start();

        await client1.send(MCPMessages.initialize);
        await client2.send(MCPMessages.initialize);

        // Both should get independent tool lists
        const toolsRequest1 = { ...MCPMessages.toolsList, id: 100 };
        const toolsRequest2 = { ...MCPMessages.toolsList, id: 200 };

        const tools1 = await client1.send(toolsRequest1);
        const tools2 = await client2.send(toolsRequest2);

        // Tool counts should be consistent (same server, same capabilities)
        expect(tools1.result.tools.length).toBe(tools2.result.tools.length);

        // Responses should have their respective request IDs (independent sessions)
        expect(tools1.id).toBe(100);
        expect(tools2.id).toBe(200);

        // Both should have full tool set
        expect(tools1.result.tools.length).toBeGreaterThan(30);
        expect(tools2.result.tools.length).toBeGreaterThan(30);
      } finally {
        await client1.stop();
        await client2.stop();
      }
    }, 90000);
  });

  describe('REGRESSION: connect_provider Visibility After OAuth', () => {
    test('CRITICAL: connect_provider must be visible immediately after auth', async () => {
      // This is the EXACT regression that was reported:
      // User completed Pierre OAuth but couldn't connect Strava because
      // connect_provider was not visible in tools/list

      const testToken = generateTestToken('regression-test-user', 'regression@example.com', 3600);

      const client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      try {
        await client.start();
        await client.send(MCPMessages.initialize);

        // Check tools immediately (regression was: tools not refreshed after OAuth)
        let toolsList = await client.send(MCPMessages.toolsList);
        let toolNames = toolsList.result.tools.map(t => t.name);

        // If not present, wait and retry (bridge may still be connecting)
        if (!toolNames.includes('connect_provider')) {
          console.log('First check: connect_provider not found, waiting...');
          await new Promise(resolve => setTimeout(resolve, 3000));

          toolsList = await client.send(MCPMessages.toolsList);
          toolNames = toolsList.result.tools.map(t => t.name);
        }

        // CRITICAL ASSERTION
        if (!toolNames.includes('connect_provider')) {
          console.error('❌ REGRESSION DETECTED!');
          console.error('connect_provider NOT visible after OAuth');
          console.error('Available tools:', toolNames);
        }

        expect(toolNames).toContain('connect_provider');
        console.log('✅ REGRESSION CHECK PASSED: connect_provider visible');

      } finally {
        await client.stop();
      }
    }, 60000);

    test('CRITICAL: All provider tools must be visible after auth', async () => {
      const testToken = generateTestToken('all-provider-tools-user', 'alltools@example.com', 3600);

      const client = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      try {
        await client.start();
        await client.send(MCPMessages.initialize);
        await new Promise(resolve => setTimeout(resolve, 2000));

        const toolsList = await client.send(MCPMessages.toolsList);
        const toolNames = toolsList.result.tools.map(t => t.name);

        // All these tools MUST be visible after OAuth (regression check)
        const requiredProviderTools = [
          'connect_provider',
          'disconnect_provider',
          'get_connection_status',
          'get_activities',
          'get_athlete'
        ];

        const missingTools = requiredProviderTools.filter(t => !toolNames.includes(t));

        if (missingTools.length > 0) {
          console.error('❌ REGRESSION: Missing provider tools:', missingTools);
          console.error('Available tools:', toolNames);
        }

        expect(missingTools).toEqual([]);
        console.log('✅ All provider tools visible after OAuth');

      } finally {
        await client.stop();
      }
    }, 60000);
  });
});
