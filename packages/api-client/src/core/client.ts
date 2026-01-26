// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Core API client factory with platform-agnostic axios configuration
// ABOUTME: Handles auth interceptors, CSRF tokens, and error handling

import * as axiosModule from 'axios';
import type { AxiosInstance, AxiosResponse, AxiosError, InternalAxiosRequestConfig } from 'axios';

// Handle both ESM and CJS imports (for test environment compatibility)
const axios = axiosModule.default ?? axiosModule;
import type { PlatformAdapter, ApiClientOptions } from '../types/platform';

/**
 * Creates an axios instance configured with the platform adapter.
 * Handles authentication, CSRF tokens, and auth failure detection.
 */
export function createAxiosClient(adapter: PlatformAdapter): AxiosInstance {
  const { httpConfig, authStorage, authFailure } = adapter;

  const instance = axios.create({
    baseURL: httpConfig.baseURL,
    timeout: httpConfig.timeout ?? 30000,
    withCredentials: httpConfig.withCredentials ?? false,
    headers: {
      'Content-Type': 'application/json',
      ...httpConfig.defaultHeaders,
    },
  });

  // Request interceptor: Add auth token and CSRF token
  instance.interceptors.request.use(
    async (config: InternalAxiosRequestConfig) => {
      // Add JWT token for mobile (web uses httpOnly cookies)
      if (adapter.platform === 'mobile') {
        const token = await authStorage.getToken();
        if (token && config.headers) {
          config.headers.Authorization = `Bearer ${token}`;
        }
      }

      // Add CSRF token for state-changing requests
      const csrfToken = await authStorage.getCsrfToken();
      if (csrfToken && config.headers) {
        config.headers['X-CSRF-Token'] = csrfToken;
      }

      return config;
    },
    (error: AxiosError) => Promise.reject(error)
  );

  // Response interceptor: Handle auth failures and extract CSRF tokens
  instance.interceptors.response.use(
    (response: AxiosResponse) => {
      // Extract CSRF token from response headers if present
      const csrfToken = response.headers['x-csrf-token'];
      if (csrfToken) {
        // Fire and forget - don't await
        authStorage.setCsrfToken(csrfToken).catch(() => {
          // Silently ignore storage errors
        });
      }
      return response;
    },
    async (error: AxiosError) => {
      // Handle 401 Unauthorized
      if (error.response?.status === 401) {
        // Clear auth data
        await authStorage.clear();
        // Notify listeners
        authFailure.onAuthFailure();
      }
      return Promise.reject(error);
    }
  );

  return instance;
}

/**
 * API client instance with access to the underlying axios instance
 * and platform adapter for domain APIs to use.
 */
export interface ApiClient {
  /** Configured axios instance */
  axios: AxiosInstance;
  /** Platform adapter for auth operations */
  adapter: PlatformAdapter;
}

/**
 * Creates an API client with the given options.
 */
export function createApiClient(options: ApiClientOptions): ApiClient {
  const axiosInstance = options.axiosInstance ?? createAxiosClient(options.adapter);

  return {
    axios: axiosInstance,
    adapter: options.adapter,
  };
}
