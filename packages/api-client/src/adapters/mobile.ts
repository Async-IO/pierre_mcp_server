// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Mobile platform adapter using SecureStore for tokens and AsyncStorage for non-sensitive data
// ABOUTME: Provides platform-specific implementation for React Native environment with secure token storage

import type { PlatformAdapter, AuthStorage, AuthFailureHandler } from '../types/platform';

const STORAGE_KEYS = {
  TOKEN: '@pierre/jwt_token',
  REFRESH_TOKEN: '@pierre/refresh_token',
  CSRF_TOKEN: '@pierre/csrf_token',
  USER: '@pierre/user',
} as const;

/**
 * Interface for AsyncStorage (to avoid direct dependency on @react-native-async-storage).
 * Consumers must provide their own AsyncStorage instance.
 * Used for non-sensitive data like user profile.
 */
export interface AsyncStorageLike {
  getItem(key: string): Promise<string | null>;
  setItem(key: string, value: string): Promise<void>;
  removeItem(key: string): Promise<void>;
  multiRemove(keys: string[]): Promise<void>;
}

/**
 * Interface for SecureStore (to avoid direct dependency on expo-secure-store).
 * Consumers must provide their own SecureStore instance.
 * Used for sensitive data like JWT and refresh tokens.
 */
export interface SecureStorageLike {
  getItemAsync(key: string): Promise<string | null>;
  setItemAsync(key: string, value: string): Promise<void>;
  deleteItemAsync(key: string): Promise<void>;
}

interface MobileAuthStorageOptions {
  /** AsyncStorage for non-sensitive data (user profile, CSRF token) */
  asyncStorage: AsyncStorageLike;
  /** SecureStore for sensitive data (JWT, refresh token). If not provided, falls back to AsyncStorage. */
  secureStorage?: SecureStorageLike;
}

/**
 * Creates an AuthStorage implementation using SecureStore for tokens and AsyncStorage for other data.
 * Tokens (JWT, refresh) are stored securely using expo-secure-store when available.
 * Non-sensitive data (user profile, CSRF) remains in AsyncStorage for performance.
 */
function createMobileAuthStorage(options: MobileAuthStorageOptions): AuthStorage {
  const { asyncStorage, secureStorage } = options;

  // Helper to get/set/remove from secure storage with AsyncStorage fallback
  const secureGet = async (key: string): Promise<string | null> => {
    if (secureStorage) {
      return secureStorage.getItemAsync(key);
    }
    return asyncStorage.getItem(key);
  };

  const secureSet = async (key: string, value: string): Promise<void> => {
    if (secureStorage) {
      await secureStorage.setItemAsync(key, value);
    } else {
      await asyncStorage.setItem(key, value);
    }
  };

  const secureRemove = async (key: string): Promise<void> => {
    if (secureStorage) {
      await secureStorage.deleteItemAsync(key);
    } else {
      await asyncStorage.removeItem(key);
    }
  };

  return {
    // JWT token - stored securely
    async getToken(): Promise<string | null> {
      return secureGet(STORAGE_KEYS.TOKEN);
    },

    async setToken(token: string): Promise<void> {
      await secureSet(STORAGE_KEYS.TOKEN, token);
    },

    async removeToken(): Promise<void> {
      await secureRemove(STORAGE_KEYS.TOKEN);
    },

    // CSRF token - stored in AsyncStorage (session-scoped, less sensitive)
    async getCsrfToken(): Promise<string | null> {
      return asyncStorage.getItem(STORAGE_KEYS.CSRF_TOKEN);
    },

    async setCsrfToken(token: string | null): Promise<void> {
      if (token) {
        await asyncStorage.setItem(STORAGE_KEYS.CSRF_TOKEN, token);
      } else {
        await asyncStorage.removeItem(STORAGE_KEYS.CSRF_TOKEN);
      }
    },

    // User profile - stored in AsyncStorage (not sensitive)
    async getUser<T>(): Promise<T | null> {
      const userJson = await asyncStorage.getItem(STORAGE_KEYS.USER);
      if (!userJson) return null;
      try {
        return JSON.parse(userJson) as T;
      } catch {
        return null;
      }
    },

    async setUser<T>(user: T): Promise<void> {
      await asyncStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(user));
    },

    // Refresh token - stored securely
    async getRefreshToken(): Promise<string | null> {
      return secureGet(STORAGE_KEYS.REFRESH_TOKEN);
    },

    async setRefreshToken(token: string): Promise<void> {
      await secureSet(STORAGE_KEYS.REFRESH_TOKEN, token);
    },

    async clear(): Promise<void> {
      // Clear secure storage tokens
      await secureRemove(STORAGE_KEYS.TOKEN);
      await secureRemove(STORAGE_KEYS.REFRESH_TOKEN);
      // Clear AsyncStorage items
      await asyncStorage.multiRemove([
        STORAGE_KEYS.CSRF_TOKEN,
        STORAGE_KEYS.USER,
      ]);
    },
  };
}

/**
 * Simple event emitter for auth failure events in React Native.
 */
class AuthEventEmitter {
  private listeners: Set<() => void> = new Set();

  emit(): void {
    this.listeners.forEach((listener) => listener());
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}

/**
 * Creates an AuthFailureHandler using a simple event emitter.
 */
function createMobileAuthFailureHandler(): AuthFailureHandler & { emitter: AuthEventEmitter } {
  const emitter = new AuthEventEmitter();

  return {
    emitter,
    onAuthFailure(): void {
      emitter.emit();
    },
    subscribe(listener: () => void): () => void {
      return emitter.subscribe(listener);
    },
  };
}

export interface MobileAdapterOptions {
  /** AsyncStorage instance from @react-native-async-storage/async-storage */
  asyncStorage: AsyncStorageLike;
  /** SecureStore instance from expo-secure-store for secure token storage. Highly recommended for production. */
  secureStorage?: SecureStorageLike;
  /** Base URL for API requests. Defaults to EXPO_PUBLIC_API_URL or platform-specific localhost */
  baseURL?: string;
  /** Request timeout in milliseconds. Defaults to 300000 (5 minutes for LLM responses) */
  timeout?: number;
}

/**
 * Creates a platform adapter for mobile (React Native) environment.
 */
export function createMobileAdapter(options: MobileAdapterOptions): PlatformAdapter {
  // Try to get base URL from Expo env
  const getDefaultBaseUrl = (): string => {
    if (typeof process !== 'undefined' && process.env?.EXPO_PUBLIC_API_URL) {
      return process.env.EXPO_PUBLIC_API_URL;
    }
    return 'http://localhost:8081';
  };

  return {
    httpConfig: {
      baseURL: options.baseURL ?? getDefaultBaseUrl(),
      timeout: options.timeout ?? 300000, // 5 minutes for slow LLM responses
      withCredentials: false, // Mobile uses Bearer token, not cookies
    },
    authStorage: createMobileAuthStorage({
      asyncStorage: options.asyncStorage,
      secureStorage: options.secureStorage,
    }),
    authFailure: createMobileAuthFailureHandler(),
    platform: 'mobile',
  };
}

/** Export storage keys for direct access if needed */
export { STORAGE_KEYS as MOBILE_STORAGE_KEYS };

/** Export the auth event emitter for testing */
export { AuthEventEmitter };
