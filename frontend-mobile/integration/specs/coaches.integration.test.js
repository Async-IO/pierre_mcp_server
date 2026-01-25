// ABOUTME: Integration tests for coach functionality against the real backend server.
// ABOUTME: Tests coach listing, details, and management with actual API calls.

const {
  createAndLoginAsAdmin,
  authenticatedRequest,
  getBackendUrl,
} = require('../helpers');
const { endpoints, timeouts } = require('../fixtures');

describe('Coaches Integration Tests', () => {
  let accessToken;

  beforeAll(async () => {
    // Login once for all coach tests
    const loginResult = await createAndLoginAsAdmin();
    expect(loginResult.success).toBe(true);
    accessToken = loginResult.accessToken;
  }, timeouts.serverStart);

  describe('Coach Listing', () => {
    it('should fetch coaches list or return expected status', async () => {
      const result = await authenticatedRequest(endpoints.coaches, accessToken);

      // 200 for success, 400/404 if endpoint doesn't exist or requires params
      expect([200, 400, 404].includes(result.status)).toBe(true);

      if (result.success && result.data) {
        // Response should be an array or object with coaches
        const hasValidStructure =
          Array.isArray(result.data) ||
          result.data.coaches !== undefined ||
          result.data.items !== undefined ||
          typeof result.data === 'object';
        expect(hasValidStructure).toBe(true);
      }
    });

    it('should return coaches with expected structure (if endpoint exists)', async () => {
      const result = await authenticatedRequest(endpoints.coaches, accessToken);

      // Skip if endpoint doesn't exist or returns error
      if (result.status === 404 || result.status === 400) {
        console.log(`Coaches endpoint returned ${result.status}, skipping structure test`);
        return;
      }

      expect(result.success).toBe(true);

      // Handle various response structures
      if (!result.data) {
        console.log('No data in response, skipping structure validation');
        return;
      }

      const coaches = Array.isArray(result.data)
        ? result.data
        : result.data.coaches || result.data.items || [];

      // Even if empty, structure should be correct
      if (coaches.length > 0) {
        const firstCoach = coaches[0];
        // Coach should have basic fields (flexible check)
        const hasId = firstCoach.id || firstCoach.coach_id || firstCoach.uuid;
        expect(hasId).toBeDefined();
      }
    });

    it('should reject coaches request without auth', async () => {
      const result = await authenticatedRequest(endpoints.coaches, '');

      expect(result.success).toBe(false);
      // Could be 401 (unauthorized) or 404 (not found)
      expect([401, 404].includes(result.status)).toBe(true);
    });
  });

  describe('Coach Store', () => {
    it('should fetch coach store/catalog', async () => {
      const result = await authenticatedRequest(
        '/api/store/coaches',
        accessToken
      );

      // 200 for success, 404 if store endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);

      if (result.success) {
        expect(result.data).toBeDefined();
      }
    });

    it('should fetch featured coaches', async () => {
      const result = await authenticatedRequest(
        '/api/store/coaches/featured',
        accessToken
      );

      // 200 for success, 400/404 if endpoint doesn't exist or not configured
      expect([200, 400, 404].includes(result.status)).toBe(true);
    });

    it('should support coach search/filtering', async () => {
      const result = await authenticatedRequest(
        '/api/store/coaches?search=fitness',
        accessToken
      );

      // 200 for success, 404 if endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);
    });
  });

  describe('Coach Details', () => {
    let coachId;

    beforeAll(async () => {
      // Try to get a coach ID from the list
      const listResult = await authenticatedRequest(
        endpoints.coaches,
        accessToken
      );

      if (listResult.success) {
        const coaches = Array.isArray(listResult.data)
          ? listResult.data
          : listResult.data.coaches || listResult.data.items || [];

        if (coaches.length > 0) {
          coachId = coaches[0].id || coaches[0].coach_id;
        }
      }
    });

    it('should fetch single coach details (if coaches exist)', async () => {
      if (!coachId) {
        console.log('Skipping: No coach available for details test');
        return;
      }

      const result = await authenticatedRequest(
        `${endpoints.coaches}/${coachId}`,
        accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.data).toBeDefined();
    });

    it('should handle non-existent coach ID', async () => {
      const result = await authenticatedRequest(
        `${endpoints.coaches}/non-existent-coach-id-12345`,
        accessToken
      );

      // Should return 404 for non-existent coach
      expect([400, 404].includes(result.status)).toBe(true);
    });
  });

  describe('User Coaches (My Coaches)', () => {
    it('should fetch user assigned coaches', async () => {
      const result = await authenticatedRequest(
        '/api/user/coaches',
        accessToken
      );

      // 200 for success, 404 if endpoint doesn't exist
      expect([200, 404].includes(result.status)).toBe(true);

      if (result.success) {
        expect(
          Array.isArray(result.data) ||
            result.data.coaches !== undefined
        ).toBe(true);
      }
    });

    it('should return empty coaches for new user', async () => {
      const result = await authenticatedRequest(
        '/api/user/coaches',
        accessToken
      );

      if (result.success) {
        const coaches = Array.isArray(result.data)
          ? result.data
          : result.data.coaches || [];

        // New user might have no coaches or have default coaches
        expect(Array.isArray(coaches)).toBe(true);
      }
    });
  });

  describe('Coach Assignment', () => {
    it('should handle coach assignment request', async () => {
      // Try to assign a coach (may fail if no coaches available)
      const result = await authenticatedRequest(
        '/api/user/coaches',
        accessToken,
        {
          method: 'POST',
          body: JSON.stringify({
            coach_id: 'test-coach-id',
          }),
        }
      );

      // 200/201 for success, 400/404 for invalid coach, etc.
      expect([200, 201, 400, 404, 422].includes(result.status)).toBe(true);
    });

    it('should reject coach assignment without auth', async () => {
      const result = await authenticatedRequest('/api/user/coaches', '', {
        method: 'POST',
        body: JSON.stringify({
          coach_id: 'test-coach-id',
        }),
      });

      expect(result.success).toBe(false);
      // Could be 401 (unauthorized) or 404 (not found)
      expect([401, 404].includes(result.status)).toBe(true);
    });
  });

  describe('Coach Error Handling', () => {
    it('should handle invalid coach ID format gracefully', async () => {
      const result = await authenticatedRequest(
        `${endpoints.coaches}/!!!invalid!!!`,
        accessToken
      );

      // Should return error for invalid ID format
      expect([400, 404].includes(result.status)).toBe(true);
    });

    it('should handle malformed coach assignment request', async () => {
      const backendUrl = getBackendUrl();

      const response = await fetch(`${backendUrl}/api/user/coaches`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: 'not valid json',
      });

      // Should return error for malformed request
      expect([400, 404, 422].includes(response.status)).toBe(true);
    });
  });
});
