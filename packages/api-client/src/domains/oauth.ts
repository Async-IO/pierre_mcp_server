// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: OAuth domain API - provider connections and authorization
// ABOUTME: Handles OAuth flow initiation and connection status

import type { AxiosInstance } from 'axios';
import type {
  ProviderStatus,
  ExtendedProviderStatus,
  ProvidersStatusResponse,
  ApiMetadata,
} from '@pierre/shared-types';
import { ENDPOINTS } from '../core/endpoints';

// Re-export for consumers
export type { ProviderStatus, ExtendedProviderStatus, ProvidersStatusResponse };

// Extended provider status with additional OAuth details
export interface OAuthProvider extends ProviderStatus {
  connected_at?: string;
  expires_at?: string;
  scopes?: string[];
}

export interface OAuthStatusResponse {
  providers: ProviderStatus[];
  metadata: ApiMetadata;
}

export interface MobileOAuthInitResponse {
  authorization_url: string;
  state: string;
  code_verifier?: string;
  metadata: ApiMetadata;
}

/**
 * Creates the OAuth API methods bound to an axios instance.
 */
export function createOAuthApi(axios: AxiosInstance, getBaseUrl: () => string) {
  return {
    /**
     * Get the connection status of all OAuth providers.
     */
    async getStatus(): Promise<OAuthStatusResponse> {
      const response = await axios.get<OAuthStatusResponse>(ENDPOINTS.OAUTH.STATUS);
      return response.data;
    },

    /**
     * Get the OAuth authorization URL for a provider (web).
     */
    getAuthorizeUrl(provider: string, redirectUri?: string): string {
      const baseUrl = getBaseUrl();
      const url = new URL(`${baseUrl}${ENDPOINTS.OAUTH.AUTHORIZE(provider)}`);
      if (redirectUri) {
        url.searchParams.set('redirect_uri', redirectUri);
      }
      return url.toString();
    },

    /**
     * Initialize mobile OAuth flow for a provider.
     * Returns the authorization URL and PKCE parameters.
     */
    async initMobileOAuth(
      provider: string,
      redirectUri?: string
    ): Promise<MobileOAuthInitResponse> {
      const params = new URLSearchParams();
      if (redirectUri) {
        params.set('redirect_uri', redirectUri);
      }

      const queryString = params.toString();
      const url = queryString
        ? `${ENDPOINTS.OAUTH.MOBILE_INIT(provider)}?${queryString}`
        : ENDPOINTS.OAUTH.MOBILE_INIT(provider);

      const response = await axios.get<MobileOAuthInitResponse>(url);
      return response.data;
    },

    /**
     * Get all providers (OAuth and non-OAuth) with connection status.
     * Includes synthetic providers and other non-OAuth data sources.
     */
    async getProvidersStatus(): Promise<ProvidersStatusResponse> {
      const response = await axios.get<ProvidersStatusResponse>(ENDPOINTS.PROVIDERS.STATUS);
      return response.data;
    },

    // Aliases for backward compatibility
    getOAuthStatus() {
      return this.getStatus();
    },

    getOAuthAuthorizeUrl(provider: string, redirectUri?: string) {
      return this.getAuthorizeUrl(provider, redirectUri);
    },
  };
}

export type OAuthApi = ReturnType<typeof createOAuthApi>;
