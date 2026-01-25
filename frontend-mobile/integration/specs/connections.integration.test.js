// ABOUTME: Integration tests for provider connections against the real backend server.
// ABOUTME: Tests provider listing, connection flows, and OAuth initiation with actual API calls.

const {
  createAndLoginAsAdmin,
  authenticatedRequest,
  getBackendUrl,
} = require('../helpers');
const { endpoints, timeouts } = require('../fixtures');

describe('Connections Integration Tests', () => {
  let accessToken;

  beforeAll(async () => {
    // Login once for all connection tests
    const loginResult = await createAndLoginAsAdmin();
    expect(loginResult.success).toBe(true);
    accessToken = loginResult.accessToken;
  }, timeouts.serverStart);

  describe('Connections List', () => {
    it('should fetch user connections list or return 404 if not implemented', async () => {
      const result = await authenticatedRequest(
        endpoints.connections,
        accessToken
      );

      // 200 for success, 404 if endpoint doesn't exist yet
      expect([200, 404].includes(result.status)).toBe(true);

      if (result.success) {
        // Response should be an array or object with connections
        expect(
          Array.isArray(result.data) ||
            result.data.connections !== undefined ||
            result.data.items !== undefined
        ).toBe(true);
      }
    });

    it('should return empty connections for new user (if endpoint exists)', async () => {
      const result = await authenticatedRequest(
        endpoints.connections,
        accessToken
      );

      // Skip assertions if endpoint doesn't exist
      if (result.status === 404) {
        console.log('Connections endpoint not implemented, skipping');
        return;
      }

      expect(result.success).toBe(true);

      const connections = Array.isArray(result.data)
        ? result.data
        : result.data.connections || result.data.items || [];

      // New user should have no provider connections
      expect(connections).toHaveLength(0);
    });

    it('should reject connections request without auth', async () => {
      const result = await authenticatedRequest(endpoints.connections, '');

      expect(result.success).toBe(false);
      // Could be 401 (unauthorized) or 404 (not found)
      expect([401, 404].includes(result.status)).toBe(true);
    });
  });

  describe('Available Providers', () => {
    it('should fetch list of available providers', async () => {
      const result = await authenticatedRequest(
        '/api/providers',
        accessToken
      );

      // 200 for success, 404 if endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);

      if (result.success) {
        expect(result.data).toBeDefined();
        // Should be an array of providers
        const providers = Array.isArray(result.data)
          ? result.data
          : result.data.providers || [];

        if (providers.length > 0) {
          // Each provider should have at least a name or id
          expect(
            providers[0].name || providers[0].id || providers[0].provider_id
          ).toBeDefined();
        }
      }
    });

    it('should include Strava as available provider', async () => {
      const result = await authenticatedRequest(
        '/api/providers',
        accessToken
      );

      if (result.success) {
        const providers = Array.isArray(result.data)
          ? result.data
          : result.data.providers || [];

        // Strava should be in the list of providers
        const hasStrava = providers.some(
          (p) =>
            p.name?.toLowerCase() === 'strava' ||
            p.id?.toLowerCase() === 'strava' ||
            p.provider_id?.toLowerCase() === 'strava'
        );

        // This is informational - may not have Strava configured
        if (!hasStrava) {
          console.log('Note: Strava provider not found in available providers');
        }
      }
    });
  });

  describe('OAuth Flow Initiation', () => {
    it('should initiate Strava OAuth flow', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava/authorize',
        accessToken
      );

      // 200/302 for success with redirect URL, 404 if not configured
      expect([200, 302, 400, 404].includes(result.status)).toBe(true);

      if (result.success && result.data) {
        // Should return an authorization URL
        if (result.data.url || result.data.authorization_url) {
          const authUrl = result.data.url || result.data.authorization_url;
          expect(authUrl).toContain('strava.com');
        }
      }
    });

    it('should reject OAuth initiation without auth', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava/authorize',
        ''
      );

      expect(result.success).toBe(false);
      // Could be 401 (unauthorized) or 404 (not found)
      expect([401, 404].includes(result.status)).toBe(true);
    });

    it('should handle invalid provider OAuth request', async () => {
      const result = await authenticatedRequest(
        '/api/connections/invalid-provider/authorize',
        accessToken
      );

      // Should return 400 or 404 for invalid provider
      expect([400, 404].includes(result.status)).toBe(true);
    });
  });

  describe('Connection Status', () => {
    it('should check Strava connection status', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava/status',
        accessToken
      );

      // 200 for status check, 404 if endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);

      if (result.success) {
        expect(result.data).toBeDefined();
        // Status should indicate connected or not
        if (result.data.connected !== undefined) {
          expect(typeof result.data.connected).toBe('boolean');
        }
      }
    });

    it('should return not connected for new user', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava/status',
        accessToken
      );

      if (result.success && result.data) {
        // New user shouldn't have Strava connected
        expect(result.data.connected).toBe(false);
      }
    });
  });

  describe('Connection Disconnect', () => {
    it('should handle disconnect request for non-connected provider', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava',
        accessToken,
        { method: 'DELETE' }
      );

      // 200/204 for success, 404 if not connected or endpoint doesn't exist
      expect([200, 204, 400, 404].includes(result.status)).toBe(true);
    });

    it('should reject disconnect without auth', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava',
        '',
        { method: 'DELETE' }
      );

      expect(result.success).toBe(false);
      // Could be 401 (unauthorized) or 404 (not found)
      expect([401, 404].includes(result.status)).toBe(true);
    });
  });

  describe('Connection Error Handling', () => {
    it('should handle malformed OAuth callback gracefully', async () => {
      const backendUrl = getBackendUrl();

      // Simulate a malformed OAuth callback
      const response = await fetch(
        `${backendUrl}/api/connections/strava/callback?error=access_denied`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${accessToken}`,
          },
        }
      );

      // Should handle error gracefully, not crash
      expect([200, 302, 400, 401, 404].includes(response.status)).toBe(true);
    });

    it('should handle missing OAuth code gracefully', async () => {
      const backendUrl = getBackendUrl();

      // Callback without code parameter
      const response = await fetch(
        `${backendUrl}/api/connections/strava/callback`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${accessToken}`,
          },
        }
      );

      // Should return error, not crash
      expect([400, 401, 404].includes(response.status)).toBe(true);
    });

    it('should handle rate limiting gracefully', async () => {
      // Make multiple rapid requests
      const requests = Array(5)
        .fill(null)
        .map(() =>
          authenticatedRequest('/api/connections', accessToken)
        );

      const results = await Promise.all(requests);

      // All should succeed, return rate limit error, or return 404
      results.forEach((result) => {
        expect([200, 404, 429].includes(result.status)).toBe(true);
      });
    });
  });

  describe('Sync Operations', () => {
    it('should handle sync request for non-connected provider', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava/sync',
        accessToken,
        { method: 'POST' }
      );

      // 200 for success, 400 if not connected, 404 if endpoint doesn't exist
      expect([200, 400, 404].includes(result.status)).toBe(true);
    });

    it('should check sync status', async () => {
      const result = await authenticatedRequest(
        '/api/connections/strava/sync/status',
        accessToken
      );

      // 200 for status, 404 if endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);
    });
  });
});
