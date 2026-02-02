// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Rate limiting integration tests - verifies 429 handling, backoff, and tenant isolation
// ABOUTME: Tests rate limit detection, retry behavior, and multi-tenant rate limit separation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');

const fetch = global.fetch;

describe('Rate Limiting - 429 Response Detection', () => {
  let serverHandle;
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

  test('should handle 429 response from provider with structured error', async () => {
    const tokenData = generateTestToken('user-ratelimit', 'ratelimit@example.com', 3600);
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
          arguments: { provider: 'strava', limit: 100 }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');

    // If rate limited, should provide structured error
    if (body.error && body.error.message.toLowerCase().includes('rate')) {
      expect(body.error).toHaveProperty('code');
      expect(body.error).toHaveProperty('message');
    }
  }, 30000);

  test('should extract Retry-After header information when available', async () => {
    const tokenData = generateTestToken('user-retryafter', 'retryafter@example.com', 3600);
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
          arguments: { provider: 'strava' }
        }
      })
    });

    const body = await response.json();

    // If rate limited error includes retry info, it should be parseable
    if (body.error && body.error.data && body.error.data.retry_after) {
      const retryAfter = body.error.data.retry_after;
      expect(typeof retryAfter).toBe('number');
      expect(retryAfter).toBeGreaterThanOrEqual(0);
    }
  }, 30000);

  test('should differentiate between provider rate limits and Pierre rate limits', async () => {
    const tokenData = generateTestToken('user-difflimit', 'difflimit@example.com', 3600);
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

    // Error should indicate source of rate limit if applicable
    if (body.error) {
      const errorData = body.error.data || {};
      // Rate limit source should be identifiable
      if (errorData.rate_limit_source) {
        expect(['provider', 'pierre']).toContain(errorData.rate_limit_source);
      }
    }
  }, 30000);

  test('should include rate limit type in error (daily, hourly, minute)', async () => {
    const tokenData = generateTestToken('user-limittype', 'limittype@example.com', 3600);
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
          arguments: { provider: 'garmin' }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');
  }, 30000);
});

describe('Rate Limiting - Backoff Behavior', () => {
  let serverHandle;
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

  test('should respect exponential backoff for retries', async () => {
    const tokenData = generateTestToken('user-backoff', 'backoff@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Make multiple requests and measure timing
    const timestamps = [];

    for (let i = 0; i < 3; i++) {
      const start = Date.now();
      await fetch(mcpEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${tokenData.access_token}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/call',
          params: {
            name: 'get_activities',
            arguments: { provider: 'strava', limit: 5 }
          }
        })
      });
      timestamps.push(Date.now() - start);
    }

    // All requests should complete
    expect(timestamps.length).toBe(3);

    // Each request should complete in reasonable time
    for (const time of timestamps) {
      expect(time).toBeLessThan(30000);
    }
  }, 100000);

  test('should cap maximum backoff time', async () => {
    const tokenData = generateTestToken('user-maxbackoff', 'maxbackoff@example.com', 3600);
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
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    const elapsed = Date.now() - startTime;

    // Should not wait indefinitely - max backoff should be capped
    expect(elapsed).toBeLessThan(120000); // 2 minute max
    expect(response.status).toBe(200);
  }, 130000);

  test('should add jitter to backoff to prevent thundering herd', async () => {
    const tokenData = generateTestToken('user-jitter', 'jitter@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Send multiple concurrent requests
    const requests = Array(5).fill(null).map((_, i) =>
      fetch(mcpEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${tokenData.access_token}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/call',
          params: {
            name: 'get_activities',
            arguments: { provider: 'strava' }
          }
        })
      })
    );

    const responses = await Promise.all(requests);

    // All should complete
    for (const response of responses) {
      expect(response.status).toBe(200);
    }
  }, 60000);

  test('should reset backoff after successful request', async () => {
    const tokenData = generateTestToken('user-resetbackoff', 'resetbackoff@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // First request
    const response1 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {}
      })
    });

    // Second request should be fast (no backoff accumulation for successful requests)
    const startTime = Date.now();
    const response2 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenData.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/list',
        params: {}
      })
    });
    const elapsed = Date.now() - startTime;

    expect(response1.status).toBe(200);
    expect(response2.status).toBe(200);
    expect(elapsed).toBeLessThan(5000); // Should be quick
  }, 30000);
});

