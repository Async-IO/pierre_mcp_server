#!/usr/bin/env node
// ABOUTME: Test script to debug get_activities via the actual bridge.js
// ABOUTME: Simulates exactly what Claude Desktop does

const { Client } = require('@modelcontextprotocol/sdk/client/index.js');
const { StreamableHTTPClientTransport } = require('@modelcontextprotocol/sdk/client/streamableHttp.js');

async function testGetActivities() {
  console.log('=== Testing get_activities via Bridge ===\n');

  // Get JWT token from user login (testuser_mcp@example.com, expires 2025-10-01)
  const JWT_TOKEN = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIwNDAxYjZmOS01NzdjLTQ3YjctOTU3YS1mZTMxZGJmMGQ3NzgiLCJlbWFpbCI6InRlc3R1c2VyX21jcEBleGFtcGxlLmNvbSIsImlhdCI6MTc1OTIwMDUzMjAwMCwiZXhwIjoxNzU5Mjg2OTMyLCJwcm92aWRlcnMiOltdfQ.Wf0Si57W6gTirzZFDiYKZdKqKZeibMLBk2WWUbpfwSs";

  const serverUrl = 'http://127.0.0.1:8081/mcp';

  console.log('Server URL:', serverUrl);
  console.log('Using existing JWT token from user\n');

  // Create transport with auth header
  const transport = new StreamableHTTPClientTransport(
    new URL(serverUrl),
    {
      requestInit: {
        headers: {
          'Authorization': `Bearer ${JWT_TOKEN}`
        }
      }
    }
  );

  const client = new Client(
    {
      name: 'test-bridge-client',
      version: '1.0.0'
    },
    {
      capabilities: {
        tools: {}
      }
    }
  );

  try {
    console.log('1. Connecting to server...');
    await client.connect(transport);
    console.log('✅ Connected\n');

    console.log('2. Listing tools...');
    const toolsResult = await client.listTools();
    console.log(`✅ Found ${toolsResult.tools.length} tools`);
    const getActivitiesTool = toolsResult.tools.find(t => t.name === 'get_activities');
    if (getActivitiesTool) {
      console.log('   - get_activities tool found:', JSON.stringify(getActivitiesTool.inputSchema, null, 2));
    }
    console.log('');

    console.log('3. Calling get_activities...');
    const result = await client.callTool({
      name: 'get_activities',
      arguments: {
        provider: 'strava',
        limit: 10
      }
    });

    console.log('✅ Success! Result:');
    console.log(JSON.stringify(result, null, 2));

  } catch (error) {
    console.error('❌ Error:', error.message);
    console.error('Error details:', error);

    if (error.code) {
      console.error('MCP Error Code:', error.code);
    }
    if (error.data) {
      console.error('MCP Error Data:', error.data);
    }

    process.exit(1);
  } finally {
    await client.close();
  }
}

testGetActivities().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});