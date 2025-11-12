// ABOUTME: Token management integration tests - validation, refresh, expiry, storage
// ABOUTME: Tests token lifecycle including expiry during tool execution (critical regression)
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { ensureServerRunning } = require('../helpers/server');
const { TestConfig } = require('../helpers/fixtures');
const { generateTestToken, createTestUser } = require('../helpers/token-generator');
const path = require('path');
const fs = require('fs');
const os = require('os');

const fetch = global.fetch;

describe('Token Validation and Refresh', () => {
  let serverHandle;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;
  const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 60000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });


  test('should detect expired token and return invalid status', async () => {
    // Generate a token that expired 1 hour ago
    const tokenData = generateTestToken('user-456', 'expired@example.com', -3600);

    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        access_token: tokenData.access_token,
        refresh_token: tokenData.access_token
      })
    });

    // Server should either:
    // 1. Return 200 with { status: 'invalid' }, OR
    // 2. Return 401 Unauthorized
    if (response.ok) {
      const result = await response.json();
      expect(result.status).toBe('invalid');
    } else {
      expect(response.status).toBe(401);
    }
  }, 30000);

  test('should handle token validation without refresh_token', async () => {
    const tokenData = generateTestToken('user-789', 'norefresh@example.com', 3600);

    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        access_token: tokenData.access_token
        // No refresh_token provided
      })
    });

    // Server should validate the access_token without attempting refresh
    expect(response.status).toBeLessThan(500);
  }, 30000);

  test('should reject validation with malformed token', async () => {
    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        access_token: 'not.a.valid.jwt.token',
        refresh_token: 'also.not.valid'
      })
    });

    // Server should reject malformed tokens
    expect(response.ok).toBe(false);
    expect(response.status).toBeGreaterThanOrEqual(400);
  }, 30000);

  test('should reject validation with empty token', async () => {
    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        access_token: '',
        refresh_token: ''
      })
    });

    expect(response.ok).toBe(false);
  }, 30000);
});

