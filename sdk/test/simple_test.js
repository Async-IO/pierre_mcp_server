#!/usr/bin/env node

// ABOUTME: Simple smoke test for basic MCP client functionality
// ABOUTME: Quick validation test for development and debugging workflow
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Simple test script to run the Claude Desktop test
 */

import { spawn } from 'child_process';

console.log('[Claude Desktop Test] 🧪 Starting MCP client test...');

// Create bridge process
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
  stdio: ['pipe', 'pipe', 'inherit']
});

let messageId = 1;
const pendingRequests = new Map();

function sendMcpMessage(method, params = {}) {
  const message = {
    jsonrpc: '2.0',
    id: messageId++,
    method,
    params
  };

  console.log(`[Test] ➡️ Sending: ${method}`);
  bridgeProcess.stdin.write(JSON.stringify(message) + '\n');

  return new Promise((resolve, reject) => {
    pendingRequests.set(message.id, { resolve, reject, method });
    setTimeout(() => {
      if (pendingRequests.has(message.id)) {
        pendingRequests.delete(message.id);
        reject(new Error(`Timeout waiting for ${method}`));
      }
    }, 30000);
  });
}

bridgeProcess.stdout.on('data', (data) => {
  const lines = data.toString().split('\n').filter(line => line.trim());

  for (const line of lines) {
    try {
      const response = JSON.parse(line);
      console.log(`[Test] ⬅️ Response: ${JSON.stringify(response, null, 2)}`);

      if (pendingRequests.has(response.id)) {
        const { resolve } = pendingRequests.get(response.id);
        pendingRequests.delete(response.id);
        resolve(response);
      }
    } catch (error) {
      console.log(`[Test] 📢 Output: ${line}`);
    }
  }
});

bridgeProcess.on('error', (error) => {
  console.error(`[Test] ❌ Error:`, error);
  process.exit(1);
});

bridgeProcess.on('exit', (code) => {
  console.log(`[Test] 🏁 Process exited with code ${code}`);
  process.exit(code || 0);
});

async function runTest() {
  try {
    console.log('[Test] 1️⃣ Initializing...');
    const initResponse = await sendMcpMessage('initialize', {
      protocolVersion: '2025-06-18',
      capabilities: { tools: {} },
      clientInfo: { name: 'test-client', version: '1.0.0' }
    });

    if (initResponse.error) {
      throw new Error(`Init failed: ${JSON.stringify(initResponse.error)}`);
    }

    console.log('[Test] ✅ Initialized successfully');

    console.log('[Test] 2️⃣ Listing tools...');
    const toolsResponse = await sendMcpMessage('tools/list');

    if (toolsResponse.error) {
      throw new Error(`Tools failed: ${JSON.stringify(toolsResponse.error)}`);
    }

    console.log(`[Test] 🔧 Found ${toolsResponse.result?.tools?.length || 0} tools`);

    console.log('[Test] ✅ Test completed successfully!');

  } catch (error) {
    console.error('[Test] ❌ Test failed:', error);
    process.exit(1);
  }

  setTimeout(() => {
    bridgeProcess.kill();
  }, 2000);
}

// Start test after delay
setTimeout(runTest, 2000);