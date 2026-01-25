// ABOUTME: Test data fixtures for mobile integration tests.
// ABOUTME: Provides consistent test users, API keys, and other test data.

/**
 * Pre-defined test users for integration tests.
 * These users are created via the admin-setup binary.
 */
const testUsers = {
  admin: {
    email: 'mobile-integration-admin@test.local',
    password: 'MobileIntegrationPass123!',
    role: 'admin',
  },
  superAdmin: {
    email: 'mobile-integration-super@test.local',
    password: 'SuperMobilePass456!',
    role: 'super_admin',
  },
  regularUser: {
    email: 'mobile-integration-user@test.local',
    password: 'RegularMobilePass789!',
    role: 'user',
  },
};

/**
 * Test API key configurations.
 */
const testApiKeys = {
  readOnly: {
    name: 'Mobile Integration Test Read-Only Key',
    scopes: ['read'],
  },
  readWrite: {
    name: 'Mobile Integration Test Read-Write Key',
    scopes: ['read', 'write'],
  },
  fullAccess: {
    name: 'Mobile Integration Test Full Access Key',
    scopes: ['read', 'write', 'admin'],
  },
};

/**
 * Generate a unique email for test isolation.
 *
 * @param {string} prefix - Email prefix
 * @returns {string}
 */
function generateUniqueEmail(prefix = 'test') {
  const timestamp = Date.now();
  const random = Math.random().toString(36).substring(2, 8);
  return `${prefix}-${timestamp}-${random}@test.local`;
}

/**
 * Generate a unique API key name for test isolation.
 *
 * @param {string} prefix - Key name prefix
 * @returns {string}
 */
function generateUniqueKeyName(prefix = 'Test Key') {
  const timestamp = Date.now();
  return `${prefix} ${timestamp}`;
}

/**
 * Valid password that meets typical requirements.
 */
const validPassword = 'ValidMobileTestPass123!';

/**
 * Invalid passwords for negative testing.
 */
const invalidPasswords = {
  tooShort: 'short',
  noUppercase: 'lowercaseonly123!',
  noLowercase: 'UPPERCASEONLY123!',
  noNumbers: 'NoNumbersHere!',
  noSpecial: 'NoSpecialChars123',
};

/**
 * Common test timeouts (in milliseconds).
 */
const timeouts = {
  short: 5000,
  medium: 10000,
  long: 30000,
  serverStart: 60000,
  apiCall: 15000,
};

/**
 * API endpoints used in tests.
 */
const endpoints = {
  health: '/health',
  oauthToken: '/oauth/token',
  dashboardOverview: '/api/dashboard/overview',
  chatConversations: '/api/chat/conversations',
  coaches: '/api/coaches',
  connections: '/api/connections',
  adminSetupStatus: '/admin/setup/status',
};

module.exports = {
  testUsers,
  testApiKeys,
  generateUniqueEmail,
  generateUniqueKeyName,
  validPassword,
  invalidPasswords,
  timeouts,
  endpoints,
};
