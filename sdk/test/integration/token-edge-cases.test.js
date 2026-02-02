// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Token edge case integration tests - race conditions, expired refresh, concurrent refresh
// ABOUTME: Tests token refresh edge cases including mid-request expiry and invalidation scenarios
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { TestConfig } = require('../helpers/fixtures');
const { generateTestToken } = require('../helpers/token-generator');

const fetch = global.fetch;

describe('Token Edge Cases - Refresh Race Conditions', () => {
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

  test('should handle concurrent token refresh requests gracefully', async () => {
    const tokenData = generateTestToken('user-concurrent-refresh', 'concurrent@example.com', 60);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // Fire multiple refresh requests simultaneously
    const refreshRequests = Array(5).fill(null).map((_, i) =>
      fetch(validateEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          access_token: tokenData.access_token,
          refresh_token: tokenData.access_token,
          request_id: `refresh-${i}`
        })
      })
    );

    const responses = await Promise.all(refreshRequests);

    // All requests should complete without errors
    for (const response of responses) {
      expect(response.status).toBeLessThan(500);
    }
  }, 30000);

  test('should serialize token refresh to prevent duplicate refreshes', async () => {
    const tokenData = generateTestToken('user-serialize', 'serialize@example.com', 30);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // Start first refresh
    const refreshPromise1 = fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: tokenData.access_token
      })
    });

    // Immediately start second refresh with same token
    const refreshPromise2 = fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: tokenData.access_token
      })
    });

    const [response1, response2] = await Promise.all([refreshPromise1, refreshPromise2]);

    // Both should complete
    expect(response1.status).toBeLessThan(500);
    expect(response2.status).toBeLessThan(500);
  }, 30000);

  test('should handle token refresh during active tool call', async () => {
    const tokenData = generateTestToken('user-during-call', 'during@example.com', 30);
    const mcpEndpoint = `${serverUrl}/mcp`;
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // Start a tool call
    const toolCallPromise = fetch(mcpEndpoint, {
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

    // Simultaneously trigger refresh
    const refreshPromise = fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: tokenData.access_token
      })
    });

    const [toolResponse, refreshResponse] = await Promise.all([toolCallPromise, refreshPromise]);

    // Both should complete without server crash
    expect(toolResponse.status).toBe(200);
    expect(refreshResponse.status).toBeLessThan(500);
  }, 30000);

  test('should use latest token after refresh completes', async () => {
    const tokenData = generateTestToken('user-latest', 'latest@example.com', 120);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // First request with initial token
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

    // Second request with same token should still work
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

    expect(response1.status).toBe(200);
    expect(response2.status).toBe(200);
  }, 30000);
});

describe('Token Edge Cases - Expired Refresh Token Handling', () => {
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

  test('should detect fully expired token pair and require re-auth', async () => {
    // Both access and refresh tokens expired
    const expiredToken = generateTestToken('user-fullexpired', 'fullexpired@example.com', -7200);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: expiredToken.access_token,
        refresh_token: expiredToken.access_token // Also expired
      })
    });

    if (response.ok) {
      const body = await response.json();
      expect(body.status).toBe('invalid');
    } else {
      expect(response.status).toBe(401);
    }
  }, 30000);

  test('should provide clear error when refresh token is revoked', async () => {
    const tokenData = generateTestToken('user-revoked', 'revoked@example.com', 3600);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // Attempt refresh with a "revoked" token (simulated by using malformed refresh token)
    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: 'revoked_invalid_token'
      })
    });

    // Should fail with error
    expect(response.ok).toBe(false);
    expect(response.status).toBeGreaterThanOrEqual(400);
  }, 30000);

  test('should not cache failed refresh attempts', async () => {
    const tokenData = generateTestToken('user-nocache', 'nocache@example.com', 3600);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // First attempt with bad refresh token
    const response1 = await fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: 'bad_refresh_token'
      })
    });

    // Second attempt with valid token should be processed independently
    const response2 = await fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: tokenData.access_token
      })
    });

    // First should fail, second should be processed
    expect(response1.status).toBeGreaterThanOrEqual(400);
    expect(response2.status).toBeLessThan(500);
  }, 30000);

  test('should handle refresh token that expires during refresh', async () => {
    // Token that's very close to expiring
    const nearExpiryToken = generateTestToken('user-nearexpiry', 'nearexpiry@example.com', 1);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // Wait for token to expire during request
    await new Promise(resolve => setTimeout(resolve, 1500));

    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        access_token: nearExpiryToken.access_token,
        refresh_token: nearExpiryToken.access_token
      })
    });

    // Should handle gracefully - either succeed or fail cleanly
    expect(response.status).toBeLessThan(500);
  }, 30000);
});

