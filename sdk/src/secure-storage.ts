// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Secure token storage using OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
// ABOUTME: Provides encrypted storage for OAuth tokens with automatic migration from plaintext files

// NOTE: keytar is lazy-loaded to prevent D-Bus hangs in Linux CI environments
// import * as keytar from 'keytar';  // Moved to lazy loading in createSecureStorage()
import { readFileSync, writeFileSync, existsSync, unlinkSync } from 'fs';
import { join } from 'path';
import { homedir, networkInterfaces } from 'os';
import { createCipheriv, createDecipheriv, randomBytes, createHash } from 'crypto';

const KEYCHAIN_SERVICE = 'pierre-mcp-client';
const KEYCHAIN_ACCOUNT_PREFIX = 'pierre-mcp-tokens';

/**
 * Secure token storage interface
 */
export interface SecureTokenStorage {
  /**
   * Store tokens securely in OS keychain
   */
  saveTokens(tokens: Record<string, any>): Promise<void>;

  /**
   * Retrieve tokens from OS keychain
   */
  getTokens(): Promise<Record<string, any> | null>;

  /**
   * Clear all tokens from OS keychain
   */
  clearTokens(): Promise<void>;

  /**
   * Migrate tokens from plaintext file to keychain
   */
  migrateFromPlaintextFile(filePath: string): Promise<boolean>;
}

/**
 * OS Keychain-based secure storage implementation
 * NOTE: keytar must be passed as a parameter to avoid top-level import issues
 */
export class KeychainTokenStorage implements SecureTokenStorage {
  private log: (message: string, ...args: any[]) => void;
  private keytar: any;

  constructor(keytar: any, logFunction?: (message: string, ...args: any[]) => void) {
    this.keytar = keytar;
    this.log = logFunction || ((msg) => console.error(`[SecureStorage] ${msg}`));
  }

  async saveTokens(tokens: Record<string, any>): Promise<void> {
    try {
      const serialized = JSON.stringify(tokens);
      await this.keytar.setPassword(
        KEYCHAIN_SERVICE,
        KEYCHAIN_ACCOUNT_PREFIX,
        serialized
      );
      this.log('Saved tokens to OS keychain');
    } catch (error) {
      this.log(`Failed to save tokens to keychain: ${error}`);
      throw new Error(`Keychain storage failed: ${error}`);
    }
  }

  async getTokens(): Promise<Record<string, any> | null> {
    try {
      const serialized = await this.keytar.getPassword(
        KEYCHAIN_SERVICE,
        KEYCHAIN_ACCOUNT_PREFIX
      );

      if (!serialized) {
        this.log('No tokens found in keychain');
        return null;
      }

      const tokens = JSON.parse(serialized);
      this.log('Retrieved tokens from keychain');
      return tokens;
    } catch (error) {
      this.log(`Failed to retrieve tokens from keychain: ${error}`);
      return null;
    }
  }

  async clearTokens(): Promise<void> {
    try {
      const deleted = await this.keytar.deletePassword(
        KEYCHAIN_SERVICE,
        KEYCHAIN_ACCOUNT_PREFIX
      );
      if (deleted) {
        this.log('Cleared tokens from keychain');
      } else {
        this.log('No tokens found to clear in keychain');
      }
    } catch (error) {
      this.log(`Failed to clear tokens from keychain: ${error}`);
      throw new Error(`Keychain clear failed: ${error}`);
    }
  }

  async migrateFromPlaintextFile(filePath: string): Promise<boolean> {
    try {
      if (!existsSync(filePath)) {
        this.log(`No plaintext token file found at ${filePath}, skipping migration`);
        return false;
      }

      this.log(`Migrating tokens from plaintext file: ${filePath}`);

      // Read plaintext tokens
      const plaintextData = readFileSync(filePath, 'utf8');
      const tokens = JSON.parse(plaintextData);

      // Save to keychain
      await this.saveTokens(tokens);

      // Create backup before deletion
      const backupPath = `${filePath}.backup`;
      writeFileSync(backupPath, plaintextData, 'utf8');
      this.log(`Created backup of plaintext tokens at ${backupPath}`);

      // Delete plaintext file
      unlinkSync(filePath);
      this.log(`Deleted plaintext token file: ${filePath}`);

      this.log('Successfully migrated tokens to keychain');
      return true;
    } catch (error) {
      this.log(`Failed to migrate tokens: ${error}`);
      // Don't throw - migration failure shouldn't break the app
      return false;
    }
  }
}

