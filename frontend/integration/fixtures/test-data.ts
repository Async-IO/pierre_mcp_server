// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Test data fixtures for integration tests.
// ABOUTME: Provides consistent test users, API keys, and other test data.

import type { TestUser } from '../helpers/db-setup';

/**
 * Pre-defined test users for integration tests.
 * These users are created via the pierre-cli binary.
 */
export const testUsers: Record<string, TestUser> = {
  admin: {
    email: 'integration-admin@test.local',
    password: 'IntegrationTestPass123!',
    role: 'admin',
  },
  superAdmin: {
    email: 'integration-super@test.local',
    password: 'SuperAdminPass456!',
    role: 'super_admin',
  },
  regularUser: {
    email: 'integration-user@test.local',
    password: 'RegularUserPass789!',
    role: 'user',
  },
};

/**
 * Test API key configurations.
 */
export const testApiKeys = {
  readOnly: {
    name: 'Integration Test Read-Only Key',
    scopes: ['read'],
  },
  readWrite: {
    name: 'Integration Test Read-Write Key',
    scopes: ['read', 'write'],
  },
  fullAccess: {
    name: 'Integration Test Full Access Key',
    scopes: ['read', 'write', 'admin'],
  },
};

/**
 * Expected dashboard statistics structure.
 */
export interface DashboardOverview {
  total_api_keys: number;
  active_api_keys: number;
  total_requests_today: number;
  total_requests_this_month: number;
}

/**
 * Generate a unique email for test isolation.
 */
export function generateUniqueEmail(prefix: string = 'test'): string {
  const timestamp = Date.now();
  const random = Math.random().toString(36).substring(2, 8);
  return `${prefix}-${timestamp}-${random}@test.local`;
}

/**
 * Generate a unique API key name for test isolation.
 */
export function generateUniqueKeyName(prefix: string = 'Test Key'): string {
  const timestamp = Date.now();
  return `${prefix} ${timestamp}`;
}

/**
 * Valid password that meets typical requirements.
 */
export const validPassword = 'ValidTestPass123!';

/**
 * Invalid passwords for negative testing.
 */
export const invalidPasswords = {
  tooShort: 'short',
  noUppercase: 'lowercaseonly123!',
  noLowercase: 'UPPERCASEONLY123!',
  noNumbers: 'NoNumbersHere!',
  noSpecial: 'NoSpecialChars123',
};

/**
 * Common test timeouts (in milliseconds).
 */
export const timeouts = {
  short: 5000,
  medium: 10000,
  long: 30000,
  serverStart: 60000,
};
