// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: E2E tests for secure credential storage module in the SDK.
// ABOUTME: Tests store, retrieve, delete operations and security properties.

const { existsSync, unlinkSync, readFileSync } = require('fs');
const { join } = require('path');
const { homedir } = require('os');

// Set CI environment to force encrypted file storage
process.env.CI = 'true';

// Import from SDK main entry point (now exported)
const { createSecureStorage, EncryptedFileStorage } = require('../../dist/index.js');

const ENCRYPTED_FILE_PATH = join(homedir(), '.pierre-mcp-tokens.enc');
const TIMEOUT = 15000;

describe('Secure Storage E2E Tests', () => {
  let storage;

  beforeAll(async () => {
    // Create storage instance
    storage = await createSecureStorage((msg) => console.log(msg));
  });

  afterEach(async () => {
    // Clean up tokens after each test
    try {
      await storage.clearTokens();
    } catch (e) {
      // Ignore cleanup errors
    }
  });

  afterAll(() => {
    // Final cleanup
    if (existsSync(ENCRYPTED_FILE_PATH)) {
      unlinkSync(ENCRYPTED_FILE_PATH);
    }
  });

  describe('CRUD Operations', () => {
    test('should store tokens successfully', async () => {
      const tokens = {
        access_token: 'test-access-token-12345',
        refresh_token: 'test-refresh-token-67890',
        expires_at: Date.now() + 3600000,
      };

      await expect(storage.saveTokens(tokens)).resolves.not.toThrow();
    }, TIMEOUT);

    test('should retrieve stored tokens', async () => {
      const tokens = {
        access_token: 'test-access-token-abc',
        refresh_token: 'test-refresh-token-xyz',
        user_id: 'user-123',
      };

      await storage.saveTokens(tokens);
      const retrieved = await storage.getTokens();

      expect(retrieved).toBeDefined();
      expect(retrieved.access_token).toBe('test-access-token-abc');
      expect(retrieved.refresh_token).toBe('test-refresh-token-xyz');
      expect(retrieved.user_id).toBe('user-123');
    }, TIMEOUT);

    test('should return null for non-existent tokens', async () => {
      await storage.clearTokens();
      const retrieved = await storage.getTokens();

      expect(retrieved).toBeNull();
    }, TIMEOUT);

    test('should delete tokens successfully', async () => {
      const tokens = { access_token: 'to-be-deleted' };

      await storage.saveTokens(tokens);
      await storage.clearTokens();

      const retrieved = await storage.getTokens();
      expect(retrieved).toBeNull();
    }, TIMEOUT);

    test('should overwrite existing tokens', async () => {
      const tokens1 = { access_token: 'first-token' };
      const tokens2 = { access_token: 'second-token' };

      await storage.saveTokens(tokens1);
      await storage.saveTokens(tokens2);

      const retrieved = await storage.getTokens();
      expect(retrieved.access_token).toBe('second-token');
    }, TIMEOUT);
  });

  describe('Security Properties', () => {
    test('should not store tokens in plaintext', async () => {
      const tokens = {
        access_token: 'secret-token-should-not-appear',
        password: 'super-secret-password',
      };

      await storage.saveTokens(tokens);

      // For encrypted file storage, verify the file is encrypted
      if (existsSync(ENCRYPTED_FILE_PATH)) {
        const fileContent = readFileSync(ENCRYPTED_FILE_PATH, 'utf8');
        expect(fileContent).not.toContain('secret-token-should-not-appear');
        expect(fileContent).not.toContain('super-secret-password');
        // Should be in encrypted format (iv:authTag:ciphertext)
        expect(fileContent.split(':').length).toBe(3);
      }
    }, TIMEOUT);

    test('should handle complex token objects', async () => {
      const tokens = {
        access_token: 'complex-token',
        user: {
          id: 'user-456',
          email: 'test@example.com',
          roles: ['user', 'admin'],
        },
        metadata: {
          created_at: new Date().toISOString(),
          scopes: ['read', 'write'],
        },
      };

      await storage.saveTokens(tokens);
      const retrieved = await storage.getTokens();

      expect(retrieved.user).toEqual(tokens.user);
      expect(retrieved.metadata).toEqual(tokens.metadata);
    }, TIMEOUT);

    test('should handle special characters in tokens', async () => {
      const tokens = {
        access_token: 'token-with-special-chars-!@#$%^&*()',
        unicode_value: '🔒 secure token 日本語',
      };

      await storage.saveTokens(tokens);
      const retrieved = await storage.getTokens();

      expect(retrieved.access_token).toBe('token-with-special-chars-!@#$%^&*()');
      expect(retrieved.unicode_value).toBe('🔒 secure token 日本語');
    }, TIMEOUT);
  });

  describe('Error Handling', () => {
    test('should handle empty token object', async () => {
      await expect(storage.saveTokens({})).resolves.not.toThrow();
      const retrieved = await storage.getTokens();
      expect(retrieved).toEqual({});
    }, TIMEOUT);

    test('should handle clearTokens when no tokens exist', async () => {
      await storage.clearTokens();
      await expect(storage.clearTokens()).resolves.not.toThrow();
    }, TIMEOUT);
  });

  describe('Encrypted File Storage Specific', () => {
    let encryptedStorage;

    beforeEach(() => {
      encryptedStorage = new EncryptedFileStorage((msg) => console.log(msg));
    });

    afterEach(async () => {
      try {
        await encryptedStorage.clearTokens();
      } catch (e) {
        // Ignore
      }
    });

    test('should use encrypted file path', async () => {
      const tokens = { access_token: 'file-storage-test' };

      await encryptedStorage.saveTokens(tokens);

      expect(existsSync(ENCRYPTED_FILE_PATH)).toBe(true);
    }, TIMEOUT);

    test('should use AES-256-GCM encryption format', async () => {
      const tokens = { access_token: 'aes-gcm-test' };

      await encryptedStorage.saveTokens(tokens);

      if (existsSync(ENCRYPTED_FILE_PATH)) {
        const content = readFileSync(ENCRYPTED_FILE_PATH, 'utf8');
        const parts = content.split(':');

        // Format: iv:authTag:ciphertext
        expect(parts.length).toBe(3);
        // IV should be 24 hex chars (12 bytes)
        expect(parts[0].length).toBe(24);
        // AuthTag should be 32 hex chars (16 bytes)
        expect(parts[1].length).toBe(32);
        // Ciphertext should be present
        expect(parts[2].length).toBeGreaterThan(0);
      }
    }, TIMEOUT);
  });

  describe('Migration', () => {
    const legacyTokenPath = join(homedir(), '.pierre-mcp-tokens-test-legacy.json');
    const { writeFileSync } = require('fs');

    afterEach(() => {
      // Clean up test files
      [legacyTokenPath, `${legacyTokenPath}.backup`].forEach((path) => {
        if (existsSync(path)) {
          unlinkSync(path);
        }
      });
    });

    test('should migrate from plaintext file', async () => {
      const legacyTokens = {
        access_token: 'legacy-token',
        refresh_token: 'legacy-refresh',
      };

      // Create legacy plaintext file
      writeFileSync(legacyTokenPath, JSON.stringify(legacyTokens), 'utf8');

      // Migrate
      const migrated = await storage.migrateFromPlaintextFile(legacyTokenPath);

      expect(migrated).toBe(true);
      // Original file should be deleted
      expect(existsSync(legacyTokenPath)).toBe(false);
      // Backup should exist
      expect(existsSync(`${legacyTokenPath}.backup`)).toBe(true);
    }, TIMEOUT);

    test('should skip migration when no legacy file exists', async () => {
      const migrated = await storage.migrateFromPlaintextFile(
        '/nonexistent/path/to/tokens.json'
      );

      expect(migrated).toBe(false);
    }, TIMEOUT);
  });
});
