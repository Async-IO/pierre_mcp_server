// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: User profile and settings API methods - profile updates, MCP tokens, OAuth apps
// ABOUTME: Handles user-specific settings and third-party integrations

import { axios } from './client';

export const userApi = {
  async updateProfile(data: {
    display_name: string;
  }): Promise<{
    message: string;
    user: { id: string; email: string; display_name?: string };
  }> {
    const response = await axios.put('/api/user/profile', data);
    return response.data;
  },

  async getUserStats(): Promise<{
    connected_providers: number;
    days_active: number;
  }> {
    const response = await axios.get('/api/user/stats');
    return response.data;
  },

  async changePassword(data: {
    current_password: string;
    new_password: string;
  }): Promise<{ message: string }> {
    const response = await axios.put('/api/user/change-password', data);
    return response.data;
  },

  // MCP Token Management
  async createMcpToken(data: {
    name: string;
    expires_in_days?: number;
  }): Promise<{
    id: string;
    name: string;
    token_prefix: string;
    token_value: string;
    expires_at: string | null;
    created_at: string;
  }> {
    const response = await axios.post('/api/user/mcp-tokens', data);
    return response.data;
  },

  async getMcpTokens(): Promise<{
    tokens: Array<{
      id: string;
      name: string;
      token_prefix: string;
      expires_at: string | null;
      last_used_at: string | null;
      usage_count: number;
      is_revoked: boolean;
      created_at: string;
    }>;
  }> {
    const response = await axios.get('/api/user/mcp-tokens');
    return response.data;
  },

  async revokeMcpToken(tokenId: string): Promise<{ success: boolean }> {
    const response = await axios.delete(`/api/user/mcp-tokens/${tokenId}`);
    return response.data;
  },

  // OAuth App Credentials
  async getUserOAuthApps(): Promise<{
    apps: Array<{
      provider: string;
      client_id: string;
      redirect_uri: string;
      created_at: string;
    }>;
  }> {
    const response = await axios.get('/api/users/oauth-apps');
    return response.data;
  },

  async registerUserOAuthApp(data: {
    provider: string;
    client_id: string;
    client_secret: string;
    redirect_uri: string;
  }): Promise<{
    success: boolean;
    provider: string;
    message: string;
  }> {
    const response = await axios.post('/api/users/oauth-apps', data);
    return response.data;
  },

  async deleteUserOAuthApp(provider: string): Promise<void> {
    await axios.delete(`/api/users/oauth-apps/${provider}`);
  },

  // LLM Settings
  async getLlmSettings(): Promise<{
    current_provider: string | null;
    providers: Array<{
      name: string;
      display_name: string;
      has_credentials: boolean;
      credential_source: string | null;
      is_active: boolean;
    }>;
    user_credentials: Array<{
      id: string;
      provider: string;
      user_id: string | null;
      created_at: string;
      updated_at: string;
    }>;
    tenant_credentials: Array<{
      id: string;
      provider: string;
      user_id: string | null;
      created_at: string;
      updated_at: string;
    }>;
  }> {
    const response = await axios.get('/api/user/llm-settings');
    return response.data;
  },

  async saveLlmCredentials(data: {
    provider: string;
    api_key: string;
    base_url?: string;
    default_model?: string;
    scope?: 'user' | 'tenant';
  }): Promise<{
    success: boolean;
    id: string | null;
    message: string;
  }> {
    const response = await axios.put('/api/user/llm-settings', data);
    return response.data;
  },

  async validateLlmCredentials(data: {
    provider: string;
    api_key: string;
    base_url?: string;
  }): Promise<{
    valid: boolean;
    provider: string | null;
    models: string[] | null;
    error: string | null;
  }> {
    const response = await axios.post('/api/user/llm-settings/validate', data);
    return response.data;
  },

  async deleteLlmCredentials(provider: string): Promise<{
    success: boolean;
    message: string;
  }> {
    const response = await axios.delete(`/api/user/llm-settings/${provider}`);
    return response.data;
  },
};
