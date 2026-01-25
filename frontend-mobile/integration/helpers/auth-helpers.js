// ABOUTME: Authentication helper functions for mobile integration tests.
// ABOUTME: Provides real login flows that interact with the actual backend server via API.

const { getBackendUrl } = require('./server-manager');
const { createTestAdminUser } = require('./db-setup');
const { testUsers } = require('../fixtures/test-data');

/**
 * Perform a real login through the OAuth token endpoint.
 * This makes actual API calls to the backend server.
 *
 * @param {string} email - User email
 * @param {string} password - User password
 * @returns {Promise<{success: boolean, accessToken?: string, refreshToken?: string, user?: object, error?: string}>}
 */
async function loginWithCredentials(email, password) {
  try {
    const backendUrl = getBackendUrl();
    console.log(`[Auth] Attempting login for ${email}`);

    const response = await fetch(`${backendUrl}/oauth/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        Accept: 'application/json',
      },
      body: new URLSearchParams({
        grant_type: 'password',
        username: email,
        password: password,
      }).toString(),
    });

    const data = await response.json();

    if (!response.ok) {
      console.log(`[Auth] Login failed: ${data.error || response.statusText}`);
      return {
        success: false,
        error: data.error_description || data.error || 'Login failed',
      };
    }

    console.log(`[Auth] Login successful for ${email}`);
    return {
      success: true,
      accessToken: data.access_token,
      refreshToken: data.refresh_token,
      user: data.user,
    };
  } catch (error) {
    console.log(`[Auth] Login error: ${error.message}`);
    return {
      success: false,
      error: `Network error: ${error.message}`,
    };
  }
}

/**
 * Create a test admin user in the database and then log in.
 * This is the primary way to set up an authenticated session for tests.
 *
 * @returns {Promise<{success: boolean, accessToken?: string, refreshToken?: string, user?: object, error?: string}>}
 */
async function createAndLoginAsAdmin() {
  const user = testUsers.admin;
  console.log(`[Auth] Creating admin user: ${user.email}`);

  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    console.log(`[Auth] Failed to create admin user: ${createResult.error}`);
    return { success: false, error: createResult.error };
  }
  console.log(`[Auth] Admin user created, proceeding to login`);

  return loginWithCredentials(user.email, user.password);
}

/**
 * Create a test super admin user and log in.
 *
 * @returns {Promise<{success: boolean, accessToken?: string, refreshToken?: string, user?: object, error?: string}>}
 */
async function createAndLoginAsSuperAdmin() {
  const user = testUsers.superAdmin;

  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    return { success: false, error: createResult.error };
  }

  return loginWithCredentials(user.email, user.password);
}

/**
 * Create a custom test user and log in.
 *
 * @param {{email: string, password: string, role?: string}} user - User to create
 * @returns {Promise<{success: boolean, accessToken?: string, refreshToken?: string, user?: object, error?: string}>}
 */
async function createAndLoginTestUser(user) {
  const createResult = await createTestAdminUser(user);
  if (!createResult.success) {
    return { success: false, error: createResult.error };
  }

  return loginWithCredentials(user.email, user.password);
}

/**
 * Refresh an access token using a refresh token.
 *
 * @param {string} refreshToken - The refresh token
 * @returns {Promise<{success: boolean, accessToken?: string, refreshToken?: string, error?: string}>}
 */
async function refreshAccessToken(refreshToken) {
  try {
    const backendUrl = getBackendUrl();

    const response = await fetch(`${backendUrl}/oauth/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        Accept: 'application/json',
      },
      body: new URLSearchParams({
        grant_type: 'refresh_token',
        refresh_token: refreshToken,
      }).toString(),
    });

    const data = await response.json();

    if (!response.ok) {
      return {
        success: false,
        error: data.error_description || data.error || 'Token refresh failed',
      };
    }

    return {
      success: true,
      accessToken: data.access_token,
      refreshToken: data.refresh_token,
    };
  } catch (error) {
    return {
      success: false,
      error: `Network error: ${error.message}`,
    };
  }
}

/**
 * Make an authenticated API request.
 *
 * @param {string} endpoint - API endpoint (without base URL)
 * @param {string} accessToken - JWT access token
 * @param {object} options - Additional fetch options
 * @returns {Promise<{success: boolean, data?: any, status?: number, error?: string}>}
 */
async function authenticatedRequest(endpoint, accessToken, options = {}) {
  try {
    const backendUrl = getBackendUrl();
    const url = endpoint.startsWith('http')
      ? endpoint
      : `${backendUrl}${endpoint}`;

    const response = await fetch(url, {
      ...options,
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
        Authorization: `Bearer ${accessToken}`,
        ...options.headers,
      },
    });

    const data = await response.json().catch(() => null);

    if (!response.ok) {
      return {
        success: false,
        status: response.status,
        error: data?.error || data?.message || response.statusText,
      };
    }

    return {
      success: true,
      status: response.status,
      data,
    };
  } catch (error) {
    return {
      success: false,
      error: `Network error: ${error.message}`,
    };
  }
}

/**
 * Check if a token is valid by making a request to a protected endpoint.
 *
 * @param {string} accessToken - JWT access token to verify
 * @returns {Promise<boolean>}
 */
async function isTokenValid(accessToken) {
  const result = await authenticatedRequest(
    '/api/dashboard/overview',
    accessToken
  );
  return result.success || result.status !== 401;
}

module.exports = {
  loginWithCredentials,
  createAndLoginAsAdmin,
  createAndLoginAsSuperAdmin,
  createAndLoginTestUser,
  refreshAccessToken,
  authenticatedRequest,
  isTokenValid,
};
