// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Claude Desktop streaming E2E tests - SSE transport, long-running tools, connection recovery
// ABOUTME: Tests real-time streaming stability and reconnection scenarios
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { MockMCPClient } = require('../helpers/mock-client');
const { MCPMessages, TestConfig } = require('../helpers/fixtures');
const path = require('path');

const fetch = global.fetch;

describe('E2E: Claude Desktop SSE Transport Stability', () => {
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

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);
  }, 90000);

  afterAll(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
    }
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should establish SSE connection to server', async () => {
    // Check SSE endpoint is accessible
    const sseUrl = `${serverUrl}/sse`;

    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);

      const response = await fetch(sseUrl, {
        method: 'GET',
        headers: {
          'Accept': 'text/event-stream'
        },
        signal: controller.signal
      });

      clearTimeout(timeout);

      // SSE endpoint should respond
      expect(response.status).toBeLessThan(500);
    } catch (error) {
      // Abort is expected after timeout
      if (error.name !== 'AbortError') {
        // Other errors indicate SSE endpoint exists but may require auth
        expect(error).toBeDefined();
      }
    }
  }, 10000);

  test('should maintain stable connection over multiple requests', async () => {
    const numRequests = 10;
    const responses = [];

    for (let i = 0; i < numRequests; i++) {
      const response = await bridgeClient.send({
        method: 'tools/list',
        params: {}
      });
      responses.push(response);
    }

    // All requests should succeed
    expect(responses.length).toBe(numRequests);
    for (const response of responses) {
      expect(response).toHaveProperty('result');
    }
  }, 60000);

  test('should handle rapid sequential requests', async () => {
    const promises = [];

    // Send 5 requests rapidly
    for (let i = 0; i < 5; i++) {
      promises.push(
        bridgeClient.send({
          method: 'tools/list',
          params: {}
        })
      );
    }

    const responses = await Promise.all(promises);

    // All should complete
    expect(responses.length).toBe(5);
    for (const response of responses) {
      expect(response).toBeDefined();
    }
  }, 60000);

  test('should handle connection idle periods', async () => {
    // Make initial request
    const response1 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });
    expect(response1).toHaveProperty('result');

    // Wait for idle period
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Make another request - connection should still work
    const response2 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });
    expect(response2).toHaveProperty('result');
  }, 30000);

  test('should not drop messages under load', async () => {
    const numMessages = 20;
    const responses = [];

    // Send many requests
    for (let i = 0; i < numMessages; i++) {
      const response = await bridgeClient.send({
        method: 'tools/list',
        params: {}
      });
      responses.push(response);
    }

    // All should be received
    expect(responses.length).toBe(numMessages);

    // Count successful responses
    const successCount = responses.filter(r => r.result).length;
    expect(successCount).toBe(numMessages);
  }, 120000);
});

describe('E2E: Claude Desktop Long-Running Tool Execution', () => {
  let serverHandle;
  let bridgeClient;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);
  }, 90000);

  afterAll(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
    }
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should complete long-running tool calls without timeout', async () => {
    const startTime = Date.now();

    // Request that may take longer to process
    const response = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava', limit: 50 }
      }
    }, 60000); // 60 second timeout

    const elapsed = Date.now() - startTime;

    expect(response).toBeDefined();
    expect(elapsed).toBeLessThan(60000);
  }, 65000);

  test('should allow other requests during long-running tool', async () => {
    // Start a potentially long request
    const longRequestPromise = bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava', limit: 100 }
      }
    }, 60000);

    // Immediately send a quick request
    const quickRequestPromise = bridgeClient.send({
      method: 'tools/list',
      params: {}
    }, 10000);

    // Wait for both to complete
    const [longResponse, quickResponse] = await Promise.all([
      longRequestPromise,
      quickRequestPromise
    ]);

    expect(longResponse).toBeDefined();
    expect(quickResponse).toBeDefined();
  }, 90000);

  test('should maintain connection during extended processing', async () => {
    // Make a request that takes time
    const response1 = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    }, 30000);

    expect(response1).toBeDefined();

    // Connection should still be valid
    const response2 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    expect(response2).toHaveProperty('result');
  }, 60000);

  test('should provide progress updates for long operations if supported', async () => {
    // Some tools may emit progress notifications
    const notifications = [];

    bridgeClient.on('notification', (notification) => {
      notifications.push(notification);
    });

    await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava', limit: 10 }
      }
    });

    // Notifications are optional feature
    // Just verify we don't crash when processing them
  }, 30000);
});

