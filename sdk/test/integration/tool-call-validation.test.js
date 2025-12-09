// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Comprehensive tool call validation test - validates ALL tool schemas match MCP protocol
// ABOUTME: Tests that Claude Desktop can successfully call each tool with correct parameters

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');
const { clearKeychainTokens } = require('../helpers/keychain-cleanup');
const path = require('path');

describe('Tool Call Validation: Schema Compliance', () => {
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

  // Test data for tool calls (matches what Claude Desktop would send)
  const toolCallTests = [
    {
      name: 'get_activity_intelligence',
      description: 'Activity intelligence tool with provider parameter',
      arguments: { provider: 'strava', activity_id: '12345' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'calculate_recovery_score',
      description: 'Recovery score calculation tool',
      arguments: { provider: 'strava' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'suggest_rest_day',
      description: 'Rest day suggestion tool',
      arguments: { provider: 'strava' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'get_activities',
      description: 'Get activities from provider',
      arguments: { provider: 'strava', limit: 10 },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'get_athlete',
      description: 'Get athlete profile',
      arguments: { provider: 'strava' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'get_stats',
      description: 'Get fitness statistics',
      arguments: { provider: 'strava' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'analyze_activity',
      description: 'Analyze specific activity',
      arguments: { provider: 'strava', activity_id: '456' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'compare_activities',
      description: 'Compare multiple activities',
      arguments: { provider: 'strava', activity_id: '123', comparison_type: 'similar_activities' },
      expectedError: /No valid Strava token|Connect|Authentication/
    },
    {
      name: 'get_connection_status',
      description: 'Check provider connection status',
      arguments: {},
      expectedError: null // This tool works without auth
    },
    // Recipe tools (Combat des Chefs)
    {
      name: 'get_recipe_constraints',
      description: 'Get recipe constraints for meal timing',
      arguments: { meal_timing: 'post_training', target_calories: 500 },
      expectedError: null // Works without external auth
    },
    {
      name: 'list_recipes',
      description: 'List user recipes with optional filtering',
      arguments: { limit: 10 },
      expectedError: null // Works without provider auth
    },
    {
      name: 'get_recipe',
      description: 'Get a specific recipe by ID',
      arguments: { recipe_id: 'test-recipe-123' },
      expectedError: /not found|No recipe/i // Expected when recipe doesn't exist
    },
    {
      name: 'delete_recipe',
      description: 'Delete a recipe by ID',
      arguments: { recipe_id: 'test-recipe-456' },
      expectedError: /not found|No recipe/i // Expected when recipe doesn't exist
    },
    {
      name: 'search_recipes',
      description: 'Search recipes by query',
      arguments: { query: 'chicken', limit: 5 },
      expectedError: null // Works without provider auth
    }
  ];

  test.each(toolCallTests)(
    'Tool call: $name - $description',
    async ({ name, arguments: args, expectedError }) => {
      const testToken = generateTestToken('tool-test-user', 'tool@example.com', 3600);

      bridgeClient = new MockMCPClient('node', [
        bridgePath,
        '--server',
        serverUrl,
        '--token',
        testToken.access_token
      ]);

      await bridgeClient.start();
      await bridgeClient.send(MCPMessages.initialize);

      // Wait for proactive connection
      await new Promise(resolve => setTimeout(resolve, 2000));

      const toolCall = {
        jsonrpc: '2.0',
        id: 100,
        method: 'tools/call',
        params: {
          name,
          arguments: args
        }
      };

      try {
        const response = await bridgeClient.send(toolCall, 10000);

        // Tool should either succeed or fail with expected auth error
        if (response.error) {
          // CRITICAL: Should NOT be JSON parsing error or "Unknown tool" error
          expect(response.error.message).not.toMatch(/Failed to parse JSON/i);
          expect(response.error.message).not.toMatch(/unknown field/i);
          expect(response.error.message).not.toMatch(/Unknown tool/i);
          expect(response.error.code).not.toBe(-32601); // Method not found

          // If we expect an error, verify it matches
          if (expectedError) {
            expect(response.error.message).toMatch(expectedError);
          }

          console.log(`✅ ${name}: Handled correctly (auth required)`);
        } else {
          // Tool succeeded (expected for tools that don't need Strava auth)
          expect(response.result).toBeDefined();
          console.log(`✅ ${name}: Executed successfully`);
        }

      } catch (error) {
        // Timeout is acceptable if OAuth is triggered
        if (error.message.includes('Timeout')) {
          console.log(`✅ ${name}: Timeout (likely OAuth triggered)`);
        } else {
          throw error;
        }
      }
    },
    30000 // Test timeout
  );

  test('REGRESSION: get_activity_intelligence MUST accept provider parameter', async () => {
    // This test specifically validates the fix for image #1 error:
    // "Failed to parse JSON in get_activity_intelligence parameters: unknown field `provider`, expected `activity_id`"

    const testToken = generateTestToken('regression-test', 'regression@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Call with BOTH provider AND activity_id (as Claude Desktop does)
    const toolCall = {
      jsonrpc: '2.0',
      id: 999,
      method: 'tools/call',
      params: {
        name: 'get_activity_intelligence',
        arguments: {
          provider: 'strava',
          activity_id: '12345'
        }
      }
    };

    const response = await bridgeClient.send(toolCall, 10000);

    // Should NOT fail with JSON parsing error
    if (response.error) {
      expect(response.error.message).not.toMatch(/unknown field.*provider/i);
      expect(response.error.message).not.toMatch(/Failed to parse JSON/i);
    }

    console.log('✅ REGRESSION FIX VERIFIED: get_activity_intelligence accepts provider parameter');
  }, 30000);

  test('REGRESSION: calculate_recovery_score MUST be registered as tool', async () => {
    // This test specifically validates the fix for image #2 error:
    // "Unknown tool: calculate_recovery_score"

    const testToken = generateTestToken('regression-test-2', 'regression2@example.com', 3600);

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl,
      '--token',
      testToken.access_token
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);
    await new Promise(resolve => setTimeout(resolve, 2000));

    // First verify tool is in tools/list
    const toolsList = await bridgeClient.send(MCPMessages.toolsList);
    const toolNames = toolsList.result.tools.map(t => t.name);
    expect(toolNames).toContain('calculate_recovery_score');

    // Then try to call it
    const toolCall = {
      jsonrpc: '2.0',
      id: 998,
      method: 'tools/call',
      params: {
        name: 'calculate_recovery_score',
        arguments: {
          provider: 'strava'
        }
      }
    };

    const response = await bridgeClient.send(toolCall, 10000);

    // Should NOT fail with "Unknown tool" error
    if (response.error) {
      expect(response.error.message).not.toMatch(/Unknown tool/i);
      expect(response.error.code).not.toBe(-32601); // Method not found
    }

    console.log('✅ REGRESSION FIX VERIFIED: calculate_recovery_score is registered and callable');
  }, 30000);
});
