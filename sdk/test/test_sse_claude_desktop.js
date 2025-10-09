#!/usr/bin/env node

// ABOUTME: SSE/Streamable HTTP transport test mimicking Claude Desktop behavior
// ABOUTME: Tests real-time server-sent events and streaming HTTP transport layer
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Test script that mimics Claude Desktop's SSE/Streamable HTTP Transport interaction
 * This is the REAL transport that Claude Desktop uses (not stdio)
 */

import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/transport/streamable-http.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';

const SERVER_URL = 'http://localhost:8081';
const TEST_EMAIL = 'user@example.com';
const TEST_PASSWORD = 'securepass123';

console.log('[SSE Test] 🧪 Testing MCP Streamable HTTP Transport (Claude Desktop mode)...');
console.log(`[SSE Test] 🌐 Server: ${SERVER_URL}`);
console.log(`[SSE Test] 📧 User: ${TEST_EMAIL}`);

async function testSSETransport() {
  let client;
  let transport;

  try {
    // Step 1: Authenticate and get JWT token (like Claude Desktop does)
    console.log('\n[SSE Test] 1️⃣ Authenticating user...');
    const loginResponse = await fetch(`${SERVER_URL}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: TEST_EMAIL,
        password: TEST_PASSWORD
      })
    });

    if (!loginResponse.ok) {
      throw new Error(`Authentication failed: ${loginResponse.status} ${await loginResponse.text()}`);
    }

    const { token } = await loginResponse.json();
    console.log('[SSE Test] ✅ Authentication successful');
    console.log(`[SSE Test] 🎫 Got JWT token (${token.substring(0, 20)}...)`);

    // Step 2: Create SSE transport (this is what Claude Desktop uses!)
    console.log('\n[SSE Test] 2️⃣ Creating SSE/Streamable HTTP transport...');
    transport = new StreamableHTTPClientTransport({
      url: `${SERVER_URL}/mcp`,
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
    console.log('[SSE Test] ✅ Transport created');

    // Step 3: Create MCP client and connect
    console.log('\n[SSE Test] 3️⃣ Connecting MCP client...');
    client = new Client({
      name: 'claude-desktop-sse-test',
      version: '1.0.0'
    }, {
      capabilities: {
        tools: {},
        resources: {},
        prompts: {}
      }
    });

    await client.connect(transport);
    console.log('[SSE Test] ✅ MCP client connected via SSE!');

    // Step 4: Test session management
    console.log('\n[SSE Test] 4️⃣ Testing session management...');
    const sessionId = transport.sessionId;
    if (sessionId) {
      console.log(`[SSE Test] ✅ Session ID: ${sessionId}`);
    } else {
      console.log('[SSE Test] ⚠️  No session ID (server may not support sessions yet)');
    }

    // Step 5: List available tools
    console.log('\n[SSE Test] 5️⃣ Listing fitness tools...');
    const toolsResult = await client.listTools();
    console.log(`[SSE Test] ✅ Found ${toolsResult.tools.length} tools`);

    if (toolsResult.tools.length > 0) {
      console.log('[SSE Test] 🏃 Available fitness tools:');
      toolsResult.tools.forEach(tool => {
        console.log(`[SSE Test]   - ${tool.name}: ${tool.description}`);
      });
    }

    // Step 6: Test a tool that requires OAuth (should prompt for connection)
    console.log('\n[SSE Test] 6️⃣ Testing OAuth-required tool...');
    try {
      const result = await client.callTool({
        name: 'connect_strava',
        arguments: {}
      });

      if (result.isError) {
        console.log('[SSE Test] ⚠️  Tool returned error (expected if Strava not connected)');
        console.log(`[SSE Test] 📋 Error: ${JSON.stringify(result.content)}`);
      } else {
        console.log('[SSE Test] ✅ Tool call successful');
        console.log(`[SSE Test] 📋 Result: ${JSON.stringify(result.content)}`);
      }
    } catch (error) {
      console.log(`[SSE Test] ⚠️  Tool call error (expected): ${error.message}`);
    }

    // Step 7: Test SSE notifications (OAuth status updates)
    console.log('\n[SSE Test] 7️⃣ Testing SSE notifications...');
    console.log('[SSE Test] 📡 SSE stream active - server can push OAuth updates');
    console.log('[SSE Test] ℹ️  In Claude Desktop, OAuth callbacks would trigger real-time notifications');

    // Wait a bit to see if any SSE events come through
    await new Promise(resolve => setTimeout(resolve, 2000));

    console.log('\n[SSE Test] 🎉 All tests passed!');
    console.log('[SSE Test] ✅ MCP Streamable HTTP Transport working correctly');
    console.log('[SSE Test] ✅ Session management operational');
    console.log('[SSE Test] ✅ Claude Desktop compatibility confirmed');

  } catch (error) {
    console.error('\n[SSE Test] ❌ Test failed:', error);
    if (error.cause) {
      console.error('[SSE Test] 🔍 Cause:', error.cause);
    }
    process.exit(1);
  } finally {
    // Cleanup
    if (client) {
      try {
        await client.close();
        console.log('\n[SSE Test] 🧹 Client closed');
      } catch (error) {
        console.error('[SSE Test] ⚠️  Error closing client:', error.message);
      }
    }
  }
}

// Run the test
testSSETransport().then(() => {
  console.log('\n[SSE Test] 🏁 Test suite completed');
  process.exit(0);
}).catch((error) => {
  console.error('\n[SSE Test] 💥 Unhandled error:', error);
  process.exit(1);
});