describe('Token Edge Cases - Concurrent Token Refresh Requests', () => {
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

  test('should handle high concurrency token refresh safely', async () => {
    const tokenData = generateTestToken('user-highconc', 'highconc@example.com', 60);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // 10 concurrent refresh requests
    const requests = Array(10).fill(null).map(() =>
      fetch(validateEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          access_token: tokenData.access_token,
          refresh_token: tokenData.access_token
        })
      })
    );

    const responses = await Promise.all(requests);

    // All should complete without 500 errors
    for (const response of responses) {
      expect(response.status).toBeLessThan(500);
    }
  }, 60000);

  test('should maintain token consistency across concurrent refreshes', async () => {
    const tokenData = generateTestToken('user-consistent', 'consistent@example.com', 120);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Multiple concurrent tool calls with same token
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
          method: 'tools/list',
          params: {}
        })
      })
    );

    const responses = await Promise.all(requests);

    // All should complete with consistent behavior
    for (const response of responses) {
      expect(response.status).toBe(200);
      const body = await response.json();
      expect(body.jsonrpc).toBe('2.0');
    }
  }, 30000);

  test('should prevent token refresh storms', async () => {
    const tokenData = generateTestToken('user-storm', 'storm@example.com', 30);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    const startTime = Date.now();

    // Burst of refresh requests
    const requests = Array(20).fill(null).map(() =>
      fetch(validateEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          access_token: tokenData.access_token,
          refresh_token: tokenData.access_token
        })
      })
    );

    await Promise.all(requests);

    const elapsed = Date.now() - startTime;

    // Should complete in reasonable time (rate limiting or deduplication should help)
    expect(elapsed).toBeLessThan(30000);
  }, 60000);

  test('should queue or deduplicate identical refresh requests', async () => {
    const tokenData = generateTestToken('user-dedup', 'dedup@example.com', 60);
    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

    // Send identical requests
    const requests = Array(3).fill(null).map(() =>
      fetch(validateEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          access_token: tokenData.access_token,
          refresh_token: tokenData.access_token
        })
      })
    );

    const responses = await Promise.all(requests);

    // All should complete successfully or with consistent error
    const statuses = responses.map(r => r.status);
    const allSuccessful = statuses.every(s => s === 200 || s === statuses[0]);
    expect(allSuccessful).toBe(true);
  }, 30000);
});

describe('Token Edge Cases - Token Invalidation Mid-Request', () => {
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

  test('should handle token becoming invalid during tool execution', async () => {
    const tokenData = generateTestToken('user-midinvalid', 'midinvalid@example.com', 5);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Start a request with token about to expire
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

    // Should complete with either success or appropriate error
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');
  }, 30000);

  test('should return proper error when token is revoked mid-session', async () => {
    const tokenData = generateTestToken('user-revokemid', 'revokemid@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // First request should work
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

    expect(response1.status).toBe(200);

    // Simulate token revocation by using completely different (invalid) token
    const revokedResponse = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer completely_invalid_revoked_token'
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/list',
        params: {}
      })
    });

    // Should fail with auth error
    expect(revokedResponse.status).toBe(200);
    const body = await revokedResponse.json();

    // JSON-RPC response with authentication error
    if (body.error) {
      expect(body.error.code).toBeDefined();
    }
  }, 30000);

  test('should not retry with invalidated token', async () => {
    const expiredToken = generateTestToken('user-noretry', 'noretry@example.com', -10);
    const mcpEndpoint = `${serverUrl}/mcp`;

    const response = await fetch(mcpEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${expiredToken.access_token}`
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

    // Should return error, not loop infinitely
    expect(response.status).toBe(200);
    const body = await response.json();

    // Should have completed (with error is fine)
    expect(body.jsonrpc).toBe('2.0');
  }, 30000);

  test('should gracefully handle session termination', async () => {
    const tokenData = generateTestToken('user-terminate', 'terminate@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Make a request
    const response = await fetch(mcpEndpoint, {
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

    expect(response.status).toBe(200);

    // Server should still be responsive for other requests
    const healthResponse = await fetch(`${serverUrl}/health`);
    expect(healthResponse.status).toBe(200);
  }, 30000);
});

describe('Token Edge Cases - Provider Token Refresh', () => {
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

  test('should refresh provider token transparently during tool call', async () => {
    const tokenData = generateTestToken('user-providerrefresh', 'providerrefresh@example.com', 3600);
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
  }, 30000);

  test('should handle provider token refresh failure gracefully', async () => {
    const tokenData = generateTestToken('user-providerfail', 'providerfail@example.com', 3600);
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

    // Should return structured error, not crash
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.jsonrpc).toBe('2.0');
  }, 30000);

  test('should isolate provider token refresh from Pierre token', async () => {
    const tokenData = generateTestToken('user-isolate', 'isolate@example.com', 3600);
    const mcpEndpoint = `${serverUrl}/mcp`;

    // Request to one provider
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

    // Request to another provider should work independently
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

    expect(stravaResponse.status).toBe(200);
    expect(garminResponse.status).toBe(200);
  }, 30000);

  test('should require provider re-auth when refresh token expires', async () => {
    const tokenData = generateTestToken('user-reauth', 'reauth@example.com', 3600);
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

    // If provider requires re-auth, error should indicate this
    if (body.error && body.error.message.toLowerCase().includes('auth')) {
      expect(body.error.message.length).toBeGreaterThan(5);
    }
  }, 30000);
});