/**
 * Encrypted file-based storage (fallback when keychain unavailable)
 * Uses AES-256-GCM with machine-specific key derivation
 */
export class EncryptedFileStorage implements SecureTokenStorage {
  private log: (message: string, ...args: any[]) => void;
  private encryptedFilePath: string;
  private encryptionKey: Buffer;

  constructor(logFunction?: (message: string, ...args: any[]) => void) {
    this.log = logFunction || ((msg) => console.error(`[SecureStorage] ${msg}`));
    this.encryptedFilePath = join(homedir(), '.pierre-mcp-tokens.enc');
    this.encryptionKey = this.deriveEncryptionKey();
  }

  /**
   * Derive encryption key from machine-specific data
   * Uses MAC addresses and homedir to create a stable machine-specific key
   */
  private deriveEncryptionKey(): Buffer {
    const interfaces = networkInterfaces();
    const macAddresses: string[] = [];

    // Collect MAC addresses for machine fingerprint
    for (const name of Object.keys(interfaces)) {
      const iface = interfaces[name];
      if (iface) {
        for (const addr of iface) {
          if (addr.mac && addr.mac !== '00:00:00:00:00:00') {
            macAddresses.push(addr.mac);
          }
        }
      }
    }

    // Combine MAC addresses and homedir for stable machine-specific seed
    const macSeed = macAddresses.sort().join(':') || 'default-seed';
    const homeSeed = homedir();
    const combinedSeed = `${macSeed}:${homeSeed}:pierre-mcp-encryption-v1`;

    // Derive 32-byte key using SHA-256
    return createHash('sha256').update(combinedSeed).digest();
  }

  /**
   * Encrypt data using AES-256-GCM
   */
  private encrypt(data: string): string {
    const iv = randomBytes(12); // 96-bit IV for GCM
    const cipher = createCipheriv('aes-256-gcm', this.encryptionKey, iv);

    let encrypted = cipher.update(data, 'utf8', 'hex');
    encrypted += cipher.final('hex');

    const authTag = cipher.getAuthTag();

    // Format: iv:authTag:encrypted
    return `${iv.toString('hex')}:${authTag.toString('hex')}:${encrypted}`;
  }

  /**
   * Decrypt data using AES-256-GCM
   */
  private decrypt(encryptedData: string): string {
    const parts = encryptedData.split(':');
    if (parts.length !== 3) {
      throw new Error('Invalid encrypted data format');
    }

    const iv = Buffer.from(parts[0], 'hex');
    const authTag = Buffer.from(parts[1], 'hex');
    const encrypted = parts[2];

    const decipher = createDecipheriv('aes-256-gcm', this.encryptionKey, iv);
    decipher.setAuthTag(authTag);

    let decrypted = decipher.update(encrypted, 'hex', 'utf8');
    decrypted += decipher.final('utf8');

    return decrypted;
  }

  async saveTokens(tokens: Record<string, any>): Promise<void> {
    try {
      const serialized = JSON.stringify(tokens);
      const encrypted = this.encrypt(serialized);
      writeFileSync(this.encryptedFilePath, encrypted, 'utf8');
      this.log('Saved tokens to encrypted file');
    } catch (error) {
      this.log(`Failed to save tokens to encrypted file: ${error}`);
      throw new Error(`Encrypted file storage failed: ${error}`);
    }
  }

  async getTokens(): Promise<Record<string, any> | null> {
    try {
      if (!existsSync(this.encryptedFilePath)) {
        this.log('No encrypted token file found');
        return null;
      }

      const encryptedData = readFileSync(this.encryptedFilePath, 'utf8');
      const decrypted = this.decrypt(encryptedData);
      const tokens = JSON.parse(decrypted);

      this.log('Retrieved tokens from encrypted file');
      return tokens;
    } catch (error) {
      this.log(`Failed to retrieve tokens from encrypted file: ${error}`);
      return null;
    }
  }

