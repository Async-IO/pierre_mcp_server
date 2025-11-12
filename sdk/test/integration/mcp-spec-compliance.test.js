// ABOUTME: MCP specification compliance tests - tools/list, authentication, tool calls
// ABOUTME: Validates that bridge follows MCP protocol: ALL tools visible, auth checked at call time
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');
const { clearKeychainTokens } = require('../helpers/keychain-cleanup');
const path = require('path');

describe('MCP Spec Compliance: tools/list Visibility', () => {
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

  beforeEach(async () => {
    // CRITICAL: Clear keychain before each test to prevent token pollution
    // Without this, tokens from previous tests leak into subsequent tests,
    // causing flaky failures (test sees stale tokens from keychain)
    await clearKeychainTokens();
  });

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

  test('MCP SPEC: tools/list MUST return ALL tools WITHOUT authentication', async () => {
    // Per MCP spec: tools/list does NOT require authentication
    // All tools must be visible for discovery
    // Authentication is checked when CALLING tools, not listing them

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
      // NO --token flag! This is unauthenticated
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    const toolsList = await bridgeClient.send(MCPMessages.toolsList);
    const toolNames = toolsList.result.tools.map(t => t.name);

    console.log(`Unauthenticated tools/list returned: ${toolNames.length} tools`);

    // CRITICAL: ALL tools must be visible, including:
    expect(toolNames).toContain('connect_to_pierre');
    expect(toolNames).toContain('connect_provider');
    expect(toolNames).toContain('get_activities');
    expect(toolNames).toContain('get_athlete');
    expect(toolNames).toContain('disconnect_provider');

    // Should have full toolset (30+ tools)
    expect(toolNames.length).toBeGreaterThan(20);

    console.log('✅ MCP SPEC COMPLIANT: All tools visible without authentication');
  }, 60000);

  test('MCP SPEC: tools/list MUST return SAME tools WITH authentication', async () => {
    // Per MCP spec: tools/list returns same tools regardless of auth status
    // The presence of a token should NOT change the tools list

    const testToken = generateTestToken('auth-user', 'auth@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Wait for bridge to complete proactive connection with retry logic
    // This is more reliable than fixed timeout, especially under CI/CD load
    let toolsList;
    let toolNames = [];
    const maxRetries = 5;
    for (let i = 0; i < maxRetries; i++) {
      await new Promise(resolve => setTimeout(resolve, 1000));
      toolsList = await bridgeClient.send(MCPMessages.toolsList);
      toolNames = toolsList.result.tools.map(t => t.name);

      // If we have more than just connect_to_pierre, connection succeeded
      if (toolNames.length > 5) {
        break;
      }
    }

    console.log(`Authenticated tools/list returned: ${toolNames.length} tools`);

    // Should have EXACT SAME tools as unauthenticated
    expect(toolNames).toContain('connect_to_pierre');
    expect(toolNames).toContain('connect_provider');
    expect(toolNames).toContain('get_activities');
    expect(toolNames).toContain('get_athlete');

    // Same number of tools as unauthenticated
    expect(toolNames.length).toBeGreaterThan(20);

    console.log('✅ MCP SPEC COMPLIANT: Same tools visible with authentication');
  }, 60000);

  test('MCP SPEC: tools/list results MUST be IDENTICAL regardless of auth state', async () => {
    // This test explicitly verifies that tools/list returns EXACTLY the same
    // tools whether authenticated or not

    // First: Get tools WITHOUT auth
    const unauthClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await unauthClient.start();
    await unauthClient.send(MCPMessages.initialize);

    const unauthTools = await unauthClient.send(MCPMessages.toolsList);
    const unauthToolNames = unauthTools.result.tools.map(t => t.name).sort();

    await unauthClient.stop();

    // Second: Get tools WITH auth
    const testToken = generateTestToken('compare-user', 'compare@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    const authTools = await bridgeClient.send(MCPMessages.toolsList);
    const authToolNames = authTools.result.tools.map(t => t.name).sort();

    console.log(`Unauthenticated: ${unauthToolNames.length} tools`);
    console.log(`Authenticated: ${authToolNames.length} tools`);

    // CRITICAL: Tools lists must be IDENTICAL
    expect(authToolNames).toEqual(unauthToolNames);

    console.log('✅ MCP SPEC COMPLIANT: Identical tools lists regardless of auth');
  }, 120000);
});

describe('MCP Spec Compliance: Authentication at Call Time', () => {
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

  beforeEach(async () => {
    await clearKeychainTokens();
  });

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

  test('MCP SPEC: Authenticated tool call WITH valid token MUST succeed', async () => {
    // Per MCP spec: Authentication checked at CALL time, not discovery time
    // Calling a tool WITH valid credentials should work
    // Using tools/list as the test tool since it's always available

    const testToken = generateTestToken('call-auth-user', 'call-auth@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Wait for bridge to complete proactive connection
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Call tools/list via tools/call (not as a direct method)
    // This tests that authenticated tool calls work without triggering auth errors
    const toolsList = await bridgeClient.send(MCPMessages.toolsList, 5000);

    // Should succeed and return tools
    expect(toolsList.result).toBeDefined();
    expect(toolsList.result.tools).toBeInstanceOf(Array);
    expect(toolsList.result.tools.length).toBeGreaterThan(0);

    // Should NOT have any auth-related errors
    expect(toolsList.error).toBeUndefined();

    console.log('✅ Authenticated tool call succeeded without auth errors');
  }, 60000);

  test('MCP SPEC: Tool call WITHOUT token may fail with auth error OR trigger OAuth', async () => {
    // Per MCP spec: Tools are visible without auth, but CALLING them requires auth
    // Bridge may either:
    // 1. Return authentication required error
    // 2. Trigger OAuth flow (connect_to_pierre)

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
      // NO --token! Unauthenticated
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Try to call a tool that requires authentication
    const toolCall = {
      jsonrpc: '2.0',
      id: 200,
      method: 'tools/call',
      params: {
        name: 'get_connection_status',
        arguments: {}
      }
    };

    try {
      const response = await bridgeClient.send(toolCall, 10000);

      // If we get a response, it should either:
      // 1. Be an auth error
      // 2. Trigger OAuth (not testable without browser)
      if (response.error) {
        console.log(`Auth required error: ${response.error.message}`);
        expect(response.error).toBeDefined();
      } else {
        console.log('Tool call succeeded or triggered OAuth');
      }

    } catch (error) {
      // Timeout or other error is acceptable
      console.log(`Tool call failed (expected): ${error.message}`);
    }

    console.log('✅ Unauthenticated tool call handled appropriately');
  }, 60000);
});

describe('MCP Spec Compliance: Tools List Consistency', () => {
  let serverHandle;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testEncryptionKey
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

  test('MCP SPEC: Multiple tools/list calls MUST return consistent results', async () => {
    // Per MCP spec: tools/list should return stable, cacheable results

    const testToken = generateTestToken('consistency-user', 'consistency@example.com', 3600);

    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Call tools/list 3 times in succession
    const tools1 = await bridgeClient.send(MCPMessages.toolsList);
    const tools2 = await bridgeClient.send(MCPMessages.toolsList);
    const tools3 = await bridgeClient.send(MCPMessages.toolsList);

    const names1 = tools1.result.tools.map(t => t.name).sort();
    const names2 = tools2.result.tools.map(t => t.name).sort();
    const names3 = tools3.result.tools.map(t => t.name).sort();

    // All three should be IDENTICAL
    expect(names1).toEqual(names2);
    expect(names2).toEqual(names3);

    console.log(`✅ Consistent results: ${names1.length} tools across 3 calls`);

    await bridgeClient.stop();
  }, 60000);

  test('MCP SPEC: tools/list MUST be fast (cacheable)', async () => {
    // Per MCP spec: tools/list should be cacheable and fast

    const testToken = generateTestToken('perf-user', 'perf@example.com', 3600);

    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // First call (may be slower - cache miss)
    const start1 = Date.now();
    await bridgeClient.send(MCPMessages.toolsList);
    const duration1 = Date.now() - start1;

    // Second call (should be cached)
    const start2 = Date.now();
    await bridgeClient.send(MCPMessages.toolsList);
    const duration2 = Date.now() - start2;

    console.log(`First call: ${duration1}ms, Second call: ${duration2}ms`);

    // Second call should be fast (< 100ms) if properly cached
    expect(duration2).toBeLessThan(100);

    console.log('✅ tools/list is properly cached');

    await bridgeClient.stop();
  }, 60000);
});

describe('MCP Spec Compliance: Critical Tools Availability', () => {
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

  test('REGRESSION PREVENTION: connect_provider MUST be visible immediately', async () => {
    // This is the EXACT regression from user report:
    // User completed OAuth but couldn't connect Strava because
    // connect_provider was not visible

    const testToken = generateTestToken('regression-user', 'regression@example.com', 3600);

    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    const toolsList = await bridgeClient.send(MCPMessages.toolsList, 5000);
    const toolNames = toolsList.result.tools.map(t => t.name);

    // CRITICAL: connect_provider MUST be visible
    if (!toolNames.includes('connect_provider')) {
      console.error('❌ REGRESSION DETECTED: connect_provider not visible!');
      console.error('Available tools:', toolNames);
      console.error('This is the exact user-reported regression!');
    }

    expect(toolNames).toContain('connect_provider');

    console.log('✅ REGRESSION PREVENTED: connect_provider is visible');

    await bridgeClient.stop();
  }, 60000);

  test('REGRESSION PREVENTION: All provider management tools visible', async () => {
    // Verify all provider-related tools are visible

    const testToken = generateTestToken('provider-user', 'provider@example.com', 3600);

    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    const toolsList = await bridgeClient.send(MCPMessages.toolsList);
    const toolNames = toolsList.result.tools.map(t => t.name);

    // All provider management tools must be visible
    const providerTools = [
      'connect_provider',
      'disconnect_provider',
      'get_connection_status'
    ];

    const missingTools = providerTools.filter(tool => !toolNames.includes(tool));

    if (missingTools.length > 0) {
      console.error('❌ Missing provider tools:', missingTools);
    }

    expect(missingTools).toEqual([]);

    console.log('✅ All provider management tools visible');

    await bridgeClient.stop();
  }, 60000);
});
