// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Mobile platform adapter using AsyncStorage and event emitters
// ABOUTME: Provides platform-specific implementation for React Native environment

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
 */
export interface AsyncStorageLike {
  getItem(key: string): Promise<string | null>;
  setItem(key: string, value: string): Promise<void>;
  removeItem(key: string): Promise<void>;
  multiRemove(keys: string[]): Promise<void>;
}

/**
 * Creates an AuthStorage implementation using React Native AsyncStorage.
 */
function createMobileAuthStorage(asyncStorage: AsyncStorageLike): AuthStorage {
  return {
    async getToken(): Promise<string | null> {
      return asyncStorage.getItem(STORAGE_KEYS.TOKEN);
    },

    async setToken(token: string): Promise<void> {
      await asyncStorage.setItem(STORAGE_KEYS.TOKEN, token);
    },

    async removeToken(): Promise<void> {
      await asyncStorage.removeItem(STORAGE_KEYS.TOKEN);
    },

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

    async getRefreshToken(): Promise<string | null> {
      return asyncStorage.getItem(STORAGE_KEYS.REFRESH_TOKEN);
    },

    async setRefreshToken(token: string): Promise<void> {
      await asyncStorage.setItem(STORAGE_KEYS.REFRESH_TOKEN, token);
    },

    async clear(): Promise<void> {
      await asyncStorage.multiRemove([
        STORAGE_KEYS.TOKEN,
        STORAGE_KEYS.REFRESH_TOKEN,
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
    authStorage: createMobileAuthStorage(options.asyncStorage),
    authFailure: createMobileAuthFailureHandler(),
    platform: 'mobile',
  };
}

/** Export storage keys for direct access if needed */
export { STORAGE_KEYS as MOBILE_STORAGE_KEYS };

/** Export the auth event emitter for testing */
export { AuthEventEmitter };