  async clearTokens(): Promise<void> {
    try {
      if (existsSync(this.encryptedFilePath)) {
        unlinkSync(this.encryptedFilePath);
        this.log('Cleared tokens from encrypted file');
      } else {
        this.log('No encrypted token file found to clear');
      }
    } catch (error) {
      this.log(`Failed to clear tokens from encrypted file: ${error}`);
      throw new Error(`Encrypted file clear failed: ${error}`);
    }
  }

  async migrateFromPlaintextFile(filePath: string): Promise<boolean> {
    try {
      if (!existsSync(filePath)) {
        this.log(`No plaintext token file found at ${filePath}, skipping migration`);
        return false;
      }

      this.log(`Migrating tokens from plaintext file to encrypted storage: ${filePath}`);

      // Read plaintext tokens
      const plaintextData = readFileSync(filePath, 'utf8');
      const tokens = JSON.parse(plaintextData);

      // Save to encrypted file
      await this.saveTokens(tokens);

      // Create backup before deletion
      const backupPath = `${filePath}.backup`;
      writeFileSync(backupPath, plaintextData, 'utf8');
      this.log(`Created backup of plaintext tokens at ${backupPath}`);

      // Delete plaintext file
      unlinkSync(filePath);
      this.log(`Deleted plaintext token file: ${filePath}`);

      this.log('Successfully migrated tokens to encrypted file');
      return true;
    } catch (error) {
      this.log(`Failed to migrate tokens: ${error}`);
      return false;
    }
  }
}

/**
 * Factory function to create secure storage with automatic fallback
 * Tries OS keychain first, falls back to encrypted file if unavailable
 */
export async function createSecureStorage(
  logFunction?: (message: string, ...args: any[]) => void
): Promise<SecureTokenStorage> {
  const log = logFunction || ((msg: string) => console.error(`[SecureStorage] ${msg}`));

  // DEBUG: Log environment variable values
  log('[DEBUG] createSecureStorage() called');
  log(`[DEBUG]   process.env.CI = "${process.env.CI}"`);
  log(`[DEBUG]   process.env.GITHUB_ACTIONS = "${process.env.GITHUB_ACTIONS}"`);
  log(`[DEBUG]   CI check: ${process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true'}`);

  // KNOWN ISSUE: keytar hangs on Linux CI due to D-Bus access requirements.
  // Workaround: Use encrypted file storage in CI environments to prevent MCP validator timeout.
  // Background: keytar requires D-Bus for credential storage on Linux, which is not available in CI containers.
  if (process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true') {
    log('CI environment detected - using encrypted file storage (keytar disabled for now)');
    const encryptedStorage = new EncryptedFileStorage(logFunction);

    // Attempt migration from plaintext file
    const legacyTokenPath = join(homedir(), '.pierre-mcp-tokens.json');
    await encryptedStorage.migrateFromPlaintextFile(legacyTokenPath);

    return encryptedStorage;
  }

  // Try OS keychain first (lazy-load keytar to avoid D-Bus hangs on Linux CI)
  try {
    log('[DEBUG] Attempting to lazy-load keytar...');
    // Use dynamic import to avoid loading keytar at module-import time
    const keytar = await import('keytar');
    log('[DEBUG] keytar loaded successfully');

    const keychainStorage = new KeychainTokenStorage(keytar, logFunction);

    // Test keychain availability by trying to get tokens
    await keychainStorage.getTokens();

    log('Using OS keychain for secure token storage');

    // Attempt automatic migration from legacy plaintext file
    const legacyTokenPath = join(homedir(), '.pierre-mcp-tokens.json');
    await keychainStorage.migrateFromPlaintextFile(legacyTokenPath);

    return keychainStorage;
  } catch (error) {
    log(`OS keychain unavailable: ${error}`);
    log('Falling back to encrypted file storage');

    // Fallback to encrypted file storage
    const encryptedStorage = new EncryptedFileStorage(logFunction);

    // Attempt migration with fallback storage
    const legacyTokenPath = join(homedir(), '.pierre-mcp-tokens.json');
    await encryptedStorage.migrateFromPlaintextFile(legacyTokenPath);

    return encryptedStorage;
  }
}
