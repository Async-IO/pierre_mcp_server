// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: User domain API - profile, stats, MCP tokens, OAuth apps
// ABOUTME: Handles user account management and settings

import type { AxiosInstance } from 'axios';
import type { User, McpToken, OAuthApp, OAuthAppCredentials } from '@pierre/shared-types';
import { ENDPOINTS } from '../core/endpoints';

// Re-export types for consumers
export type { User, McpToken };

// Types - aligned with actual backend responses

export interface UserStats {
  connected_providers: number;
  days_active: number;
}

export interface McpTokensResponse {
  tokens: McpToken[];
}

export interface CreateMcpTokenRequest {
  name: string;
  expires_in_days?: number;
}

// Use OAuthApp from shared-types, re-export as UserOAuthApp for backward compat
export type UserOAuthApp = OAuthApp;

export interface UserOAuthAppsResponse {
  apps: UserOAuthApp[];
}

export interface LlmSettings {
  provider: string;
  api_key?: string;
  model?: string;
  enabled: boolean;
}

export interface LlmSettingsResponse {
  settings: LlmSettings[];
}

export interface UpdateProfileResponse {
  message: string;
  user: { id: string; email: string; display_name?: string };
}

/**
 * Creates the user API methods bound to an axios instance.
 */
export function createUserApi(axios: AxiosInstance) {
  return {
    /**
     * Get user profile.
     */
    async getProfile(): Promise<User> {
      const response = await axios.get<User>(ENDPOINTS.USER.PROFILE);
      return response.data;
    },

    /**
     * Update user profile.
     */
    async updateProfile(data: { display_name: string }): Promise<UpdateProfileResponse> {
      const response = await axios.put<UpdateProfileResponse>(ENDPOINTS.USER.PROFILE, data);
      return response.data;
    },

    /**
     * Get user stats.
     */
    async getStats(): Promise<UserStats> {
      const response = await axios.get<UserStats>(ENDPOINTS.USER.STATS);
      return response.data;
    },

    /**
     * Change password.
     */
    async changePassword(currentPassword: string, newPassword: string): Promise<{ message: string }> {
      const response = await axios.put<{ message: string }>(ENDPOINTS.USER.CHANGE_PASSWORD, {
        current_password: currentPassword,
        new_password: newPassword,
      });
      return response.data;
    },

    // ==================== MCP TOKENS ====================

    /**
     * List MCP tokens.
     */
    async getMcpTokens(): Promise<McpTokensResponse> {
      const response = await axios.get<McpTokensResponse>(ENDPOINTS.USER.MCP_TOKENS);
      return response.data;
    },

    /**
     * Create a new MCP token.
     */
    async createMcpToken(request: CreateMcpTokenRequest): Promise<McpToken> {
      const response = await axios.post<McpToken>(ENDPOINTS.USER.MCP_TOKENS, request);
      return response.data;
    },

    /**
     * Revoke an MCP token.
     */
    async revokeMcpToken(tokenId: string): Promise<{ success: boolean }> {
      const response = await axios.delete<{ success: boolean }>(ENDPOINTS.USER.MCP_TOKEN(tokenId));
      return response.data;
    },

    // ==================== OAUTH APPS ====================

    /**
     * Get user's registered OAuth apps.
     */
    async getOAuthApps(): Promise<UserOAuthAppsResponse> {
      const response = await axios.get<UserOAuthAppsResponse>(ENDPOINTS.USER.OAUTH_APPS);
      return response.data;
    },

    /**
     * Register an OAuth app.
     */
    async registerOAuthApp(credentials: OAuthAppCredentials): Promise<{
      success: boolean;
      provider: string;
      message: string;
    }> {
      const response = await axios.post<{
        success: boolean;
        provider: string;
        message: string;
      }>(ENDPOINTS.USER.OAUTH_APPS, credentials);
      return response.data;
    },

    /**
     * Delete an OAuth app.
     */
    async deleteOAuthApp(provider: string): Promise<void> {
      await axios.delete(ENDPOINTS.USER.OAUTH_APP(provider));
    },

    // ==================== LLM SETTINGS ====================

    /**
     * Get LLM settings.
     */
    async getLlmSettings(): Promise<LlmSettingsResponse> {
      const response = await axios.get<LlmSettingsResponse>(ENDPOINTS.USER.LLM_SETTINGS);
      return response.data;
    },

    /**
     * Update LLM settings for a provider.
     */
    async updateLlmSettings(
      provider: string,
      settings: Partial<LlmSettings>
    ): Promise<LlmSettings> {
      const response = await axios.put<LlmSettings>(
        ENDPOINTS.USER.LLM_SETTINGS_PROVIDER(provider),
        settings
      );
      return response.data;
    },

    /**
     * Validate LLM settings.
     */
    async validateLlmSettings(
      provider: string,
      apiKey: string
    ): Promise<{ valid: boolean; error?: string }> {
      const response = await axios.post<{ valid: boolean; error?: string }>(
        ENDPOINTS.USER.LLM_SETTINGS_VALIDATE,
        { provider, api_key: apiKey }
      );
      return response.data;
    },

    // Aliases for backward compatibility
    getUserStats() {
      return this.getStats();
    },

    getUserOAuthApps() {
      return this.getOAuthApps();
    },

    registerUserOAuthApp(credentials: OAuthAppCredentials) {
      return this.registerOAuthApp(credentials);
    },

    deleteUserOAuthApp(provider: string) {
      return this.deleteOAuthApp(provider);
    },
  };
}

export type UserApi = ReturnType<typeof createUserApi>;
