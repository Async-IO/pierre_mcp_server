// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Admin API methods - user management, tokens, config, coach management, store moderation
// ABOUTME: Handles all administrative functionality for super_admin and admin roles

import { axios } from './client';
import type { Coach } from './coaches';

export const adminApi = {
  // ==================== SETUP STATUS ====================
  async getSetupStatus() {
    const response = await axios.get('/admin/setup/status');
    return response.data;
  },

  // ==================== ADMIN TOKEN MANAGEMENT ====================
  async getAdminTokens(params?: { include_inactive?: boolean }) {
    const queryParams = new URLSearchParams();
    if (params?.include_inactive !== undefined) {
      queryParams.append('include_inactive', params.include_inactive.toString());
    }
    const queryString = queryParams.toString();
    const url = queryString ? `/api/admin/tokens?${queryString}` : '/api/admin/tokens';
    const response = await axios.get(url);
    return response.data;
  },

  async createAdminToken(data: {
    service_name: string;
    service_description?: string;
    permissions?: string[];
    is_super_admin?: boolean;
    expires_in_days?: number;
  }) {
    const response = await axios.post('/api/admin/tokens', data);
    return response.data;
  },

  async getAdminTokenDetails(tokenId: string) {
    const response = await axios.get(`/api/admin/tokens/${tokenId}`);
    return response.data;
  },

  async revokeAdminToken(tokenId: string) {
    const response = await axios.post(`/api/admin/tokens/${tokenId}/revoke`);
    return response.data;
  },

  async rotateAdminToken(tokenId: string, data?: { expires_in_days?: number }) {
    const response = await axios.post(`/api/admin/tokens/${tokenId}/rotate`, data || {});
    return response.data;
  },

  async getAdminTokenInfo() {
    const response = await axios.get('/admin/token-info');
    return response.data;
  },

  async getAdminHealth() {
    const response = await axios.get('/admin/health');
    return response.data;
  },

  async getAdminTokenAudit(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}/audit`);
    return response.data;
  },

  async getAdminTokenUsageStats(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}/usage-stats`);
    return response.data;
  },

  async getAdminTokenProvisionedKeys(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}/provisioned-keys`);
    return response.data;
  },

  // ==================== API KEY PROVISIONING ====================
  async provisionApiKey(data: {
    user_email: string;
    tier: string;
    description?: string;
    expires_in_days?: number;
    rate_limit_requests?: number;
    rate_limit_period?: string;
  }) {
    const response = await axios.post('/admin/provision-api-key', data);
    return response.data;
  },

  async revokeApiKey(data: { key_id?: string; user_email?: string }) {
    const response = await axios.post('/admin/revoke-api-key', data);
    return response.data;
  },

  async listApiKeys(params?: {
    user_email?: string;
    active_only?: boolean;
    limit?: number;
    offset?: number;
  }) {
    const queryParams = new URLSearchParams();
    if (params?.user_email) queryParams.append('user_email', params.user_email);
    if (params?.active_only !== undefined) queryParams.append('active_only', params.active_only.toString());
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.offset) queryParams.append('offset', params.offset.toString());

    const response = await axios.get(`/admin/list-api-keys?${queryParams}`);
    return response.data;
  },

  // ==================== TIER CONFIGURATION ====================
  async getTierDefaults(): Promise<Record<string, { daily_limit: number | null; monthly_limit: number | null }>> {
    return Promise.resolve({
      trial: { daily_limit: 100, monthly_limit: 1000 },
      starter: { daily_limit: 1000, monthly_limit: 10000 },
      professional: { daily_limit: 10000, monthly_limit: 100000 },
      enterprise: { daily_limit: null, monthly_limit: null },
    });
  },

  // ==================== USER MANAGEMENT ====================
  async getPendingUsers() {
    const response = await axios.get('/api/admin/pending-users');
    return response.data.users || [];
  },

  async approveUser(userId: string, reason?: string) {
    const response = await axios.post(`/api/admin/approve-user/${userId}`, { reason });
    return response.data;
  },

  async suspendUser(userId: string, reason?: string) {
    const response = await axios.post(`/api/admin/suspend-user/${userId}`, { reason });
    return response.data;
  },

  async getAllUsers(params?: {
    status?: 'pending' | 'active' | 'suspended';
    limit?: number;
    offset?: number;
  }) {
    const queryParams = new URLSearchParams();
    if (params?.status) queryParams.append('status', params.status);
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.offset) queryParams.append('offset', params.offset.toString());

    const queryString = queryParams.toString();
    const url = queryString ? `/api/admin/users?${queryString}` : '/api/admin/users';
    const response = await axios.get(url);
    return response.data.users || [];
  },

  async resetUserPassword(userId: string): Promise<{
    success: boolean;
    temporary_password: string;
    expires_at: string;
    user_email: string;
  }> {
    const response = await axios.post(`/api/admin/users/${userId}/reset-password`);
    return response.data;
  },

  async getUserRateLimit(userId: string): Promise<{
    user_id: string;
    tier: string;
    rate_limits: {
      daily: { limit: number | null; used: number; remaining: number | null };
      monthly: { limit: number | null; used: number; remaining: number | null };
    };
    reset_times: {
      daily_reset: string;
      monthly_reset: string;
    };
  }> {
    const response = await axios.get(`/api/admin/users/${userId}/rate-limit`);
    return response.data.data;
  },

  async getUserActivity(userId: string, days: number = 30): Promise<{
    user_id: string;
    period_days: number;
    total_requests: number;
    top_tools: Array<{
      tool_name: string;
      call_count: number;
      percentage: number;
    }>;
  }> {
    const response = await axios.get(`/api/admin/users/${userId}/activity?days=${days}`);
    return response.data.data;
  },

  // ==================== ADMIN SETTINGS ====================
  async getAutoApprovalSetting(): Promise<{ enabled: boolean; description: string }> {
    const response = await axios.get('/api/admin/settings/auto-approval');
    return response.data.data;
  },

  async updateAutoApprovalSetting(enabled: boolean): Promise<{ enabled: boolean; description: string }> {
    const response = await axios.put('/api/admin/settings/auto-approval', { enabled });
    return response.data.data;
  },

  // ==================== CONFIGURATION MANAGEMENT ====================
  async getConfigCatalog(tenantId?: string): Promise<{
    success: boolean;
    data: {
      total_parameters: number;
      runtime_configurable_count: number;
      static_count: number;
      categories: Array<{
        id: string;
        name: string;
        display_name: string;
        description: string;
        display_order: number;
        is_active: boolean;
        parameters: Array<{
          key: string;
          display_name: string;
          description: string;
          category: string;
          data_type: string;
          current_value: unknown;
          default_value: unknown;
          is_modified: boolean;
          valid_range?: { min?: number; max?: number; step?: number };
          enum_options?: string[];
          units?: string;
          scientific_basis?: string;
          is_runtime_configurable: boolean;
          requires_restart: boolean;
        }>;
      }>;
    };
  }> {
    const params = new URLSearchParams();
    if (tenantId) params.append('tenant_id', tenantId);
    const queryString = params.toString();
    const url = queryString ? `/api/admin/config/catalog?${queryString}` : '/api/admin/config/catalog';
    const response = await axios.get(url);
    return response.data;
  },

  async getConfigAuditLog(params?: {
    category?: string;
    config_key?: string;
    admin_user_id?: string;
    tenant_id?: string;
    limit?: number;
    offset?: number;
  }): Promise<{
    success: boolean;
    data: {
      entries: Array<{
        id: string;
        timestamp: string;
        admin_user_id: string;
        admin_email: string;
        category: string;
        config_key: string;
        old_value: unknown;
        new_value: unknown;
        data_type: string;
        reason?: string;
        tenant_id?: string;
        ip_address?: string;
        user_agent?: string;
      }>;
      total_count: number;
      offset: number;
      limit: number;
    };
  }> {
    const queryParams = new URLSearchParams();
    if (params?.category) queryParams.append('category', params.category);
    if (params?.config_key) queryParams.append('config_key', params.config_key);
    if (params?.admin_user_id) queryParams.append('admin_user_id', params.admin_user_id);
    if (params?.tenant_id) queryParams.append('tenant_id', params.tenant_id);
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.offset) queryParams.append('offset', params.offset.toString());

    const queryString = queryParams.toString();
    const url = queryString ? `/api/admin/config/audit?${queryString}` : '/api/admin/config/audit';
    const response = await axios.get(url);
    return response.data;
  },

  async updateConfig(request: {
    parameters: Record<string, unknown>;
    reason?: string;
  }, tenantId?: string): Promise<{
    success: boolean;
    data: {
      updated_count: number;
      requires_restart: boolean;
      validation_errors?: Array<{ parameter: string; error: string }>;
    };
  }> {
    const params = new URLSearchParams();
    if (tenantId) params.append('tenant_id', tenantId);
    const queryString = params.toString();
    const url = queryString ? `/api/admin/config?${queryString}` : '/api/admin/config';
    const response = await axios.put(url, request);
    return response.data;
  },

  async resetConfig(request: {
    category?: string;
    parameters?: string[];
  }, tenantId?: string): Promise<{ success: boolean; data: { reset_count: number } }> {
    const params = new URLSearchParams();
    if (tenantId) params.append('tenant_id', tenantId);
    const queryString = params.toString();
    const url = queryString ? `/api/admin/config/reset?${queryString}` : '/api/admin/config/reset';
    const response = await axios.post(url, request);
    return response.data;
  },

  // ==================== IMPERSONATION ====================
  async startImpersonation(targetUserId: string, reason?: string): Promise<{
    success: boolean;
    session_id: string;
    token: string;
    target_user: {
      id: string;
      email: string;
      display_name?: string;
      role: string;
    };
    message: string;
  }> {
    const response = await axios.post('/api/admin/impersonate', {
      target_user_id: targetUserId,
      reason,
    });
    return response.data;
  },

  async endImpersonation(): Promise<{
    success: boolean;
    message: string;
    session_id: string;
    duration_seconds: number;
  }> {
    const response = await axios.post('/api/admin/impersonate/end');
    return response.data;
  },

  async getImpersonationSessions(): Promise<{
    sessions: Array<{
      id: string;
      impersonator_id: string;
      impersonator_email?: string;
      target_user_id: string;
      target_user_email?: string;
      reason?: string;
      started_at: string;
      ended_at?: string;
      is_active: boolean;
      duration_seconds: number;
    }>;
    total_count: number;
  }> {
    const response = await axios.get('/api/admin/impersonate/sessions');
    return response.data;
  },

  // ==================== SYSTEM COACHES ====================
  async getSystemCoaches(): Promise<{
    coaches: Coach[];
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/admin/coaches');
    return response.data;
  },

  async createSystemCoach(data: {
    title: string;
    description?: string;
    system_prompt: string;
    category?: string;
    tags?: string[];
    visibility?: string;
  }): Promise<Coach> {
    const response = await axios.post('/api/admin/coaches', data);
    return response.data;
  },

  async getSystemCoach(coachId: string): Promise<Coach> {
    const response = await axios.get(`/api/admin/coaches/${coachId}`);
    return response.data;
  },

  async updateSystemCoach(coachId: string, data: {
    title?: string;
    description?: string;
    system_prompt?: string;
    category?: string;
    tags?: string[];
  }): Promise<Coach> {
    const response = await axios.put(`/api/admin/coaches/${coachId}`, data);
    return response.data;
  },

  async deleteSystemCoach(coachId: string): Promise<void> {
    await axios.delete(`/api/admin/coaches/${coachId}`);
  },

  async assignCoachToUsers(coachId: string, userIds: string[]): Promise<{
    coach_id: string;
    assigned_count: number;
    total_requested: number;
  }> {
    const response = await axios.post(`/api/admin/coaches/${coachId}/assign`, { user_ids: userIds });
    return response.data;
  },

  async unassignCoachFromUsers(coachId: string, userIds: string[]): Promise<{
    coach_id: string;
    removed_count: number;
    total_requested: number;
  }> {
    const response = await axios.delete(`/api/admin/coaches/${coachId}/assign`, {
      data: { user_ids: userIds },
    });
    return response.data;
  },

  async getCoachAssignments(coachId: string): Promise<{
    coach_id: string;
    assignments: Array<{
      user_id: string;
      user_email?: string;
      assigned_at: string;
      assigned_by?: string;
    }>;
  }> {
    const response = await axios.get(`/api/admin/coaches/${coachId}/assignments`);
    return response.data;
  },

  // ==================== TOOL SELECTION ====================
  async getToolCatalog(): Promise<{
    success: boolean;
    message: string;
    data: Array<{
      tool_name: string;
      display_name: string;
      description: string;
      category: string;
      default_enabled: boolean;
      is_globally_disabled: boolean;
      available_in_tiers: string[];
    }>;
  }> {
    const response = await axios.get('/api/admin/tools/catalog');
    return response.data;
  },

  async getToolCatalogEntry(toolName: string): Promise<{
    success: boolean;
    message: string;
    data: {
      tool_name: string;
      display_name: string;
      description: string;
      category: string;
      default_enabled: boolean;
      is_globally_disabled: boolean;
      available_in_tiers: string[];
    };
  }> {
    const response = await axios.get(`/api/admin/tools/catalog/${toolName}`);
    return response.data;
  },

  async getGlobalDisabledTools(): Promise<{
    success: boolean;
    message: string;
    data: { disabled_tools: string[]; count: number };
  }> {
    const response = await axios.get('/api/admin/tools/global-disabled');
    return response.data;
  },

  async getTenantTools(tenantId: string): Promise<{
    success: boolean;
    message: string;
    data: Array<{
      tool_name: string;
      display_name: string;
      description: string;
      category: string;
      is_enabled: boolean;
      source: string;
      min_plan: string;
    }>;
  }> {
    const response = await axios.get(`/api/admin/tools/tenant/${tenantId}`);
    return response.data;
  },

  async setToolOverride(
    tenantId: string,
    toolName: string,
    isEnabled: boolean,
    reason?: string
  ): Promise<{
    success: boolean;
    message: string;
    data: {
      tool_name: string;
      tenant_id: string;
      is_enabled: boolean;
      created_by: string;
      reason?: string;
      created_at: string;
    };
  }> {
    const response = await axios.post(`/api/admin/tools/tenant/${tenantId}/override`, {
      tool_name: toolName,
      is_enabled: isEnabled,
      reason,
    });
    return response.data;
  },

  async removeToolOverride(tenantId: string, toolName: string): Promise<{
    success: boolean;
    message: string;
  }> {
    const response = await axios.delete(`/api/admin/tools/tenant/${tenantId}/override/${toolName}`);
    return response.data;
  },

  async getToolAvailabilitySummary(tenantId: string): Promise<{
    success: boolean;
    message: string;
    data: {
      tenant_id: string;
      total_tools: number;
      enabled_tools: number;
      disabled_tools: number;
      overridden_tools: number;
      globally_disabled_count: number;
      plan_restricted_count: number;
    };
  }> {
    const response = await axios.get(`/api/admin/tools/tenant/${tenantId}/summary`);
    return response.data;
  },

  // ==================== STORE MANAGEMENT ====================
  async getStoreReviewQueue(): Promise<{
    coaches: Array<{
      id: string;
      title: string;
      description: string | null;
      category: string;
      tags: string[];
      sample_prompts: string[];
      token_count: number;
      install_count: number;
      icon_url: string | null;
      published_at: string | null;
      author_id: string | null;
      author_email?: string;
      system_prompt: string;
      created_at: string;
      submitted_at: string;
      publish_status: string;
    }>;
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/admin/store/coaches?status=pending_review');
    return response.data;
  },

  async getPublishedStoreCoaches(options?: {
    sort_by?: 'newest' | 'most_installed';
    limit?: number;
    offset?: number;
  }): Promise<{
    coaches: Array<{
      id: string;
      title: string;
      description: string | null;
      category: string;
      tags: string[];
      sample_prompts: string[];
      token_count: number;
      install_count: number;
      icon_url: string | null;
      published_at: string | null;
      author_id: string | null;
      author_email?: string;
      system_prompt: string;
      created_at: string;
      publish_status: string;
    }>;
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    params.append('status', 'published');
    if (options?.sort_by) params.append('sort_by', options.sort_by);
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.offset) params.append('offset', options.offset.toString());
    const response = await axios.get(`/api/admin/store/coaches?${params}`);
    return response.data;
  },

  async getRejectedStoreCoaches(options?: {
    limit?: number;
    offset?: number;
  }): Promise<{
    coaches: Array<{
      id: string;
      title: string;
      description: string | null;
      category: string;
      tags: string[];
      sample_prompts: string[];
      token_count: number;
      install_count: number;
      icon_url: string | null;
      published_at: string | null;
      author_id: string | null;
      author_email?: string;
      system_prompt: string;
      created_at: string;
      rejected_at: string;
      rejection_reason: string;
      rejection_notes?: string;
      publish_status: string;
    }>;
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    params.append('status', 'rejected');
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.offset) params.append('offset', options.offset.toString());
    const response = await axios.get(`/api/admin/store/coaches?${params}`);
    return response.data;
  },

  async getStoreStats(): Promise<{
    pending_count: number;
    published_count: number;
    rejected_count: number;
    total_installs: number;
    rejection_rate: number;
  }> {
    const response = await axios.get('/api/admin/store/stats');
    return response.data;
  },

  async approveStoreCoach(coachId: string): Promise<{
    success: boolean;
    message: string;
    coach_id: string;
  }> {
    const response = await axios.post(`/api/admin/store/coaches/${coachId}/approve`);
    return response.data;
  },

  async rejectStoreCoach(coachId: string, reason: string, notes?: string): Promise<{
    success: boolean;
    message: string;
    coach_id: string;
  }> {
    const response = await axios.post(`/api/admin/store/coaches/${coachId}/reject`, { reason, notes });
    return response.data;
  },

  async unpublishStoreCoach(coachId: string): Promise<{
    success: boolean;
    message: string;
    coach_id: string;
  }> {
    const response = await axios.post(`/api/admin/store/coaches/${coachId}/unpublish`);
    return response.data;
  },
};
