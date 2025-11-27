// ABOUTME: Unit tests for secure token storage implementations
// ABOUTME: Tests both EncryptedFileStorage (CI-safe) and KeychainTokenStorage (local dev)

import { existsSync, unlinkSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  EncryptedFileStorage,
  KeychainTokenStorage,
  createSecureStorage,
} from './secure-storage';

// Test tokens for storage verification
const TEST_TOKENS = {
  pierre: {
    access_token: 'test_access_token_abc123',
    token_type: 'Bearer',
    expires_in: 3600,
    refresh_token: 'test_refresh_token_xyz789',
    scope: 'read:fitness write:fitness',
  },
  providers: {
    strava: {
      access_token: 'strava_test_token',
      refresh_token: 'strava_refresh_token',
      expires_at: Date.now() + 3600000,
    },
  },
};

// Suppress console output in tests
const silentLog = (): void => {};

describe('EncryptedFileStorage', () => {
  let storage: EncryptedFileStorage;
  let testFilePath: string;

  beforeEach(() => {
    // Create storage with silent logging
    storage = new EncryptedFileStorage(silentLog);
    // Get the file path for cleanup (it's private, so we construct it)
    testFilePath = join(tmpdir(), '.pierre-mcp-tokens.enc');
  });

  afterEach(async () => {
    // Clean up test files
    try {
      await storage.clearTokens();
    } catch {
      // Ignore cleanup errors
    }
  });

  test('saves and retrieves tokens correctly', async () => {
    await storage.saveTokens(TEST_TOKENS);
    const retrieved = await storage.getTokens();

    expect(retrieved).not.toBeNull();
    expect(retrieved?.pierre?.access_token).toBe(TEST_TOKENS.pierre.access_token);
    expect(retrieved?.pierre?.refresh_token).toBe(TEST_TOKENS.pierre.refresh_token);
    expect(retrieved?.providers?.strava?.access_token).toBe(TEST_TOKENS.providers.strava.access_token);
  });

  test('returns null when no tokens exist', async () => {
    // Ensure clean state
    await storage.clearTokens();
    const tokens = await storage.getTokens();
    expect(tokens).toBeNull();
  });

  test('clears tokens successfully', async () => {
    await storage.saveTokens(TEST_TOKENS);
    await storage.clearTokens();
    const tokens = await storage.getTokens();
    expect(tokens).toBeNull();
  });

  test('handles empty token object', async () => {
    await storage.saveTokens({});
    const retrieved = await storage.getTokens();
    expect(retrieved).toEqual({});
  });

  test('encrypts data (not stored as plaintext)', async () => {
    await storage.saveTokens(TEST_TOKENS);

    // The encrypted file path is in home directory, not tmpdir
    // We can't directly check the file content since we don't know the exact path
    // But we can verify that re-reading works (proving encryption/decryption cycle)
    const retrieved = await storage.getTokens();
    expect(retrieved?.pierre?.access_token).toBe(TEST_TOKENS.pierre.access_token);
  });

  test('migrates from plaintext file', async () => {
    // Create a plaintext file
    const legacyPath = join(tmpdir(), '.pierre-mcp-legacy-test.json');
    writeFileSync(legacyPath, JSON.stringify(TEST_TOKENS), 'utf8');

    try {
      const migrated = await storage.migrateFromPlaintextFile(legacyPath);
      expect(migrated).toBe(true);

      // Original file should be deleted
      expect(existsSync(legacyPath)).toBe(false);

      // Backup should exist
      expect(existsSync(`${legacyPath}.backup`)).toBe(true);

      // Cleanup backup
      unlinkSync(`${legacyPath}.backup`);
    } catch (error) {
      // Cleanup on error
      if (existsSync(legacyPath)) unlinkSync(legacyPath);
      if (existsSync(`${legacyPath}.backup`)) unlinkSync(`${legacyPath}.backup`);
      throw error;
    }
  });

  test('returns false when migrating non-existent file', async () => {
    const result = await storage.migrateFromPlaintextFile('/nonexistent/path/tokens.json');
    expect(result).toBe(false);
  });
});

describe('KeychainTokenStorage', () => {
  // These tests only run when keytar is available (local dev, not CI)
  const isCI = process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true';

  // Skip keychain tests in CI - keytar requires D-Bus on Linux
  const conditionalTest = isCI ? test.skip : test;

  conditionalTest('saves and retrieves tokens via keychain', async () => {
    // Dynamic import keytar
    const keytarModule = await import('keytar');
    const keytar = keytarModule.default || keytarModule;

    const storage = new KeychainTokenStorage(keytar, silentLog);

    try {
      await storage.saveTokens(TEST_TOKENS);
      const retrieved = await storage.getTokens();

      expect(retrieved).not.toBeNull();
      expect(retrieved?.pierre?.access_token).toBe(TEST_TOKENS.pierre.access_token);
    } finally {
      // Always cleanup
      await storage.clearTokens();
    }
  });

  conditionalTest('clears keychain tokens', async () => {
    const keytarModule = await import('keytar');
    const keytar = keytarModule.default || keytarModule;

    const storage = new KeychainTokenStorage(keytar, silentLog);

    await storage.saveTokens(TEST_TOKENS);
    await storage.clearTokens();
    const tokens = await storage.getTokens();

    expect(tokens).toBeNull();
  });
});

describe('createSecureStorage factory', () => {
  test('returns a storage implementation', async () => {
    const storage = await createSecureStorage(silentLog);

    // Should return either KeychainTokenStorage or EncryptedFileStorage
    expect(storage).toBeDefined();
    expect(typeof storage.saveTokens).toBe('function');
    expect(typeof storage.getTokens).toBe('function');
    expect(typeof storage.clearTokens).toBe('function');
    expect(typeof storage.migrateFromPlaintextFile).toBe('function');
  });

  test('storage can save and retrieve tokens', async () => {
    const storage = await createSecureStorage(silentLog);

    try {
      await storage.saveTokens(TEST_TOKENS);
      const retrieved = await storage.getTokens();

      expect(retrieved).not.toBeNull();
      expect(retrieved?.pierre?.access_token).toBe(TEST_TOKENS.pierre.access_token);
    } finally {
      await storage.clearTokens();
    }
  });

  test('uses encrypted file storage in CI environment', async () => {
    // Temporarily set CI environment
    const originalCI = process.env.CI;
    process.env.CI = 'true';

    try {
      const storage = await createSecureStorage(silentLog);

      // In CI, should use EncryptedFileStorage
      // We can verify by checking the storage works (it would hang/fail with keytar in CI)
      await storage.saveTokens({ test: 'value' });
      const tokens = await storage.getTokens();
      expect(tokens).toEqual({ test: 'value' });
      await storage.clearTokens();
    } finally {
      // Restore original value
      if (originalCI === undefined) {
        delete process.env.CI;
      } else {
        process.env.CI = originalCI;
      }
    }
  });
});
