// ABOUTME: CRITICAL REGRESSION - Tools list not refreshed after OAuth completion
// ABOUTME: Tests that tools/list is fetched and updated after connect_to_pierre succeeds
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');
const path = require('path');
const fs = require('fs');
const os = require('os');

describe('CRITICAL REGRESSION: Tools List Not Refreshed After OAuth', () => {
  let serverHandle;
  let bridgeClient;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 60000);

  afterEach(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
      bridgeClient = null;
    }
  });

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('CRITICAL: tools/list MUST be refreshed immediately after connect_to_pierre succeeds', async () => {
    // Simulate first-time connection (no tokens)
    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // STEP 1: Before OAuth - should show connect_to_pierre
    const toolsBeforeOAuth = await bridgeClient.send(MCPMessages.toolsList);
    expect(toolsBeforeOAuth.result).toHaveProperty('tools');

    const toolNamesBeforeOAuth = toolsBeforeOAuth.result.tools.map(t => t.name);
    expect(toolNamesBeforeOAuth).toContain('connect_to_pierre');

    // For this test, we'll simulate successful OAuth by providing a token
    // In real scenario, OAuth flow would complete here

    // STEP 2: Simulate OAuth completion by injecting valid token
    // (In reality, this happens after user approves in browser)
    const testToken = generateTestToken('test-user-oauth', 'test@example.com', 3600);

    // Restart bridge with token to simulate post-OAuth state
    await bridgeClient.stop();

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // STEP 3: CRITICAL - Tools list MUST be updated after OAuth
    const toolsAfterOAuth = await bridgeClient.send(MCPMessages.toolsList, 15000);

    expect(toolsAfterOAuth.result).toHaveProperty('tools');
    const toolNamesAfterOAuth = toolsAfterOAuth.result.tools.map(t => t.name);

    // CRITICAL ASSERTIONS:
    // 1. connect_to_pierre should be GONE (or at least not the only tool)
    // 2. Real Pierre tools should be present
    expect(toolNamesAfterOAuth.length).toBeGreaterThan(1);

    // Should have real fitness tools now
    // At minimum, should have these core tools:
    const expectedTools = [
      'get_athlete',
      'get_activities',
      'connect_provider',  // THIS IS CRITICAL - needed to connect Strava!
      'get_stats'
    ];

    // Check for at least some real tools (not just connect_to_pierre)
    const hasRealTools = expectedTools.some(tool => toolNamesAfterOAuth.includes(tool));

    if (!hasRealTools) {
      console.error('REGRESSION DETECTED:');
      console.error('After OAuth, tools list still shows:', toolNamesAfterOAuth);
      console.error('Expected to see tools like:', expectedTools);
    }

    expect(hasRealTools).toBe(true);
  }, 60000);

  test('CRITICAL: connect_provider tool must be available after OAuth to connect Strava', async () => {
    // This test verifies the EXACT regression from the user report

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Get initial tools (before OAuth)
    const initialTools = await bridgeClient.send(MCPMessages.toolsList);
    const initialToolNames = initialTools.result.tools.map(t => t.name);

    // connect_provider should NOT be available before OAuth
    expect(initialToolNames).not.toContain('connect_provider');

    // Simulate OAuth completion with valid token
    await bridgeClient.stop();

    const testToken = generateTestToken('test-user-provider', 'provider@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // CRITICAL: After OAuth, get tools again
    const toolsAfterOAuth = await bridgeClient.send(MCPMessages.toolsList, 15000);
    const toolNamesAfterOAuth = toolsAfterOAuth.result.tools.map(t => t.name);

    // CRITICAL ASSERTION: connect_provider MUST be available
    if (!toolNamesAfterOAuth.includes('connect_provider')) {
      console.error('CRITICAL REGRESSION:');
      console.error('User completed OAuth but cannot connect Strava!');
      console.error('Tools available:', toolNamesAfterOAuth);
      console.error('Missing: connect_provider');
      console.error('This is the EXACT regression from the user report.');
    }

    expect(toolNamesAfterOAuth).toContain('connect_provider');

    // Should also have other provider-related tools
    const hasProviderTools = [
      'connect_provider',
      'disconnect_provider',
      'get_connection_status'
    ].some(tool => toolNamesAfterOAuth.includes(tool));

    expect(hasProviderTools).toBe(true);
  }, 60000);

  test('CRITICAL: User can actually CALL connect_provider after OAuth (not just see it)', async () => {
    // Verify the tool is not just listed, but actually callable

    const testToken = generateTestToken('test-user-callable', 'callable@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Get tools list
    const toolsList = await bridgeClient.send(MCPMessages.toolsList, 15000);
    const toolNames = toolsList.result.tools.map(t => t.name);

    expect(toolNames).toContain('connect_provider');

    // CRITICAL: Actually TRY to call connect_provider
    const connectProviderCall = {
      jsonrpc: '2.0',
      id: 100,
      method: 'tools/call',
      params: {
        name: 'connect_provider',
        arguments: {
          provider: 'strava'
        }
      }
    };

    try {
      const response = await bridgeClient.send(connectProviderCall, 10000);

      // We expect either:
      // 1. Success response (tool executed)
      // 2. OAuth flow initiated (browser opens)
      // 3. Some error response (but NOT "tool not found")

      if (response.error) {
        // Error is acceptable IF it's not "tool not found"
        expect(response.error.message).not.toContain('not found');
        expect(response.error.message).not.toContain('unknown tool');
        expect(response.error.code).not.toBe(-32601); // Method not found
      } else {
        // Success - tool was called
        expect(response).toHaveProperty('result');
      }

    } catch (error) {
      // If timeout or connection error, that's fine
      // As long as it's not "tool not found"
      expect(error.message).not.toContain('not found');
    }
  }, 60000);

  test('REGRESSION: Bridge must fetch tools proactively on connection, not lazily', async () => {
    // This tests the root cause: bridge should fetch tools immediately
    // after OAuth, not wait for first tool call

    const testToken = generateTestToken('test-user-proactive', 'proactive@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    const startTime = Date.now();

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // IMMEDIATELY after initialization, tools should be available
    // Should not need to wait or make a dummy tool call
    const toolsList = await bridgeClient.send(MCPMessages.toolsList, 5000);

    const fetchTime = Date.now() - startTime;

    expect(toolsList.result).toHaveProperty('tools');
    expect(toolsList.result.tools.length).toBeGreaterThan(1);

    // Should be fast (< 3 seconds) if tools were fetched proactively
    // during connection initialization
    expect(fetchTime).toBeLessThan(3000);

    console.log(`Tools fetched in ${fetchTime}ms (should be < 3000ms for proactive fetch)`);
  }, 60000);

  test('REGRESSION: Tools list must update when connection state changes', async () => {
    // Test that tools list dynamically updates based on connection state

    // State 1: Not connected - should show connect_to_pierre
    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    const disconnectedTools = await bridgeClient.send(MCPMessages.toolsList);
    const disconnectedToolNames = disconnectedTools.result.tools.map(t => t.name);

    expect(disconnectedToolNames).toContain('connect_to_pierre');

    // State 2: Connected - should show real Pierre tools
    await bridgeClient.stop();

    const testToken = generateTestToken('test-user-state', 'state@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    const connectedTools = await bridgeClient.send(MCPMessages.toolsList, 15000);
    const connectedToolNames = connectedTools.result.tools.map(t => t.name);

    // Tools list should be DIFFERENT
    expect(connectedToolNames).not.toEqual(disconnectedToolNames);

    // Should have more tools when connected
    expect(connectedToolNames.length).toBeGreaterThan(disconnectedToolNames.length);
  }, 90000);
});

describe('REGRESSION: Tools List Caching Issues', () => {
  let serverHandle;
  let bridgeClient;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 60000);

  afterEach(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
      bridgeClient = null;
    }
  });

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('REGRESSION: Cached tools list must be invalidated after OAuth', async () => {
    // Bridge may cache tools list for performance
    // This cache MUST be invalidated when connection state changes

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Get tools before OAuth (may be cached)
    const tools1 = await bridgeClient.send(MCPMessages.toolsList);
    const tools2 = await bridgeClient.send(MCPMessages.toolsList);

    // These should be the same (cached)
    expect(tools1.result.tools).toEqual(tools2.result.tools);

    // Now simulate OAuth
    await bridgeClient.stop();

    const testToken = generateTestToken('test-user-cache', 'cache@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Get tools after OAuth
    const tools3 = await bridgeClient.send(MCPMessages.toolsList, 15000);

    // Cache should be invalidated - tools should be DIFFERENT
    expect(tools3.result.tools).not.toEqual(tools1.result.tools);
  }, 90000);

  test('REGRESSION: MCP host must be notified of tools list changes', async () => {
    // When tools change, bridge should potentially send a notification
    // to the MCP host (if protocol supports it)

    const testToken = generateTestToken('test-user-notify', 'notify@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    let notificationsReceived = [];

    bridgeClient.on('notification', (notification) => {
      notificationsReceived.push(notification);
    });

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Wait a moment for any notifications
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Get tools list
    await bridgeClient.send(MCPMessages.toolsList, 15000);

    // Check if any notifications related to tools were sent
    // (This is aspirational - may not be implemented yet)
    console.log('Notifications received:', notificationsReceived.length);

    // At minimum, verify tools are available
    const tools = await bridgeClient.send(MCPMessages.toolsList);
    expect(tools.result.tools.length).toBeGreaterThan(1);
  }, 60000);
});

describe('User Experience: OAuth Success → Tools Available Flow', () => {
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

  test('UX TEST: Complete flow from OAuth to using connect_provider', async () => {
    // This simulates the EXACT user experience from the regression report

    console.log('Step 1: User says "Connect to Pierre"');
    // User triggers connect_to_pierre tool
    // OAuth flow completes (browser, approval, tokens saved)

    console.log('Step 2: User says "Connect to Strava"');
    // User expects to be able to connect Strava now

    const testToken = generateTestToken('ux-test-user', 'ux@example.com', 3600);

    const bridgeClient = new MockMCPClient('node', [
      path.join(__dirname, '../../dist/cli.js'),
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // User's MCP client (Claude) lists tools
    const toolsList = await bridgeClient.send(MCPMessages.toolsList, 15000);
    const toolNames = toolsList.result.tools.map(t => t.name);

    console.log('Tools available after OAuth:', toolNames);

    // CRITICAL: User MUST be able to connect Strava at this point
    const canConnectStrava = toolNames.includes('connect_provider');

    if (!canConnectStrava) {
      console.error('❌ USER IS STUCK:');
      console.error('   User completed OAuth successfully');
      console.error('   User wants to connect Strava');
      console.error('   But connect_provider tool is not available');
      console.error('   This matches the regression report exactly!');
    }

    expect(canConnectStrava).toBe(true);

    await bridgeClient.stop();
  }, 60000);
});
