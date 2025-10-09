#!/usr/bin/env node

// ABOUTME: Integration test for OAuth token refresh and tools list updates
// ABOUTME: Verifies tools availability changes from connect-only to full list after authentication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Test OAuth authentication and tools refresh flow
 *
 * This test verifies:
 * 1. Initial connection shows only connect_to_pierre tool
 * 2. After OAuth authentication, all fitness tools are available
 * 3. Tools list properly refreshes after authentication
 */

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StdioClientTransport } = require('@modelcontextprotocol/sdk/client/stdio.js');
const { spawn } = require('child_process');

const PIERRE_SERVER_URL = process.env.PIERRE_SERVER_URL || 'http://localhost:8081';
const VERBOSE = process.env.VERBOSE === 'true';

function log(message) {
  console.log(`[Test] ${message}`);
}

function debug(message) {
  if (VERBOSE) {
    console.log(`[Debug] ${message}`);
  }
}

async function testOAuthToolsRefresh() {
  log('Starting OAuth tools refresh test');

  let bridgeProcess;
  let client;

  try {
    // Step 1: Start bridge process
    log('Step 1: Starting bridge process...');
    bridgeProcess = spawn('node', [
      './dist/cli.js',
      '--server', PIERRE_SERVER_URL,
      '--verbose'
    ], {
      stdio: ['pipe', 'pipe', 'pipe']
    });

    // Capture bridge logs
    if (VERBOSE) {
      bridgeProcess.stdout.on('data', (data) => {
        process.stderr.write(`[Bridge stdout] ${data}`);
      });
    }

    bridgeProcess.stderr.on('data', (data) => {
      const output = data.toString();
      if (VERBOSE) {
        process.stderr.write(`[Bridge stderr] ${output}`);
      }
    });

    bridgeProcess.on('error', (error) => {
      console.error(`[Bridge Error] ${error.message}`);
    });

    // Give bridge time to initialize
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Step 2: Connect MCP client to bridge via stdio
    log('Step 2: Connecting MCP client to bridge...');

    client = new Client(
      { name: 'test-client', version: '1.0.0' },
      { capabilities: { tools: {} } }
    );

    const transport = new StdioClientTransport({
      command: 'node',
      args: ['./dist/cli.js', '--server', PIERRE_SERVER_URL, '--verbose']
    });

    await client.connect(transport);
    log('✅ Client connected to bridge');

    // Step 3: Get initial tools list (should only have connect_to_pierre)
    log('Step 3: Getting initial tools list...');
    const initialTools = await client.listTools();

    log(`Found ${initialTools.tools.length} tools before authentication:`);
    initialTools.tools.forEach(tool => {
      log(`  - ${tool.name}: ${tool.description}`);
    });

    if (initialTools.tools.length !== 1 || initialTools.tools[0].name !== 'connect_to_pierre') {
      throw new Error(`Expected only connect_to_pierre tool, but got: ${initialTools.tools.map(t => t.name).join(', ')}`);
    }
    log('✅ Initial tools list correct (only connect_to_pierre)');

    // Step 4: Call connect_to_pierre (this would normally open browser)
    log('Step 4: Attempting to call connect_to_pierre...');
    log('⚠️  Note: This test requires manual OAuth flow completion');
    log('⚠️  The browser will open - please login with:');
    log('⚠️    Email: user@example.com');
    log('⚠️    Password: userpass123');
    log('');
    log('Waiting 60 seconds for OAuth flow to complete...');

    // Call the tool but don't await - let it trigger OAuth flow
    const connectPromise = client.callTool({
      name: 'connect_to_pierre',
      arguments: {}
    });

    // Wait for OAuth flow (this gives user time to authenticate)
    await new Promise(resolve => setTimeout(resolve, 60000));

    // Try to get the result
    try {
      const connectResult = await Promise.race([
        connectPromise,
        new Promise((_, reject) => setTimeout(() => reject(new Error('Connect timeout')), 5000))
      ]);
      log(`✅ connect_to_pierre result: ${JSON.stringify(connectResult, null, 2)}`);
    } catch (error) {
      log(`⚠️  connect_to_pierre call status unknown: ${error.message}`);
    }

    // Step 5: Get tools list again (should now have all fitness tools)
    log('Step 5: Getting authenticated tools list...');
    const authenticatedTools = await client.listTools();

    log(`Found ${authenticatedTools.tools.length} tools after authentication:`);
    authenticatedTools.tools.forEach(tool => {
      log(`  - ${tool.name}: ${tool.description}`);
    });

    // Verify we have more than just connect_to_pierre
    const hasStravaTools = authenticatedTools.tools.some(t =>
      t.name.includes('activities') ||
      t.name.includes('athlete') ||
      t.name.includes('strava')
    );

    if (!hasStravaTools) {
      throw new Error(`Expected Strava tools after authentication, but only got: ${authenticatedTools.tools.map(t => t.name).join(', ')}`);
    }

    log('✅ Authenticated tools list includes fitness tools!');

    // Step 6: Try to call a fitness tool
    log('Step 6: Testing fitness tool call...');
    const fitnessTools = authenticatedTools.tools.filter(t =>
      t.name !== 'connect_to_pierre' &&
      t.name !== 'connect_provider'
    );

    if (fitnessTools.length > 0) {
      const testTool = fitnessTools[0];
      log(`Calling ${testTool.name}...`);

      try {
        const toolResult = await client.callTool({
          name: testTool.name,
          arguments: {}
        });
        log(`✅ Tool call successful: ${testTool.name}`);
        debug(`Result: ${JSON.stringify(toolResult, null, 2)}`);
      } catch (error) {
        log(`⚠️  Tool call failed (may need Strava connection): ${error.message}`);
      }
    }

    log('');
    log('=== Test Summary ===');
    log(`✅ Initial tools: ${initialTools.tools.length} (connect_to_pierre only)`);
    log(`✅ After auth: ${authenticatedTools.tools.length} tools available`);
    log(`✅ Tools refresh: ${hasStravaTools ? 'SUCCESS' : 'FAILED'}`);
    log('');
    log('🎉 OAuth tools refresh test PASSED!');

  } catch (error) {
    console.error('❌ Test failed:', error.message);
    console.error(error.stack);
    process.exit(1);
  } finally {
    // Cleanup
    if (client) {
      try {
        await client.close();
      } catch (e) {
        debug(`Error closing client: ${e.message}`);
      }
    }

    if (bridgeProcess) {
      bridgeProcess.kill();
    }
  }
}

// Run test
testOAuthToolsRefresh().catch(error => {
  console.error('Fatal error:', error);
  process.exit(1);
});