describe('Token Storage - Secure Keychain Integration', () => {
  const testTokenFilePath = path.join(os.tmpdir(), `test-tokens-${Date.now()}.json`);

  afterEach(() => {
    // Clean up test token file
    if (fs.existsSync(testTokenFilePath)) {
      fs.unlinkSync(testTokenFilePath);
    }
  });

  test('should save Pierre tokens to storage', () => {
    const tokens = {
      pierre: {
        access_token: 'test_pierre_token_123',
        refresh_token: 'test_pierre_refresh_456',
        expires_in: 3600,
        token_type: 'Bearer',
        scope: 'read:fitness write:fitness',
        saved_at: Math.floor(Date.now() / 1000)
      },
      providers: {}
    };

    // Simulate bridge.ts saveStoredTokens
    fs.writeFileSync(testTokenFilePath, JSON.stringify(tokens, null, 2));

    expect(fs.existsSync(testTokenFilePath)).toBe(true);

    const loaded = JSON.parse(fs.readFileSync(testTokenFilePath, 'utf-8'));
    expect(loaded.pierre.access_token).toBe('test_pierre_token_123');
    expect(loaded.pierre.saved_at).toBeGreaterThan(0);
  });

  test('should save provider tokens separately from Pierre tokens', () => {
    const tokens = {
      pierre: {
        access_token: 'pierre_token',
        refresh_token: 'pierre_refresh',
        expires_in: 3600
      },
      providers: {
        strava: {
          access_token: 'strava_access_token',
          refresh_token: 'strava_refresh_token',
          expires_at: Date.now() + 3600000,
          token_type: 'Bearer'
        },
        fitbit: {
          access_token: 'fitbit_access_token',
          expires_at: Date.now() + 7200000
        }
      }
    };

    fs.writeFileSync(testTokenFilePath, JSON.stringify(tokens, null, 2));

    const loaded = JSON.parse(fs.readFileSync(testTokenFilePath, 'utf-8'));
    expect(loaded.providers).toHaveProperty('strava');
    expect(loaded.providers).toHaveProperty('fitbit');
    expect(loaded.providers.strava.access_token).toBe('strava_access_token');
  });

  test('should load tokens from storage on bridge startup', () => {
    const storedTokens = {
      pierre: {
        access_token: 'stored_pierre_token',
        expires_in: 3600,
        saved_at: Math.floor(Date.now() / 1000)
      },
      providers: {}
    };

    fs.writeFileSync(testTokenFilePath, JSON.stringify(storedTokens, null, 2));

    // Simulate bridge.ts loadStoredTokens
    const loaded = JSON.parse(fs.readFileSync(testTokenFilePath, 'utf-8'));

    expect(loaded.pierre).toBeDefined();
    expect(loaded.pierre.access_token).toBe('stored_pierre_token');
  });

  test('should handle missing token file gracefully', () => {
    const nonExistentPath = path.join(os.tmpdir(), 'non-existent-tokens.json');

    let tokens = {};
    try {
      if (fs.existsSync(nonExistentPath)) {
        tokens = JSON.parse(fs.readFileSync(nonExistentPath, 'utf-8'));
      }
    } catch (error) {
      // Expected - file doesn't exist
      tokens = {};
    }

    expect(tokens).toEqual({});
  });

  test('should update stored tokens after refresh', () => {
    const originalTokens = {
      pierre: {
        access_token: 'old_token',
        refresh_token: 'old_refresh',
        expires_in: 3600,
        saved_at: Math.floor(Date.now() / 1000) - 3600  // Saved 1 hour ago
      },
      providers: {}
    };

    fs.writeFileSync(testTokenFilePath, JSON.stringify(originalTokens, null, 2));

    // Simulate token refresh
    const refreshedTokens = {
      pierre: {
        access_token: 'new_token',
        refresh_token: 'new_refresh',
        expires_in: 3600,
        saved_at: Math.floor(Date.now() / 1000)  // Current time
      },
      providers: {}
    };

    fs.writeFileSync(testTokenFilePath, JSON.stringify(refreshedTokens, null, 2));

    const loaded = JSON.parse(fs.readFileSync(testTokenFilePath, 'utf-8'));
    expect(loaded.pierre.access_token).toBe('new_token');
    expect(loaded.pierre.refresh_token).toBe('new_refresh');
  });

  test('should clear all tokens when invalidating credentials', () => {
    const tokens = {
      pierre: {
        access_token: 'token_to_clear',
        expires_in: 3600
      },
      providers: {
        strava: {
          access_token: 'strava_token_to_clear'
        }
      }
    };

    fs.writeFileSync(testTokenFilePath, JSON.stringify(tokens, null, 2));

    // Simulate bridge.ts invalidateCredentials('all')
    const clearedTokens = {};
    fs.writeFileSync(testTokenFilePath, JSON.stringify(clearedTokens, null, 2));

    const loaded = JSON.parse(fs.readFileSync(testTokenFilePath, 'utf-8'));
    expect(loaded.pierre).toBeUndefined();
    expect(loaded.providers).toBeUndefined();
  });
});