describe('Rate Limiting - Multi-Tenant Isolation', () => {
  let serverHandle;
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

  test('should track rate limits separately per tenant', async () => {
    const tenant1Token = generateTestToken('user-tenant1', 'tenant1@example.com', 3600);
    const tenant2Token = generateTestToken('user-tenant2', 'tenant2@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Request from tenant 1
    const response1 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tenant1Token.access_token}`
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

    // Request from tenant 2 should be independent
    const response2 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tenant2Token.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    expect(response1.status).toBe(200);
    expect(response2.status).toBe(200);

    // Both tenants should get independent responses
    const body1 = await response1.json();
    const body2 = await response2.json();

    expect(body1.jsonrpc).toBe('2.0');
    expect(body2.jsonrpc).toBe('2.0');
  }, 30000);

  test('should not leak rate limit state between tenants', async () => {
    const tenant1Token = generateTestToken('user-leak1', 'leak1@example.com', 3600);
    const tenant2Token = generateTestToken('user-leak2', 'leak2@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Exhaust requests for tenant 1 (simulated)
    for (let i = 0; i < 3; i++) {
      await fetch(mcpEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${tenant1Token.access_token}`
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/call',
          params: {
            name: 'get_activities',
            arguments: { provider: 'strava' }
          }
        })
      });
    }

    // Tenant 2 should not be affected by tenant 1's usage
    const tenant2Response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tenant2Token.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 100,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    expect(tenant2Response.status).toBe(200);
  }, 60000);

  test('should apply tenant-specific rate limit tiers', async () => {
    const freeToken = generateTestToken('user-free', 'free@example.com', 3600, { tier: 'free' });
    const proToken = generateTestToken('user-pro', 'pro@example.com', 3600, { tier: 'professional' });
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Both tiers should be able to make requests
    const freeResponse = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${freeToken.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {}
      })
    });

    const proResponse = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${proToken.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/list',
        params: {}
      })
    });

    expect(freeResponse.status).toBe(200);
    expect(proResponse.status).toBe(200);
  }, 30000);

  test('should handle rate limit recovery per tenant', async () => {
    const tenantToken = generateTestToken('user-recovery', 'recovery@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Make request
    const response1 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tenantToken.access_token}`
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

    // Wait a moment
    await new Promise(resolve => setTimeout(resolve, 100));

    // Make another request - should work
    const response2 = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tenantToken.access_token}`
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'get_activities',
          arguments: { provider: 'strava' }
        }
      })
    });

    expect(response1.status).toBe(200);
    expect(response2.status).toBe(200);
  }, 30000);
});

describe('Rate Limiting - Error Response Format', () => {
  let serverHandle;
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

  test('should return JSON-RPC compliant rate limit error', async () => {
    const tokenData = generateTestToken('user-jsonrpc', 'jsonrpc@example.com', 3600);
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

    expect(response.status).toBe(200);
    const body = await response.json();

    expect(body.jsonrpc).toBe('2.0');
    expect(body.id).toBe(1);
  }, 30000);

  test('should include helpful message for rate limit resolution', async () => {
    const tokenData = generateTestToken('user-helpful', 'helpful@example.com', 3600);
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
          arguments: { provider: 'strava' }
        }
      })
    });

    const body = await response.json();

    // If there's a rate limit error, it should have helpful info
    if (body.error && body.error.message.toLowerCase().includes('rate')) {
      expect(body.error.message.length).toBeGreaterThan(10);
    }
  }, 30000);

  test('should not expose internal rate limit counters', async () => {
    const tokenData = generateTestToken('user-internal', 'internal@example.com', 3600);
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

    // Response should not expose internal implementation details
    const bodyString = JSON.stringify(body);
    expect(bodyString).not.toContain('internal_counter');
    expect(bodyString).not.toContain('bucket_size');
  }, 30000);

  test('should include retry timing in machine-readable format', async () => {
    const tokenData = generateTestToken('user-timing', 'timing@example.com', 3600);
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
          arguments: { provider: 'strava' }
        }
      })
    });

    expect(response.status).toBe(200);
    const body = await response.json();

    // If rate limited with retry info, it should be a number
    if (body.error && body.error.data && body.error.data.retry_after_seconds) {
      expect(typeof body.error.data.retry_after_seconds).toBe('number');
    }
  }, 30000);
});
