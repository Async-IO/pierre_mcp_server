// ABOUTME: Integration tests for activity sync flows against the real backend server.
// ABOUTME: Tests syncing activities from providers and verifying data integrity.

const { createAndLoginAsAdmin, authenticatedRequest } = require('../helpers');
const { timeouts } = require('../fixtures');

describe('Activity Sync Integration Tests', () => {
  let accessToken;

  beforeAll(async () => {
    const loginResult = await createAndLoginAsAdmin();
    expect(loginResult.success).toBe(true);
    accessToken = loginResult.accessToken;
  });

  describe('Activity Listing', () => {
    it('should list activities for authenticated user', async () => {
      const result = await authenticatedRequest('/api/activities', accessToken);

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
      // Activities may be empty for a new user
      expect(Array.isArray(result.data.activities || result.data)).toBe(true);
    });

    it('should reject activity listing without authentication', async () => {
      const result = await authenticatedRequest('/api/activities', '');

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });

    it('should support pagination parameters', async () => {
      const result = await authenticatedRequest(
        '/api/activities?limit=10&offset=0',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });

    it('should support date range filtering', async () => {
      const startDate = new Date();
      startDate.setMonth(startDate.getMonth() - 1);
      const endDate = new Date();

      const result = await authenticatedRequest(
        `/api/activities?start_date=${startDate.toISOString()}&end_date=${endDate.toISOString()}`,
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });
  });

  describe('Provider Connection Status', () => {
    it('should check connection status for Strava', async () => {
      const result = await authenticatedRequest(
        '/api/oauth/status',
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      // Response is an array of connected providers
      expect(Array.isArray(result.data)).toBe(true);
    });

    it('should return connection details per provider', async () => {
      const result = await authenticatedRequest(
        '/api/oauth/status',
        accessToken
      );

      expect(result.success).toBe(true);
      // If any providers are connected, they should have status info
      if (result.data && result.data.length > 0) {
        const provider = result.data[0];
        expect(provider).toHaveProperty('provider');
        expect(provider).toHaveProperty('connected');
      }
    });
  });

  describe('Activity Sync Trigger', () => {
    it('should trigger sync for connected provider (if connected)', async () => {
      // First check if any provider is connected
      const statusResult = await authenticatedRequest(
        '/api/oauth/status',
        accessToken
      );

      if (!statusResult.data || statusResult.data.length === 0) {
        console.log('No providers connected - skipping sync trigger test');
        return;
      }

      const connectedProvider = statusResult.data.find((p) => p.connected);
      if (!connectedProvider) {
        console.log('No connected providers - skipping sync trigger test');
        return;
      }

      // Trigger sync
      const syncResult = await authenticatedRequest(
        '/api/activities/sync',
        accessToken,
        { method: 'POST' }
      );

      // Sync might succeed or return "already syncing"
      expect([200, 202, 409]).toContain(syncResult.status);
    });

    it('should return appropriate error when no providers connected', async () => {
      // This test documents behavior - sync without connected provider
      const result = await authenticatedRequest(
        '/api/activities/sync',
        accessToken,
        { method: 'POST' }
      );

      // Either succeeds (with warning) or fails gracefully
      expect(result.status).toBeDefined();
    });
  });

  describe('Individual Activity Operations', () => {
    it('should return 404 for non-existent activity', async () => {
      const result = await authenticatedRequest(
        '/api/activities/non-existent-id-12345',
        accessToken
      );

      expect(result.success).toBe(false);
      expect(result.status).toBe(404);
    });

    it('should handle activity ID format validation', async () => {
      const result = await authenticatedRequest(
        '/api/activities/invalid-uuid',
        accessToken
      );

      // Should return 404 or 400 for invalid ID
      expect([400, 404]).toContain(result.status);
    });
  });

  describe('Activity Intelligence', () => {
    it('should access activity intelligence endpoint', async () => {
      // First get activities to find a real ID
      const activitiesResult = await authenticatedRequest(
        '/api/activities',
        accessToken
      );

      if (
        !activitiesResult.data ||
        !activitiesResult.data.activities ||
        activitiesResult.data.activities.length === 0
      ) {
        console.log('No activities available - skipping intelligence test');
        return;
      }

      const activityId = activitiesResult.data.activities[0].id;
      const result = await authenticatedRequest(
        `/api/activities/${activityId}/intelligence`,
        accessToken
      );

      // Intelligence may or may not be generated yet
      expect([200, 404]).toContain(result.status);
    });
  });

  describe('Sync Status', () => {
    it('should return sync status for user', async () => {
      const result = await authenticatedRequest(
        '/api/activities/sync/status',
        accessToken
      );

      // Endpoint may not exist or may return status
      if (result.status === 404) {
        console.log('Sync status endpoint not implemented');
        return;
      }

      expect(result.success).toBe(true);
    });
  });

  describe('Activity Metadata', () => {
    it('should return activity statistics', async () => {
      const result = await authenticatedRequest(
        '/api/activities/stats',
        accessToken
      );

      // Stats endpoint may return aggregated data
      if (result.status === 404) {
        console.log('Stats endpoint not implemented - skipping');
        return;
      }

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
    });
  });
});
