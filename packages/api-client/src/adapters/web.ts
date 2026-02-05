// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Web platform adapter using localStorage and window events
// ABOUTME: Provides platform-specific implementation for browser environment

import type { PlatformAdapter, AuthStorage, AuthFailureHandler } from '../types/platform';

const STORAGE_KEYS = {
  TOKEN: 'pierre_auth_token',
  REFRESH_TOKEN: 'pierre_refresh_token',
  CSRF_TOKEN: 'pierre_csrf_token',
  USER: 'pierre_user',
} as const;

const AUTH_FAILURE_EVENT = 'pierre:auth:failure';

/**
 * Creates an AuthStorage implementation for browser environment.
 *
 * JWT tokens are NOT stored in localStorage (security: prevents XSS token theft).
 * REST auth uses httpOnly cookies set by the server. JWT is only held in React
 * state for WebSocket authentication. User data stays in localStorage for instant
 * UI rendering before session restore completes.
 */
function createWebAuthStorage(): AuthStorage {
  return {
    // Token methods are no-ops — httpOnly cookies handle REST auth,
    // React state holds JWT for WebSocket only
    async getToken(): Promise<string | null> {
      return null;
    },

    async setToken(): Promise<void> {
      // No-op: JWT stored in React state, not localStorage
    },

    async removeToken(): Promise<void> {
      // No-op: JWT not stored in localStorage
    },

    async getCsrfToken(): Promise<string | null> {
      return localStorage.getItem(STORAGE_KEYS.CSRF_TOKEN);
    },

    async setCsrfToken(token: string | null): Promise<void> {
      if (token) {
        localStorage.setItem(STORAGE_KEYS.CSRF_TOKEN, token);
      } else {
        localStorage.removeItem(STORAGE_KEYS.CSRF_TOKEN);
      }
    },

    async getUser<T>(): Promise<T | null> {
      const userJson = localStorage.getItem(STORAGE_KEYS.USER);
      if (!userJson) return null;
      try {
        return JSON.parse(userJson) as T;
      } catch {
        return null;
      }
    },

    async setUser<T>(user: T): Promise<void> {
      localStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(user));
    },

    // Refresh token is no-op — session restore uses httpOnly cookie
    async getRefreshToken(): Promise<string | null> {
      return null;
    },

    async setRefreshToken(): Promise<void> {
      // No-op: refresh handled via httpOnly cookie session
    },

    async clear(): Promise<void> {
      localStorage.removeItem(STORAGE_KEYS.CSRF_TOKEN);
      localStorage.removeItem(STORAGE_KEYS.USER);
    },
  };
}

/**
 * Creates an AuthFailureHandler using window custom events.
 */
function createWebAuthFailureHandler(): AuthFailureHandler {
  return {
    onAuthFailure(): void {
      window.dispatchEvent(new CustomEvent(AUTH_FAILURE_EVENT));
    },

    subscribe(listener: () => void): () => void {
      window.addEventListener(AUTH_FAILURE_EVENT, listener);
      return () => window.removeEventListener(AUTH_FAILURE_EVENT, listener);
    },
  };
}

export interface WebAdapterOptions {
  /** Base URL for API requests. Defaults to VITE_API_URL or window.location.origin */
  baseURL?: string;
  /** Request timeout in milliseconds. Defaults to 30000 */
  timeout?: number;
}

/**
 * Gets the default base URL for web environment.
 * Callers should provide baseURL explicitly when possible.
 */
function getDefaultBaseUrl(): string {
  // Check for Vite environment variable (accessed at runtime)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const importMeta = typeof import.meta !== 'undefined' ? (import.meta as any) : undefined;
  if (importMeta?.env?.VITE_API_URL) {
    return importMeta.env.VITE_API_URL as string;
  }
  // Fallback to current origin
  if (typeof window !== 'undefined') {
    return window.location.origin;
  }
  return 'http://localhost:8081';
}

/**
 * Creates a platform adapter for web (browser) environment.
 */
export function createWebAdapter(options?: WebAdapterOptions): PlatformAdapter {
  return {
    httpConfig: {
      baseURL: options?.baseURL ?? getDefaultBaseUrl(),
      timeout: options?.timeout ?? 30000,
      withCredentials: true, // Web uses httpOnly cookies
    },
    authStorage: createWebAuthStorage(),
    authFailure: createWebAuthFailureHandler(),
    platform: 'web',
  };
}

/** Export storage keys for direct access if needed */
export { STORAGE_KEYS as WEB_STORAGE_KEYS, AUTH_FAILURE_EVENT as WEB_AUTH_FAILURE_EVENT };
