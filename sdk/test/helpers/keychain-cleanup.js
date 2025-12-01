// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Test helper to clean up OS keychain between tests
// ABOUTME: Prevents test pollution by clearing shared keychain storage

/**
 * Clear tokens from OS keychain to ensure test isolation
 *
 * CRITICAL: All tests share the same keychain service/account:
 * - Service: 'pierre-mcp-client'
 * - Account: 'pierre-mcp-tokens'
 *
 * Without cleanup, tokens from one test leak into subsequent tests,
 * causing flaky failures (especially when tests run without --token flag).
 */
async function clearKeychainTokens() {
  try {
    // Dynamically import keytar (only available when keytar is installed)
    // This matches the lazy-loading pattern in secure-storage.ts
    const keytar = await import('keytar');

    const KEYCHAIN_SERVICE = 'pierre-mcp-client';
    const KEYCHAIN_ACCOUNT_PREFIX = 'pierre-mcp-tokens';

    await keytar.deletePassword(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT_PREFIX);

    // Note: deletePassword returns boolean (true if deleted, false if not found)
    // We don't check the return value because we just want to ensure it's cleared

  } catch (error) {
    // If keytar is not available or fails, silently continue
    // Tests will fall back to encrypted file storage
    if (process.env.DEBUG) {
      console.error('[Keychain Cleanup] Failed to clear keychain:', error.message);
    }
  }
}

module.exports = {
  clearKeychainTokens
};
