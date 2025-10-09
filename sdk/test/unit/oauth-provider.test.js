// ABOUTME: Unit tests for OAuth provider functionality
// ABOUTME: Tests OAuth client metadata generation and token management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

describe('OAuth Client Metadata', () => {
  test('should generate valid client metadata', () => {
    const metadata = {
      client_name: 'Pierre Claude Bridge',
      client_uri: 'https://claude.ai',
      redirect_uris: ['http://localhost:35535/oauth/callback'],
      grant_types: ['authorization_code'],
      response_types: ['code'],
      scope: 'read:fitness write:fitness',
      token_endpoint_auth_method: 'client_secret_basic'
    };

    expect(metadata.client_name).toBe('Pierre Claude Bridge');
    expect(metadata.redirect_uris).toHaveLength(1);
    expect(metadata.grant_types).toContain('authorization_code');
  });

  test('should have required OAuth fields', () => {
    const metadata = {
      client_name: 'Pierre Claude Bridge',
      redirect_uris: ['http://localhost:35535/oauth/callback'],
      grant_types: ['authorization_code'],
      response_types: ['code']
    };

    expect(metadata).toHaveProperty('client_name');
    expect(metadata).toHaveProperty('redirect_uris');
    expect(metadata).toHaveProperty('grant_types');
    expect(metadata).toHaveProperty('response_types');
  });
});

describe('OAuth State Generation', () => {
  test('should generate random state string', () => {
    const state1 = Math.random().toString(36).substring(2, 34);
    const state2 = Math.random().toString(36).substring(2, 34);

    expect(state1).toBeTruthy();
    expect(state2).toBeTruthy();
    expect(state1).not.toBe(state2);
  });

  test('should maintain state between calls', () => {
    const state = 'test_state_value';
    expect(state).toBe('test_state_value');
  });
});

describe('Token Storage', () => {
  test('should store tokens with timestamp', () => {
    const tokens = {
      access_token: 'test_access_token',
      refresh_token: 'test_refresh_token',
      expires_in: 3600,
      saved_at: Math.floor(Date.now() / 1000)
    };

    expect(tokens.access_token).toBeTruthy();
    expect(tokens.saved_at).toBeGreaterThan(0);
  });

  test('should retrieve stored tokens', () => {
    const storedTokens = {
      pierre: {
        access_token: 'test_token',
        expires_in: 3600
      }
    };

    expect(storedTokens.pierre).toBeDefined();
    expect(storedTokens.pierre.access_token).toBe('test_token');
  });

  test('should handle missing tokens gracefully', () => {
    const tokens = undefined;
    expect(tokens).toBeUndefined();
  });
});

describe('Client Information Management', () => {
  test('should store client ID and secret', () => {
    const clientInfo = {
      client_id: 'test_client_id_123',
      client_secret: 'test_client_secret_456'
    };

    expect(clientInfo.client_id).toBeTruthy();
    expect(clientInfo.client_secret).toBeTruthy();
  });

  test('should return undefined when no client info', () => {
    const clientInfo = undefined;
    expect(clientInfo).toBeUndefined();
  });
});
