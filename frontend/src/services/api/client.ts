// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: API client setup using @pierre/api-client shared package exclusively
// ABOUTME: Exports pierreApi instance and its axios for web-only domain modules

import { createPierreApi, type PierreApiService } from '@pierre/api-client';
import { createWebAdapter } from '@pierre/api-client/adapters/web';

// In development, use empty string to leverage Vite proxy (avoids CORS issues)
// In production, use VITE_API_BASE_URL environment variable
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

// Get base URL, falling back to localhost for test environments
const getBaseURL = () => {
  if (API_BASE_URL) return API_BASE_URL;
  if (typeof window !== 'undefined') return window.location.origin;
  return 'http://localhost:8081';
};

// Create Pierre API instance using shared package
const webAdapter = createWebAdapter({
  baseURL: getBaseURL(),
});
export const pierreApi: PierreApiService = createPierreApi(webAdapter);

// Export pierreApi's axios instance for web-only domain modules (admin, keys, dashboard, a2a, oauth, social).
// This instance has CSRF token and 401 interceptors configured by @pierre/api-client.
export const axios = pierreApi.axios;
