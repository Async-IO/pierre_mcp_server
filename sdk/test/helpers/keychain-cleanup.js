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
    // Dynamically import @napi-rs/keyring (Rust-based keyring, replaces deprecated keytar)
    // This matches the lazy-loading pattern in secure-storage.ts
    const { Entry } = await import('@napi-rs/keyring');

    const KEYCHAIN_SERVICE = 'pierre-mcp-client';
    const KEYCHAIN_ACCOUNT_PREFIX = 'pierre-mcp-tokens';

    const entry = new Entry(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT_PREFIX);
    entry.deletePassword();

    // Note: deletePassword throws if entry doesn't exist in @napi-rs/keyring
    // We catch and ignore since we just want to ensure it's cleared

  } catch (error) {
    // If keyring is not available or entry doesn't exist, silently continue
    // Tests will fall back to encrypted file storage
    if (process.env.DEBUG) {
      console.error('[Keychain Cleanup] Failed to clear keychain:', error.message);
    }
  }
}

module.exports = {
  clearKeychainTokens
};
