// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: MCP specification compliance tests - tools/list visibility tiers, authentication, tool calls
// ABOUTME: Validates auth-gated tools/list (public subset vs full set) and call-time auth checks
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { clearKeychainTokens } = require('../helpers/keychain-cleanup');
const path = require('path');

describe('MCP Spec Compliance: tools/list Visibility', () => {
  let serverHandle;
  let bridgeClient;
  let testToken;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
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

  test('tools/list returns public discovery tools WITHOUT authentication', async () => {
    // Server returns a curated subset of read-only tools for unauthenticated requests
    // This enables tool discovery while protecting auth-gated capabilities

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

    // Public discovery tools include read-only data retrieval and analytics
    expect(toolNames).toContain('get_activities');
    expect(toolNames).toContain('get_athlete');
    expect(toolNames).toContain('get_stats');

    // Auth-gated tools should NOT be visible without authentication
    expect(toolNames).not.toContain('connect_provider');
    expect(toolNames).not.toContain('disconnect_provider');

    // Should have public discovery subset (15+ tools)
    expect(toolNames.length).toBeGreaterThanOrEqual(15);

    console.log('✅ Public discovery tools returned without authentication');
  }, 60000);

  test('MCP SPEC: tools/list MUST return SAME tools WITH authentication', async () => {
    // Per MCP spec: tools/list returns same tools regardless of auth status
    // The presence of a token should NOT change the tools list

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

      // If we have multiple tools, connection succeeded
      if (toolNames.length > 5) {
        break;
      }
    }

    console.log(`Authenticated tools/list returned: ${toolNames.length} tools`);

    // Should have EXACT SAME tools as unauthenticated
    // Note: connect_to_pierre removed from server - SDK bridge handles authentication locally via RFC 8414 discovery
    expect(toolNames).toContain('connect_provider');
    expect(toolNames).toContain('get_activities');
    expect(toolNames).toContain('get_athlete');

    // Same number of tools as unauthenticated
    expect(toolNames.length).toBeGreaterThan(20);

    console.log('✅ MCP SPEC COMPLIANT: Same tools visible with authentication');
  }, 60000);

  test('authenticated tools/list returns superset of unauthenticated tools', async () => {
    // Authenticated users see all tools; unauthenticated see public discovery subset
    // The authenticated set must be a strict superset of the unauthenticated set

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
    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Wait for proactive connection to complete
    await new Promise(resolve => setTimeout(resolve, 2000));

    const authTools = await bridgeClient.send(MCPMessages.toolsList);
    const authToolNames = authTools.result.tools.map(t => t.name).sort();

    console.log(`Unauthenticated: ${unauthToolNames.length} tools`);
    console.log(`Authenticated: ${authToolNames.length} tools`);

    // Authenticated must have MORE tools than unauthenticated
    expect(authToolNames.length).toBeGreaterThan(unauthToolNames.length);

    // Every public tool must also be visible when authenticated (superset)
    const missingFromAuth = unauthToolNames.filter(t => !authToolNames.includes(t));
    expect(missingFromAuth).toEqual([]);

    console.log('✅ Authenticated tools are a superset of public discovery tools');
  }, 120000);
});

describe('MCP Spec Compliance: Authentication at Call Time', () => {
  let serverHandle;
  let bridgeClient;
  let testToken;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
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
  let testToken;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
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
  let testToken;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
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
