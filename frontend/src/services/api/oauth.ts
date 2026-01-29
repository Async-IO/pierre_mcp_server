// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: OAuth and provider connection API methods - status, authorization URLs
// ABOUTME: Handles third-party OAuth integrations and provider status

import { axios } from './client';

/** Provider status from the server */
export interface ProviderStatus {
  provider: string;
  display_name: string;
  requires_oauth: boolean;
  connected: boolean;
  capabilities: string[];
}

/** Response from /api/providers endpoint */
export interface ProvidersStatusResponse {
  providers: ProviderStatus[];
}

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

  /**
   * Get all available providers with connection status
   *
   * This is the unified endpoint that returns both OAuth and non-OAuth providers
   * (like synthetic). Use this instead of getOAuthStatus for the provider list.
   */
  async getProvidersStatus(): Promise<ProvidersStatusResponse> {
    const response = await axios.get<ProvidersStatusResponse>('/api/providers');
    return response.data;
  },

  // Get OAuth authorization URL for a provider
  // Calls the mobile/init endpoint to get the authorization URL from the backend
  // This works through Vite's proxy in development
  async getOAuthAuthorizeUrl(provider: string): Promise<string> {
    const response = await axios.get(`/api/oauth/mobile/init/${provider}`);
    return response.data.authorization_url;
  },
};
