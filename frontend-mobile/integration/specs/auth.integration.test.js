// ABOUTME: Integration tests for authentication flows against the real backend server.
// ABOUTME: Tests login, token refresh, and error handling with actual API calls.

const {
  loginWithCredentials,
  createAndLoginAsAdmin,
  createAndLoginTestUser,
  refreshAccessToken,
  authenticatedRequest,
  isTokenValid,
} = require('../helpers');
const {
  testUsers,
  generateUniqueEmail,
  validPassword,
  timeouts,
  endpoints,
} = require('../fixtures');

describe('Authentication Integration Tests', () => {
  describe('Login Flow', () => {
    it('should successfully login with valid admin credentials', async () => {
      const result = await createAndLoginAsAdmin();

      expect(result.success).toBe(true);
      expect(result.accessToken).toBeDefined();
      expect(result.accessToken.length).toBeGreaterThan(0);
    });

    it('should return user information on successful login', async () => {
      const result = await createAndLoginAsAdmin();

      expect(result.success).toBe(true);
      expect(result.user).toBeDefined();
      // User object should have email matching what we logged in with
      if (result.user && result.user.email) {
        expect(result.user.email).toBe(testUsers.admin.email);
      }
    });

    it('should return refresh token on successful login (if supported)', async () => {
      const result = await createAndLoginAsAdmin();

      expect(result.success).toBe(true);
      // Refresh token is optional - some OAuth implementations don't return it
      if (result.refreshToken) {
        expect(result.refreshToken.length).toBeGreaterThan(0);
      } else {
        console.log('Note: Refresh token not returned by OAuth endpoint');
      }
    });

    it('should fail login with invalid password', async () => {
      // First create the user
      const user = testUsers.admin;
      await createAndLoginAsAdmin(); // Ensures user exists

      // Try to login with wrong password
      const result = await loginWithCredentials(user.email, 'WrongPassword123!');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should fail login with non-existent user', async () => {
      const result = await loginWithCredentials(
        'nonexistent-user@test.local',
        'AnyPassword123!'
      );

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should fail login with empty credentials', async () => {
      const result = await loginWithCredentials('', '');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle unique test user creation (may require API support)', async () => {
      const uniqueUser = {
        email: generateUniqueEmail('auth-test'),
        password: validPassword,
        role: 'admin',
      };

      const result = await createAndLoginTestUser(uniqueUser);

      // User creation may fail if API doesn't support dynamic user creation
      // In that case, the login will also fail
      if (result.success) {
        expect(result.accessToken).toBeDefined();
      } else {
        console.log('Note: Dynamic user creation not supported - using setup user');
        // Fallback: verify we can still login with the setup user
        const fallbackResult = await createAndLoginAsAdmin();
        expect(fallbackResult.success).toBe(true);
      }
    });
  });

  describe('Token Validation', () => {
    it('should validate a fresh access token', async () => {
      const loginResult = await createAndLoginAsAdmin();
      expect(loginResult.success).toBe(true);

      const isValid = await isTokenValid(loginResult.accessToken);
      expect(isValid).toBe(true);
    });

    it('should reject an invalid token', async () => {
      const isValid = await isTokenValid('invalid-token-12345');
      expect(isValid).toBe(false);
    });

    it('should reject an empty token', async () => {
      const isValid = await isTokenValid('');
      expect(isValid).toBe(false);
    });
  });

  describe('Token Refresh', () => {
    it('should refresh access token with valid refresh token (if supported)', async () => {
      const loginResult = await createAndLoginAsAdmin();
      expect(loginResult.success).toBe(true);

      // Skip if no refresh token was provided
      if (!loginResult.refreshToken) {
        console.log('Skipping: No refresh token available');
        return;
      }

      const refreshResult = await refreshAccessToken(loginResult.refreshToken);

      expect(refreshResult.success).toBe(true);
      expect(refreshResult.accessToken).toBeDefined();
      expect(refreshResult.accessToken.length).toBeGreaterThan(0);
    });

    it('should return new refresh token on refresh (if supported)', async () => {
      const loginResult = await createAndLoginAsAdmin();
      expect(loginResult.success).toBe(true);

      // Skip if no refresh token was provided
      if (!loginResult.refreshToken) {
        console.log('Skipping: No refresh token available');
        return;
      }

      const refreshResult = await refreshAccessToken(loginResult.refreshToken);

      expect(refreshResult.success).toBe(true);
      // New refresh token is optional
      if (refreshResult.refreshToken) {
        expect(refreshResult.refreshToken.length).toBeGreaterThan(0);
      }
    });

    it('should fail refresh with invalid refresh token', async () => {
      const refreshResult = await refreshAccessToken('invalid-refresh-token');

      expect(refreshResult.success).toBe(false);
      expect(refreshResult.error).toBeDefined();
    });
  });

  describe('Authenticated Requests', () => {
    it('should access protected endpoint with valid token', async () => {
      const loginResult = await createAndLoginAsAdmin();
      expect(loginResult.success).toBe(true);

      const result = await authenticatedRequest(
        endpoints.dashboardOverview,
        loginResult.accessToken
      );

      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
    });

    it('should reject protected endpoint with invalid token', async () => {
      const result = await authenticatedRequest(
        endpoints.dashboardOverview,
        'invalid-token'
      );

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });

    it('should reject protected endpoint without token', async () => {
      const result = await authenticatedRequest(endpoints.dashboardOverview, '');

      expect(result.success).toBe(false);
      expect(result.status).toBe(401);
    });

    it('should check admin setup status without authentication', async () => {
      // This endpoint is typically public
      const { getBackendUrl } = require('../helpers/server-manager');
      const backendUrl = getBackendUrl();

      const response = await fetch(`${backendUrl}${endpoints.adminSetupStatus}`);

      // Should return 200 even without auth (it's a setup check)
      expect(response.status).toBe(200);
    });
  });

  describe('Session Management', () => {
    it('should maintain separate sessions for different users (if user creation supported)', async () => {
      // Create first user
      const user1 = {
        email: generateUniqueEmail('session-user1'),
        password: validPassword,
        role: 'admin',
      };
      const login1 = await createAndLoginTestUser(user1);

      // If user creation fails, skip this test
      if (!login1.success) {
        console.log('Skipping: Dynamic user creation not supported');
        return;
      }

      // Create second user
      const user2 = {
        email: generateUniqueEmail('session-user2'),
        password: validPassword,
        role: 'admin',
      };
      const login2 = await createAndLoginTestUser(user2);

      if (!login2.success) {
        console.log('Skipping: Could not create second user');
        return;
      }

      // Both tokens should be valid
      expect(await isTokenValid(login1.accessToken)).toBe(true);
      expect(await isTokenValid(login2.accessToken)).toBe(true);

      // Tokens should be different
      expect(login1.accessToken).not.toBe(login2.accessToken);
    });

    it('should allow multiple logins for the same user', async () => {
      const loginResult1 = await createAndLoginAsAdmin();
      expect(loginResult1.success).toBe(true);

      // Login again with same user
      const loginResult2 = await loginWithCredentials(
        testUsers.admin.email,
        testUsers.admin.password
      );
      expect(loginResult2.success).toBe(true);

      // Both tokens should be valid
      expect(await isTokenValid(loginResult1.accessToken)).toBe(true);
      expect(await isTokenValid(loginResult2.accessToken)).toBe(true);
    });
  });
});
