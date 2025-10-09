#!/usr/bin/env node

// ABOUTME: Direct HTTP tool invocation test using StreamableHTTPClientTransport
// ABOUTME: Tests MCP tools/call endpoint with HTTP transport bypassing bridge
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Test HTTP Tool Call - Direct test of get_connection_status via HTTP transport
 *
 * Uses StreamableHTTPClientTransport directly (no bridge)
 */

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StreamableHTTPClientTransport } = require('@modelcontextprotocol/sdk/client/streamableHttp.js');
const fs = require('fs');
const os = require('os');
const path = require('path');

const PIERRE_SERVER_URL = process.env.PIERRE_SERVER_URL || 'http://localhost:8081/mcp';
const TOKEN_FILE = path.join(os.homedir(), '.pierre-claude-tokens.json');

function log(message) {
  console.log(`[Test] ${message}`);
}

function error(message) {
  console.error(`[Test] ❌ ${message}`);
}

async function testHttpTool() {
  log('=== HTTP Tool Call Test ===');
  log('');

  let client;

  try {
    // Read JWT token
    log('Reading JWT token from:', TOKEN_FILE);
    if (!fs.existsSync(TOKEN_FILE)) {
      throw new Error(`Token file not found: ${TOKEN_FILE}`);
    }

    const tokenData = JSON.parse(fs.readFileSync(TOKEN_FILE, 'utf8'));
    const jwtToken = tokenData.pierre?.access_token;

    if (!jwtToken) {
      throw new Error('No JWT token found in token file');
    }

    log('✅ JWT token loaded');
    log('');

    // Create client with HTTP transport
    log('Creating MCP client with HTTP transport...');
    log(`Server URL: ${PIERRE_SERVER_URL}`);

    client = new Client(
      { name: 'test-http-client', version: '1.0.0' },
      { capabilities: { tools: {} } }
    );

    const transport = new StreamableHTTPClientTransport(
      new URL(PIERRE_SERVER_URL),
      {
        requestInit: {
          headers: {
            'Authorization': `Bearer ${jwtToken}`
          }
        }
      }
    );

    await client.connect(transport);
    log('✅ Client connected to server via HTTP');
    log('');

    // Call get_connection_status
    log('Calling get_connection_status...');
    const result = await client.callTool({
      name: 'get_connection_status',
      arguments: {}
    });

    log('✅ Tool call succeeded!');
    log('');
    log('Response:');
    console.log('  Full result:', JSON.stringify(result, null, 2));
    log('');

    // Verify response format
    if (!result.content || !Array.isArray(result.content)) {
      throw new Error('Response missing content array');
    }

    if (result.content.length === 0) {
      throw new Error('Response content array is empty');
    }

    if (result.content[0].type !== 'text') {
      throw new Error(`Expected content[0].type='text', got: ${result.content[0].type}`);
    }

    log('✅ Response format is MCP-compliant');
    log('');
    log('🎉 TEST PASSED!');

  } catch (err) {
    error(`Test failed: ${err.message}`);
    console.error(err.stack);
    process.exit(1);
  } finally {
    if (client) {
      try {
        await client.close();
      } catch (e) {
        // Ignore close errors
      }
    }
  }
}

// Run test
testHttpTool().catch(err => {
  error(`Fatal error: ${err.message}`);
  process.exit(1);
});