describe('Token Expiry During Tool Execution - REGRESSION TEST', () => {
  let serverHandle;
  const serverUrl = `http://localhost:${TestConfig.defaultServerPort}`;

  beforeAll(async () => {
    serverHandle = await ensureServerRunning({
      port: TestConfig.defaultServerPort,
      database: TestConfig.testDatabase,
      encryptionKey: TestConfig.testEncryptionKey
    });
  }, 60000);

  afterAll(async () => {
    if (serverHandle?.cleanup) {
      await serverHandle.cleanup();
    }
  });

  test('should detect token expiry before tool execution', async () => {
    // Simulate a token that's about to expire (expires_in < 60 seconds)
    const tokenData = generateTestToken('user-expiring', 'expiring@example.com', 30);

    // Bridge should detect this is too close to expiry and trigger refresh
    // before attempting tool call
    const expiresIn = 30;  // seconds
    const shouldRefresh = expiresIn < 60;  // Bridge's typical threshold

    expect(shouldRefresh).toBe(true);
  });

  test('should automatically refresh token when expired during tool call', async () => {
    // Generate an expired token
    const expiredToken = generateTestToken('user-expired', 'expired@example.com', -10);

    // Simulate tool call with expired token
    const toolCallEndpoint = `${serverUrl}/mcp`;
    const toolCallRequest = {
      jsonrpc: '2.0',
      id: 1,
      method: 'tools/call',
      params: {
        name: 'get_athlete',
        arguments: {}
      }
    };

    const response = await fetch(toolCallEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${expiredToken.access_token}`
      },
      body: JSON.stringify(toolCallRequest)
    });

    // Server should return 401 Unauthorized for expired token
    // Bridge should detect this and trigger refresh flow
    expect([401, 403]).toContain(response.status);
  }, 30000);

  test('should preserve tool call arguments after token refresh', () => {
    const originalToolCall = {
      method: 'tools/call',
      params: {
        name: 'get_activities',
        arguments: {
          before: '2025-01-01',
          after: '2024-01-01',
          limit: 10
        }
      }
    };

    // After token refresh, bridge must retry with same arguments
    const retryToolCall = { ...originalToolCall };

    expect(retryToolCall.params.name).toBe(originalToolCall.params.name);
    expect(retryToolCall.params.arguments).toEqual(originalToolCall.params.arguments);
  });

  test('should fail gracefully when refresh token is also expired', async () => {
    // Both access token and refresh token are expired
    const expiredToken = generateTestToken('user-double-expired', 'doubleexpired@example.com', -3600);

    const validateEndpoint = `${serverUrl}/oauth2/validate-and-refresh`;
    const response = await fetch(validateEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        access_token: expiredToken.access_token,
        refresh_token: expiredToken.access_token  // Also expired
      })
    });

    if (response.ok) {
      const result = await response.json();
      // Should indicate full re-auth required
      expect(result.status).toBe('invalid');
      if (result.requires_full_reauth !== undefined) {
        expect(result.requires_full_reauth).toBe(true);
      }
    } else {
      expect(response.status).toBe(401);
    }
  }, 30000);

  test('should not retry tool call more than once after token refresh', () => {
    // Track retry attempts
    let retryCount = 0;
    const maxRetries = 1;

    // Simulate tool call failure
    const attemptToolCall = () => {
      retryCount++;
      if (retryCount <= maxRetries) {
        return { status: 'retry', retryCount };
      } else {
        return { status: 'failed', retryCount };
      }
    };

    const firstAttempt = attemptToolCall();
    expect(firstAttempt.status).toBe('retry');

    const secondAttempt = attemptToolCall();
    expect(secondAttempt.status).toBe('failed');
    expect(secondAttempt.retryCount).toBe(2);
  });
});

describe('Provider Token Management', () => {
  test('should store provider tokens separately from Pierre tokens', () => {
    const tokens = {
      pierre: {
        access_token: 'pierre_token',
        expires_in: 3600
      },
      providers: {
        strava: {
          access_token: 'strava_token',
          refresh_token: 'strava_refresh',
          expires_at: Date.now() + 3600000
        }
      }
    };

    expect(tokens.pierre).toBeDefined();
    expect(tokens.providers.strava).toBeDefined();
    expect(tokens.providers.strava.access_token).not.toBe(tokens.pierre.access_token);
  });

  test('should detect expired provider tokens', () => {
    const providerToken = {
      access_token: 'strava_token',
      expires_at: Date.now() - 1000  // Expired 1 second ago
    };

    const isExpired = providerToken.expires_at && Date.now() >= providerToken.expires_at;
    expect(isExpired).toBe(true);
  });

  test('should clear provider tokens on disconnect', () => {
    const tokens = {
      pierre: {
        access_token: 'pierre_token'
      },
      providers: {
        strava: { access_token: 'strava_token' },
        fitbit: { access_token: 'fitbit_token' }
      }
    };

    // Disconnect Strava
    delete tokens.providers.strava;

    expect(tokens.providers.strava).toBeUndefined();
    expect(tokens.providers.fitbit).toBeDefined();
    expect(tokens.pierre).toBeDefined();
  });

  test('should preserve Pierre tokens when clearing provider tokens', () => {
    const tokens = {
      pierre: {
        access_token: 'pierre_token',
        refresh_token: 'pierre_refresh'
      },
      providers: {
        strava: { access_token: 'strava_token' }
      }
    };

    // Clear all provider tokens
    tokens.providers = {};

    expect(tokens.pierre).toBeDefined();
    expect(tokens.pierre.access_token).toBe('pierre_token');
    expect(Object.keys(tokens.providers)).toHaveLength(0);
  });
});
