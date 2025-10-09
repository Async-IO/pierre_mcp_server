#!/usr/bin/env node

// ABOUTME: Complete OAuth flow test replicating exact Claude Desktop behavior
// ABOUTME: Tests initialization, authentication, tools refresh, and provider connection using MCP SDK
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Complete OAuth Flow Test - Mimics Claude Desktop exactly
 *
 * Uses 100% MCP SDK - no custom HTTP calls
 *
 * Flow:
 * 1. Initialize connection
 * 2. Get initial tools (should be connect_to_pierre only)
 * 3. Call connect_to_pierre (manual OAuth login)
 * 4. Verify tools refresh (should have all fitness tools)
 * 5. Call connect_provider for Strava
 */

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StdioClientTransport } = require('@modelcontextprotocol/sdk/client/stdio.js');
const { spawn } = require('child_process');

const PIERRE_SERVER_URL = process.env.PIERRE_SERVER_URL || 'http://localhost:8081';
const TEST_TIMEOUT = 90000; // 90 seconds for OAuth

function log(message) {
  console.log(`[Test] ${message}`);
}

function error(message) {
  console.error(`[Test] ❌ ${message}`);
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function testCompleteFlow() {
  log('=== Complete OAuth Flow Test ===');
  log('');

  let client;
  let transport;

  try {
    // Step 1: Initialize connection (like Claude Desktop startup)
    log('Step 1: Initialize MCP client...');

    client = new Client(
      { name: 'test-client', version: '1.0.0' },
      { capabilities: { tools: {} } }
    );

    transport = new StdioClientTransport({
      command: 'node',
      args: ['./dist/cli.js', '--server', PIERRE_SERVER_URL]
    });

    await client.connect(transport);
    log('✅ Client connected to bridge via stdio');
    log('');

    // Step 2: Get initial tools (should only be connect_to_pierre)
    log('Step 2: Get initial tools list...');
    const initialTools = await client.listTools();

    log(`Found ${initialTools.tools.length} tool(s):`);
    initialTools.tools.forEach(tool => {
      log(`  - ${tool.name}`);
    });

    if (initialTools.tools.length !== 1 || initialTools.tools[0].name !== 'connect_to_pierre') {
      throw new Error(`Expected only connect_to_pierre, got: ${initialTools.tools.map(t => t.name).join(', ')}`);
    }
    log('✅ Initial tools correct: only connect_to_pierre');
    log('');

    // Step 3: Call connect_to_pierre (triggers OAuth)
    log('Step 3: Call connect_to_pierre...');
    log('⚠️  Browser will open - please login with:');
    log('    Email: user@example.com');
    log('    Password: userpass123');
    log('');
    log('Starting OAuth flow...');

    const connectPromise = client.callTool({
      name: 'connect_to_pierre',
      arguments: {}
    });

    // Wait up to 90 seconds for user to complete OAuth
    log('Waiting for OAuth completion (max 90 seconds)...');
    const connectResult = await Promise.race([
      connectPromise,
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error('OAuth timeout - please complete login faster')), TEST_TIMEOUT)
      )
    ]);

    log('✅ connect_to_pierre completed:');
    log(`   ${connectResult.content[0].text}`);
    log('');

    // Give bridge time to establish connection
    await sleep(2000);

    // Step 4: Get tools again (should now have ALL fitness tools)
    log('Step 4: Get authenticated tools list...');
    const authenticatedTools = await client.listTools();

    log(`Found ${authenticatedTools.tools.length} tool(s):`);
    if (authenticatedTools.tools.length <= 5) {
      // Show all tools if small list
      authenticatedTools.tools.forEach(tool => {
        log(`  - ${tool.name}`);
      });
    } else {
      // Show first 5 if large list
      authenticatedTools.tools.slice(0, 5).forEach(tool => {
        log(`  - ${tool.name}`);
      });
      log(`  ... and ${authenticatedTools.tools.length - 5} more`);
    }

    // Verify we have fitness tools
    const hasFitnessTools = authenticatedTools.tools.some(t =>
      t.name === 'get_activities' ||
      t.name === 'get_athlete' ||
      t.name === 'connect_provider'
    );

    if (!hasFitnessTools) {
      throw new Error(`Expected fitness tools after auth, but got: ${authenticatedTools.tools.map(t => t.name).join(', ')}`);
    }

    if (authenticatedTools.tools.length < 10) {
      throw new Error(`Expected many tools after auth, but only got ${authenticatedTools.tools.length}`);
    }

    log('✅ Authenticated tools list has all fitness tools!');
    log('');

    // Step 5: Call connect_provider for Strava
    log('Step 5: Call connect_provider for Strava...');

    const hasConnectProvider = authenticatedTools.tools.some(t => t.name === 'connect_provider');
    if (!hasConnectProvider) {
      throw new Error('connect_provider tool not found in authenticated tools list');
    }

    const stravaResult = await client.callTool({
      name: 'connect_provider',
      arguments: {
        provider: 'strava'
      }
    });

    log('✅ connect_provider completed:');
    log(`   ${stravaResult.content[0].text}`);
    log('');

    // Success summary
    log('');
    log('=== Test Summary ===');
    log(`✅ Step 1: Client initialized`);
    log(`✅ Step 2: Initial tools correct (${initialTools.tools.length} tool)`);
    log(`✅ Step 3: connect_to_pierre succeeded`);
    log(`✅ Step 4: Tools refreshed (${authenticatedTools.tools.length} tools)`);
    log(`✅ Step 5: connect_provider for Strava succeeded`);
    log('');
    log('🎉 ALL TESTS PASSED!');

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
testCompleteFlow().catch(err => {
  error(`Fatal error: ${err.message}`);
  process.exit(1);
});