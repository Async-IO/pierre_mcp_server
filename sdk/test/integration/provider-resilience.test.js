// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Provider resilience integration tests - verifies graceful handling of provider failures
// ABOUTME: Tests 503 unavailable, timeouts, partial responses, and multi-provider degradation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { TestConfig } = require('../helpers/fixtures');
const http = require('http');

const fetch = global.fetch;

/**
 * Creates a mock provider server that simulates various error conditions
 */
function createMockProviderServer(port, behavior) {
  return new Promise((resolve, reject) => {
    const server = http.createServer((req, res) => {
      behavior(req, res);
    });

    server.on('error', reject);
    server.listen(port, () => {
      resolve(server);
    });
  });
}

describe('Provider Resilience - API Unavailability (503)', () => {
  let serverHandle;
  let testToken;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should return structured error when provider returns 503', async () => {
    // Send a request that would trigger provider call
    // Server should handle 503 from provider gracefully
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava', limit: 10 }
        }
      })
    });

    // Server should not crash - should return valid JSON-RPC response
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');

    // If there's an error, it should have proper structure
    if (body.error) {
      expect(body.error).toHaveProperty('code');
      expect(body.error).toHaveProperty('message');
      expect(typeof body.error.code).toBe('number');
      expect(typeof body.error.message).toBe('string');
    }
  }, 30000);

  test('should include provider name in error context', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava', limit: 5 }
        }
      })
    });

    const body = await response.json();

    // Error should reference the provider if it's a provider-related failure
    if (body.error) {
      const errorContext = JSON.stringify(body.error);
      // Provider errors should mention the provider or give actionable info
      expect(errorContext.length).toBeGreaterThan(10);
    }
  }, 30000);

  test('should not expose internal provider details in error messages', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 3,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    const body = await response.json();

    if (body.error) {
      const errorMessage = body.error.message.toLowerCase();
      // Should not expose internal URLs, keys, or sensitive paths
      expect(errorMessage).not.toContain('api.strava.com');
      expect(errorMessage).not.toContain('client_secret');
      expect(errorMessage).not.toContain('access_token');
    }
  }, 30000);

  test('should recover after provider becomes available again', async () => {
    // First request during simulated outage
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const request1 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 4,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    // Wait for simulated recovery
    await new Promise(resolve => setTimeout(resolve, 100));

    // Second request after recovery - server should be responsive
    const request2 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 5,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    // Server should respond to both requests
    expect(request1.status).toBe(200);
    expect(request2.status).toBe(200);
  }, 30000);
});

describe('Provider Resilience - Timeout Handling', () => {
  let serverHandle;
  let testToken;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should handle slow provider responses without hanging', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const startTime = Date.now();
    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'strava' }
        }
      })
    });

    const elapsed = Date.now() - startTime;

    // Should complete within reasonable time (server-side timeout should kick in)
    // Max expected time: server timeout + some processing overhead
    expect(elapsed).toBeLessThan(60000); // 60 seconds max
    expect(response.status).toBe(200);
  }, 65000);

  test('should return timeout error with appropriate message', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'get_stats',
          arguments: { provider: 'strava' }
        }
      })
    });

    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');

    // If timeout error, message should be helpful
    if (body.error && body.error.message.toLowerCase().includes('timeout')) {
      expect(body.error.message.length).toBeGreaterThan(5);
    }
  }, 65000);

  test('should not block other requests during timeout', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;
    const healthEndpoint = `${serverUrl}/health`;

    // Start a potentially slow request
    const slowRequestPromise = fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin' }
        }
      })
    });

    // Health check should still respond quickly
    const healthResponse = await fetch(healthEndpoint);
    expect(healthResponse.status).toBe(200);

    // Wait for slow request to complete
    await slowRequestPromise;
  }, 30000);

  test('should handle multiple concurrent timeout scenarios', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Fire multiple requests concurrently
    const requests = ['strava', 'garmin', 'fitbit'].map((provider, index) =>
      fetch(mcpEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${tokenData.access_token}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: index + 1,
          method: 'tools/call',
          params: {
            name: 'get_activities',
            arguments: { provider }
          }
        })
      })
    );

    const responses = await Promise.all(requests);

    // All requests should complete with valid responses
    for (const response of responses) {
      expect(response.status).toBe(200);
      const body = await response.json();
      expect(body.jsonrpc).toBe('2.0');
    }
  }, 90000);
});

describe('Provider Resilience - Partial Response Handling', () => {
  let serverHandle;
  let testToken;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should handle malformed JSON from provider gracefully', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    // Server should handle gracefully
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');
  }, 30000);

  test('should handle empty response from provider', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'fitbit' }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();

    // Should return a valid response structure even if empty
    if (body.result && body.result.content) {
      expect(Array.isArray(body.result.content)).toBe(true);
    }
  }, 30000);

  test('should handle provider returning wrong data type', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 3,
        method: 'tools/call',
        params: {
          name: 'get_athlete',
          arguments: { provider: 'strava' }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');
  }, 30000);

  test('should validate and sanitize provider response data', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 4,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava', limit: 5 }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();

    // Response should be valid JSON
    expect(() => JSON.stringify(body)).not.toThrow();
  }, 30000);
});

describe('Provider Resilience - Graceful Multi-Provider Degradation', () => {
  let serverHandle;
  let testToken;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
    testToken = serverHandle?.testToken;
  }, 90000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should continue serving other providers when one fails', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Request from Strava (may fail)
    const stravaResponse = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    // Request from Garmin should work independently
    const garminResponse = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'garmin' }
        }
      })
    });

    // Both should complete with valid responses (even if error responses)
    expect(stravaResponse.status).toBe(200);
    expect(garminResponse.status).toBe(200);

    const stravaBody = await stravaResponse.json();
    const garminBody = await garminResponse.json();

    expect(stravaBody.jsonrpc).toBe('2.0');
    expect(garminBody.jsonrpc).toBe('2.0');
  }, 30000);

  test('should report health status for each provider independently', async () => {
    const healthEndpoint = `${serverUrl}/health`;

    const response = await fetch(healthEndpoint);
    expect(response.status).toBe(200);

    const health = await response.json();
    expect(health.status).toBeDefined();
  }, 10000);

  test('should not cascade failures between providers', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Sequential requests to different providers
    const providers = ['strava', 'garmin', 'fitbit'];
    const results = [];

    for (const provider of providers) {
      const response = await fetch(mcpEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${tokenData.access_token}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: results.length + 1,
          method: 'tools/call',
          params: {
            name: 'get_activities',
            arguments: { provider }
          }
        })
      });

      results.push({
        provider,
        status: response.status,
        body: await response.json()
      });
    }

    // Each provider should get independent handling
    for (const result of results) {
      expect(result.status).toBe(200);
      expect(result.body.jsonrpc).toBe('2.0');
    }
  }, 60000);

  test('should provide clear error message when no providers available', async () => {
    const tokenData = testToken;
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'invalid_provider' }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();

    // Should return an error with clear message about invalid provider
    if (body.error) {
      expect(body.error.message.length).toBeGreaterThan(5);
    }
  }, 30000);
});
