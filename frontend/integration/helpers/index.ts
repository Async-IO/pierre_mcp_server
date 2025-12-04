// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Re-exports all integration test helpers for convenient imports.
// ABOUTME: Provides a single entry point for commonly used test utilities.

export {
  waitForBackendHealth,
  waitForFrontendReady,
  waitForServersReady,
  getBackendUrl,
  getFrontendUrl,
  type HealthCheckResult,
} from './server-manager';

export {
  createTestAdminUser,
  generateApiToken,
  cleanupTestDatabase,
  ensureDataDirectory,
  getTestDatabasePath,
  testDatabaseExists,
  type TestUser,
  type CreateUserResult,
} from './db-setup';

export {
  loginWithCredentials,
  createAndLoginAsAdmin,
  createAndLoginAsSuperAdmin,
  createAndLoginTestUser,
  logout,
  isLoggedIn,
  navigateToTab,
  waitForDashboardLoad,
  type LoginResult,
} from './auth-helpers';
