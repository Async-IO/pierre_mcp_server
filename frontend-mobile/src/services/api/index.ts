// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// ABOUTME: API service entry point using @pierre/api-client with mobile adapter
// ABOUTME: Provides backward-compatible apiService interface for existing code

import { Platform } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import {
  createMobileAdapter,
  createPierreApi,
  createLegacyApiService,
  type PierreApiService,
} from '@pierre/api-client';

// Re-export types for consumers
export type { ForkCoachResponse } from '@pierre/api-client';

// Determine API URL based on platform
const getApiUrl = (): string => {
  if (process.env.EXPO_PUBLIC_API_URL) {
    return process.env.EXPO_PUBLIC_API_URL;
  }
  // Android emulator cannot access localhost - needs 10.0.2.2
  if (Platform.OS === 'android') {
    return 'http://10.0.2.2:8081';
  }
  return 'http://localhost:8081';
};

// Create the mobile platform adapter
const adapter = createMobileAdapter({
  asyncStorage: AsyncStorage,
  baseURL: getApiUrl(),
  timeout: 300000, // 5 minutes for slow LLM responses
});

// Create the full API service with all domain APIs
const api: PierreApiService = createPierreApi(adapter);

// Export the axios client for direct access (backward compatibility)
export const apiClient = api.axios;

// Export auth failure subscription function (backward compatibility)
export const onAuthFailure = (listener: () => void): (() => void) => {
  return adapter.authFailure.subscribe(listener);
};

// Export domain APIs for direct import
export const authApi = api.auth;
export const chatApi = api.chat;
export const coachesApi = api.coaches;
export const oauthApi = api.oauth;
export const socialApi = api.social;
export const storeApi = api.store;
export const userApi = api.user;

/**
 * Unified API service with mobile-compatible method signatures.
 *
 * This provides backward compatibility with existing code that imports apiService.
 * New code should prefer importing specific domain APIs directly.
 */
export const apiService = createLegacyApiService(adapter);
