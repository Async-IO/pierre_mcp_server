// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: OAuth provider connection API methods - status, authorization URLs
// ABOUTME: Handles third-party OAuth integrations like Strava

import { axios } from './client';

export const oauthApi = {
  async getOAuthStatus(): Promise<{
    providers: Array<{
      provider: string;
      connected: boolean;
      last_sync: string | null;
    }>;
  }> {
    const response = await axios.get('/api/oauth/status');
    // Backend returns array directly, wrap for consistency
    return { providers: response.data };
  },

  // Get OAuth authorization URL for a provider
  // Calls the mobile/init endpoint to get the authorization URL from the backend
  // This works through Vite's proxy in development
  async getOAuthAuthorizeUrl(provider: string): Promise<string> {
    const response = await axios.get(`/api/oauth/mobile/init/${provider}`);
    return response.data.authorization_url;
  },
};