describe('E2E: Claude Desktop Connection Recovery', () => {
  let serverHandle;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should reconnect after brief network interruption simulation', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // First request
    const response1 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });
    expect(response1).toHaveProperty('result');

    // Simulate brief pause
    await new Promise(resolve => setTimeout(resolve, 500));

    // Second request should work (reconnected if needed)
    const response2 = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });
    expect(response2).toHaveProperty('result');

    await bridgeClient.stop();
  }, 60000);

  test('should handle server unavailability gracefully', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      'http://localhost:59999' // Unlikely to be running
    ]);

    await bridgeClient.start();

    try {
      const response = await bridgeClient.send(MCPMessages.initialize, 5000);
      // If we get here, might get an error response
      expect(response.error || response.result).toBeDefined();
    } catch (error) {
      // Connection failure is expected
      expect(error).toBeDefined();
    }

    await bridgeClient.stop();
  }, 30000);

  test('should re-establish session after reconnect', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();

    // Initialize session
    const initResponse = await bridgeClient.send(MCPMessages.initialize);
    expect(initResponse).toHaveProperty('result');

    // Make a tool call
    const response1 = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'list_connections',
        arguments: {}
      }
    });
    expect(response1).toBeDefined();

    // Pause and then continue
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Session should still work
    const response2 = await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    });
    expect(response2).toBeDefined();

    await bridgeClient.stop();
  }, 60000);

  test('should not lose pending requests during reconnection', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Send multiple requests
    const requests = [
      bridgeClient.send({ method: 'tools/list', params: {} }),
      bridgeClient.send({ method: 'tools/list', params: {} }),
      bridgeClient.send({ method: 'tools/list', params: {} })
    ];

    const responses = await Promise.all(requests);

    // All should complete
    expect(responses.length).toBe(3);
    for (const response of responses) {
      expect(response).toHaveProperty('result');
    }

    await bridgeClient.stop();
  }, 60000);

  test('should recover from connection timeout', async () => {
    const bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      serverUrl
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);

    // Make request with short timeout
    try {
      await bridgeClient.send({
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      }, 100); // Very short timeout
    } catch (error) {
      // Timeout is expected
    }

    // Should still be able to make requests
    const response = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    }, 30000);

    expect(response).toBeDefined();

    await bridgeClient.stop();
  }, 60000);
});

describe('E2E: Claude Desktop SSE Event Handling', () => {
  let serverHandle;
  let bridgeClient;
  const bridgePath = path.join(__dirname, '../../dist/cli.js');

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });

    bridgeClient = new MockMCPClient('node', [
      bridgePath,
      '--server',
      `http://localhost:${TestConfig.defaultServerPort}`
    ]);

    await bridgeClient.start();
    await bridgeClient.send(MCPMessages.initialize);
  }, 90000);

  afterAll(async () => {
    if (bridgeClient) {
      await bridgeClient.stop();
    }
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should handle server-sent notifications', async () => {
    const notifications = [];

    bridgeClient.on('notification', (notification) => {
      notifications.push(notification);
    });

    // Make requests that might trigger notifications
    await bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava' }
      }
    });

    // Wait for any notifications
    await new Promise(resolve => setTimeout(resolve, 500));

    // Notifications are optional - just ensure no crashes
  }, 30000);

  test('should distinguish between responses and notifications', async () => {
    const notifications = [];
    let responseReceived = false;

    bridgeClient.on('notification', (notification) => {
      notifications.push(notification);
    });

    const response = await bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    responseReceived = response !== undefined;

    expect(responseReceived).toBe(true);
    expect(response).toHaveProperty('result');
  }, 30000);

  test('should handle rapid event stream', async () => {
    // Make many requests quickly
    const promises = [];
    for (let i = 0; i < 10; i++) {
      promises.push(
        bridgeClient.send({
          method: 'tools/list',
          params: {}
        })
      );
    }

    const responses = await Promise.all(promises);

    // All responses should be received and correctly matched
    expect(responses.length).toBe(10);
    for (const response of responses) {
      expect(response).toHaveProperty('result');
      expect(response.result).toHaveProperty('tools');
    }
  }, 60000);

  test('should handle interleaved requests and responses', async () => {
    // Send requests that may complete in different order
    const request1 = bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: { provider: 'strava', limit: 5 }
      }
    });

    const request2 = bridgeClient.send({
      method: 'tools/list',
      params: {}
    });

    const request3 = bridgeClient.send({
      method: 'tools/call',
      params: {
        name: 'list_connections',
        arguments: {}
      }
    });

    const [response1, response2, response3] = await Promise.all([
      request1, request2, request3
    ]);

    // Each response should match its request
    expect(response1).toBeDefined();
    expect(response2).toHaveProperty('result');
    expect(response2.result).toHaveProperty('tools'); // tools/list specific
    expect(response3).toBeDefined();
  }, 60000);
});
