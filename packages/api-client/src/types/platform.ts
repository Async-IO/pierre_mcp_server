// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Platform abstraction interfaces for HTTP client and auth storage
// ABOUTME: Allows the same API logic to work on web (axios/localStorage) and mobile (axios/AsyncStorage)

import type { AxiosInstance } from 'axios';

/**
 * Platform-specific HTTP client configuration.
 * Abstracts differences between web and mobile axios setup.
 */
export interface HttpClientConfig {
  /** Base URL for API requests */
  baseURL: string;
  /** Default timeout in milliseconds */
  timeout?: number;
  /** Whether to include credentials (cookies) - web only */
  withCredentials?: boolean;
  /** Default headers to include */
  defaultHeaders?: Record<string, string>;
}

/**
 * Platform-specific authentication storage.
 * Abstracts localStorage (web) vs AsyncStorage (mobile).
 * All methods are async for mobile compatibility.
 */
export interface AuthStorage {
  /** Get JWT token */
  getToken(): Promise<string | null>;
  /** Store JWT token */
  setToken(token: string): Promise<void>;
  /** Remove JWT token */
  removeToken(): Promise<void>;
  /** Get CSRF token */
  getCsrfToken(): Promise<string | null>;
  /** Store CSRF token */
  setCsrfToken(token: string | null): Promise<void>;
  /** Get stored user data */
  getUser<T>(): Promise<T | null>;
  /** Store user data */
  setUser<T>(user: T): Promise<void>;
  /** Get refresh token */
  getRefreshToken(): Promise<string | null>;
  /** Store refresh token */
  setRefreshToken(token: string): Promise<void>;
  /** Clear all auth data */
  clear(): Promise<void>;
}

/**
 * Platform-specific auth failure handler.
 * Abstracts window events (web) vs event emitter (mobile).
 */
export interface AuthFailureHandler {
  /** Called when authentication fails (401 response) */
  onAuthFailure(): void;
  /** Subscribe to auth failure events */
  subscribe(listener: () => void): () => void;
}

/**
 * Complete platform adapter combining all platform-specific concerns.
 */
export interface PlatformAdapter {
  /** HTTP client configuration */
  httpConfig: HttpClientConfig;
  /** Auth storage implementation */
  authStorage: AuthStorage;
  /** Auth failure handler */
  authFailure: AuthFailureHandler;
  /** Platform identifier for debugging */
  platform: 'web' | 'mobile';
}

/**
 * Options for creating the API client.
 */
export interface ApiClientOptions {
  /** Platform adapter providing platform-specific implementations */
  adapter: PlatformAdapter;
  /** Optional axios instance (for testing/customization) */
  axiosInstance?: AxiosInstance;
}

/**
 * Standard API response metadata.
 */
export interface ApiMetadata {
  timestamp: string;
  api_version: string;
}

/**
 * Paginated response with cursor.
 */
export interface CursorPaginatedResponse<T> {
  items: T[];
  next_cursor: string | null;
  has_more: boolean;
  metadata: ApiMetadata;
}

/**
 * Paginated response with offset.
 */
export interface OffsetPaginatedResponse<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
  metadata: ApiMetadata;
}
