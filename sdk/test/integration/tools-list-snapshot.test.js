// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Snapshot tests for tools/list to catch silent tool disappearance
// ABOUTME: Validates tool list consistency across HTTP and stdio transports
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StreamableHTTPClientTransport } = require('@modelcontextprotocol/sdk/client/streamableHttp.js');
const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const { clearKeychainTokens } = require('../helpers/keychain-cleanup');
const path = require('path');

describe('Tools List Snapshot Tests', () => {
  let serverHandle;
  let testToken;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;
  const mcpUrl = `${serverUrl}/mcp`;

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

  describe('Tool Names Snapshot', () => {
    test('tools/list should return expected tool names (snapshot)', async () => {
      const client = new Client(
        { name: 'snapshot-test-client', version: '1.0.0' },
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
        const toolNames = toolsResult.tools.map(t => t.name).sort();

        // Snapshot test - will fail if tools change
        expect(toolNames).toMatchSnapshot('expected-tools-list');

      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);
  });

  describe('Critical Tools Presence', () => {
    // These tools MUST always be present - removing them would be a breaking change
    // NOTE: Tool names updated for pluggable architecture (commit 787cbc31)
    const CRITICAL_TOOLS = [
      // Core connection tools
      // Note: connect_to_pierre removed - SDK bridge handles authentication locally via RFC 8414 discovery
      'connect_provider',
      'disconnect_provider',
      'get_connection_status',

      // Activity/Data tools
      'get_activities',
      'get_athlete',
      'get_stats',

      // Analytics tools (renamed in pluggable architecture)
      'analyze_training_load',      // was: analyze_activity
      'calculate_fitness_score',    // was: calculate_metrics
      'detect_patterns',            // was: analyze_performance_trends

      // Sleep/Recovery tools
      'calculate_recovery_score',
      'suggest_rest_day',

      // Goal tools
      'set_goal',
      'track_progress',
      'suggest_goals',

      // Configuration tools
      'get_user_configuration',
      'update_user_configuration'
    ];

    test('all critical tools must be present', async () => {
      const client = new Client(
        { name: 'critical-tools-test', version: '1.0.0' },
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
        const toolNames = toolsResult.tools.map(t => t.name);

        const missingTools = CRITICAL_TOOLS.filter(t => !toolNames.includes(t));

        if (missingTools.length > 0) {
          console.error('❌ CRITICAL TOOLS MISSING:');
          missingTools.forEach(t => console.error(`   - ${t}`));
        }

        expect(missingTools).toEqual([]);

      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);
  });

  describe('Transport Parity', () => {
    test('HTTP and stdio transports should return identical tool lists', async () => {
      // Get tools via HTTP transport
      const httpClient = new Client(
        { name: 'http-parity-test', version: '1.0.0' },
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

      let httpToolNames;
      try {
        await httpClient.connect(transport);
        const httpTools = await httpClient.listTools();
        httpToolNames = httpTools.tools.map(t => t.name).sort();
      } finally {
        try {
          await httpClient.close();
        } catch (e) {
          // Ignore close errors
        }
      }

      // Get tools via stdio transport
      const stdioClient = new MockMCPClient('node', [
        bridgePath,
        '--server', serverUrl,
        '--token', testToken.access_token
      ]);

      let stdioToolNames;
      try {
        await stdioClient.start();
        await stdioClient.send(MCPMessages.initialize);
        await new Promise(resolve => setTimeout(resolve, 2000));

        const stdioTools = await stdioClient.send(MCPMessages.toolsList);
        stdioToolNames = stdioTools.result.tools.map(t => t.name).sort();
      } finally {
        await stdioClient.stop();
      }

      // Compare
      const onlyInHttp = httpToolNames.filter(t => !stdioToolNames.includes(t));
      const onlyInStdio = stdioToolNames.filter(t => !httpToolNames.includes(t));

      if (onlyInHttp.length > 0 || onlyInStdio.length > 0) {
        console.error('❌ TRANSPORT PARITY VIOLATION:');
        if (onlyInHttp.length > 0) {
          console.error('   Only in HTTP:', onlyInHttp);
        }
        if (onlyInStdio.length > 0) {
          console.error('   Only in stdio:', onlyInStdio);
        }
      }

      expect(httpToolNames).toEqual(stdioToolNames);

      console.log(`✅ Transport parity: ${httpToolNames.length} tools match`);
    }, 90000);
  });

  describe('Tool Count Thresholds', () => {
    test('should have minimum expected number of tools', async () => {
      const client = new Client(
        { name: 'tool-count-test', version: '1.0.0' },
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

        // Based on schema_completeness_test.rs, we expect 40+ tools
        const MIN_EXPECTED_TOOLS = 35;

        expect(toolsResult.tools.length).toBeGreaterThanOrEqual(MIN_EXPECTED_TOOLS);

        console.log(`✅ Tool count: ${toolsResult.tools.length} (minimum: ${MIN_EXPECTED_TOOLS})`);

      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);

    test('should not exceed maximum reasonable number of tools', async () => {
      const client = new Client(
        { name: 'tool-max-test', version: '1.0.0' },
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

        // Guard against accidental duplication or runaway tool generation
        const MAX_REASONABLE_TOOLS = 100;

        expect(toolsResult.tools.length).toBeLessThanOrEqual(MAX_REASONABLE_TOOLS);

      } finally {
        try {
          await client.close();
        } catch (e) {
          // Ignore close errors
        }
      }
    }, 30000);
  });

  describe('Tool Schema Consistency', () => {
    test('all tools should have valid schema structure', async () => {
      const client = new Client(
        { name: 'schema-validation-test', version: '1.0.0' },
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

        const invalidTools = [];

        for (const tool of toolsResult.tools) {
          const issues = [];

          if (!tool.name || typeof tool.name !== 'string') {
            issues.push('missing or invalid name');
          }

          if (!tool.description || typeof tool.description !== 'string') {
            issues.push('missing or invalid description');
          }

          if (!tool.inputSchema) {
            issues.push('missing inputSchema');
          } else {
            if (tool.inputSchema.type !== 'object') {
              issues.push(`inputSchema.type should be 'object', got '${tool.inputSchema.type}'`);
            }

            // If required fields exist, they must be in properties
            if (tool.inputSchema.required && tool.inputSchema.required.length > 0) {
              if (!tool.inputSchema.properties) {
                issues.push('has required fields but no properties');
              } else {
                for (const req of tool.inputSchema.required) {
                  if (!tool.inputSchema.properties[req]) {
                    issues.push(`required field '${req}' not in properties`);
                  }
                }
              }
            }
          }

          if (issues.length > 0) {
            invalidTools.push({ name: tool.name, issues });
          }
        }

        if (invalidTools.length > 0) {
          console.error('❌ INVALID TOOL SCHEMAS:');
          invalidTools.forEach(t => {
            console.error(`   ${t.name}: ${t.issues.join(', ')}`);
          });
        }

        expect(invalidTools).toEqual([]);

        console.log(`✅ All ${toolsResult.tools.length} tools have valid schemas`);

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
