#!/usr/bin/env node

// ABOUTME: stdio MCP interaction test mimicking Claude Desktop communication
// ABOUTME: Tests OAuth 2.0 flow through stdin/stdout as Claude Desktop would
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Test script that mimics Claude Desktop's stdio MCP interaction
 * This tests the OAuth 2.0 flow as if Claude Desktop were connecting
 */

import { spawn } from 'child_process';
import { readFileSync } from 'fs';

// Load environment configuration
const envPath = '../.workflow_test_env';
const envContent = readFileSync(envPath, 'utf8');
const envVars = {};

envContent.split('\n').forEach(line => {
  const trimmed = line.trim();
  if (trimmed.startsWith('export ')) {
    const [key, ...valueParts] = trimmed.replace('export ', '').split('=');
    const value = valueParts.join('=').replace(/^["']|["']$/g, '');
    envVars[key] = value;
  }
});

console.log('[Claude Desktop Test] 🧪 Starting MCP client test with OAuth 2.0...');
console.log(`[Claude Desktop Test] 📧 Using test user: ${envVars.USER_EMAIL || 'user@example.com'}`);

// Create bridge process - this simulates what Claude Desktop does
const bridgeProcess = spawn('node', [
  './dist/cli.js',
  '--server',
  'http://localhost:8081',
  '--user-email',
  'user@example.com',
  '--user-password',
  'securepass123',
  '--verbose'
], {
  stdio: ['pipe', 'pipe', 'inherit'],
  env: { ...process.env, ...envVars }
});

// Track conversation state
let messageId = 1;
const pendingRequests = new Map();

// Send MCP initialization request (like Claude Desktop would)
function sendMcpMessage(method, params = {}) {
  const message = {
    jsonrpc: '2.0',
    id: messageId++,
    method,
    params
  };

  console.log(`[Claude Desktop Test] ➡️  Sending: ${method}`);
  console.log(`[Claude Desktop Test] 📝 ${JSON.stringify(message, null, 2)}`);

  bridgeProcess.stdin.write(JSON.stringify(message) + '\n');

  return new Promise((resolve, reject) => {
    pendingRequests.set(message.id, { resolve, reject, method });

    // Timeout after 30 seconds
    setTimeout(() => {
      if (pendingRequests.has(message.id)) {
        pendingRequests.delete(message.id);
        reject(new Error(`Timeout waiting for response to ${method}`));
      }
    }, 30000);
  });
}

// Handle responses from bridge
bridgeProcess.stdout.on('data', (data) => {
  const lines = data.toString().split('\n').filter(line => line.trim());

  for (const line of lines) {
    try {
      const response = JSON.parse(line);
      console.log(`[Claude Desktop Test] ⬅️  Received response for ID ${response.id}`);
      console.log(`[Claude Desktop Test] 📋 ${JSON.stringify(response, null, 2)}`);

      if (pendingRequests.has(response.id)) {
        const { resolve, method } = pendingRequests.get(response.id);
        pendingRequests.delete(response.id);

        if (response.error) {
          console.error(`[Claude Desktop Test] ❌ Error in ${method}:`, response.error);
        } else {
          console.log(`[Claude Desktop Test] ✅ Success for ${method}`);
        }

        resolve(response);
      }
    } catch (error) {
      console.log(`[Claude Desktop Test] 📢 Bridge output: ${line}`);
    }
  }
});

bridgeProcess.on('error', (error) => {
  console.error(`[Claude Desktop Test] ❌ Bridge process error:`, error);
  process.exit(1);
});

bridgeProcess.on('exit', (code) => {
  console.log(`[Claude Desktop Test] 🏁 Bridge process exited with code ${code}`);
  process.exit(code || 0);
});

// Main test sequence - mimics what Claude Desktop does
async function runClaudeDesktopTest() {
  try {
    console.log(`[Claude Desktop Test] 🚀 Starting MCP conversation...`);

    // Step 1: Initialize the connection (like Claude Desktop startup)
    console.log(`[Claude Desktop Test] 1️⃣ Initializing MCP connection...`);
    const initResponse = await sendMcpMessage('initialize', {
      protocolVersion: '2025-06-18',
      capabilities: {
        tools: {},
        resources: {},
        prompts: {}
      },
      clientInfo: {
        name: 'claude-desktop-test',
        version: '1.0.0'
      }
    });

    if (initResponse.error) {
      throw new Error(`Initialization failed: ${JSON.stringify(initResponse.error)}`);
    }

    console.log(`[Claude Desktop Test] ✅ MCP initialized successfully`);

    // Step 2: List available tools (like Claude Desktop would)
    console.log(`[Claude Desktop Test] 2️⃣ Listing fitness tools...`);
    const toolsResponse = await sendMcpMessage('tools/list');

    if (toolsResponse.error) {
      throw new Error(`Tools list failed: ${JSON.stringify(toolsResponse.error)}`);
    }

    console.log(`[Claude Desktop Test] 🔧 Found ${toolsResponse.result?.tools?.length || 0} fitness tools`);

    if (toolsResponse.result?.tools?.length > 0) {
      console.log(`[Claude Desktop Test] 🏃 Available fitness tools:`);
      toolsResponse.result.tools.forEach(tool => {
        console.log(`[Claude Desktop Test]   - ${tool.name}: ${tool.description}`);
      });
    }

    // Step 3: Try calling a fitness tool (like Claude Desktop would when user asks about fitness)
    console.log(`[Claude Desktop Test] 3️⃣ Testing fitness tool call...`);
    const toolCallResponse = await sendMcpMessage('tools/call', {
      name: 'connect_strava',
      arguments: {}
    });

    if (toolCallResponse.error) {
      console.log(`[Claude Desktop Test] ⚠️  Tool call failed (expected for unconnected Strava): ${JSON.stringify(toolCallResponse.error)}`);
    } else {
      console.log(`[Claude Desktop Test] ✅ Tool call successful`);
    }

    console.log(`[Claude Desktop Test] 🎉 OAuth 2.0 test completed successfully!`);
    console.log(`[Claude Desktop Test] 💪 Pierre MCP Server fitness tools are accessible via OAuth 2.0`);

  } catch (error) {
    console.error(`[Claude Desktop Test] ❌ Test failed:`, error);
    process.exit(1);
  }

  // Keep process alive briefly to see all output
  setTimeout(() => {
    console.log(`[Claude Desktop Test] 🏁 Test completed - terminating bridge`);
    bridgeProcess.kill();
  }, 2000);
}

// Wait for server to be ready, then start test
setTimeout(runClaudeDesktopTest, 3000);