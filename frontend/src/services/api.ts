// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import axios, { AxiosError, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';

// In development, use empty string to leverage Vite proxy (avoids CORS issues)
// In production, use VITE_API_BASE_URL environment variable
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

class ApiService {
  private csrfToken: string | null = null;

  constructor() {
    // Set up axios defaults
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.headers.common['Content-Type'] = 'application/json';

    // Enable sending cookies with requests
    axios.defaults.withCredentials = true;

    // Set up response interceptor for error handling
    this.setupInterceptors();
  }

  private setupInterceptors() {
    // Request interceptor to add CSRF token
    axios.interceptors.request.use(
      (config: InternalAxiosRequestConfig) => {
        // Add CSRF token for state-changing operations
        if (this.csrfToken && config.headers && ['POST', 'PUT', 'DELETE', 'PATCH'].includes(config.method?.toUpperCase() || '')) {
          config.headers['X-CSRF-Token'] = this.csrfToken;
        }
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor to handle 401 errors
    axios.interceptors.response.use(
      (response: AxiosResponse) => response,
      async (error: AxiosError) => {
        if (error.response?.status === 401) {
          // Authentication failed, trigger logout
          this.handleAuthFailure();
        }
        return Promise.reject(error);
      }
    );
  }

  private handleAuthFailure() {
    // Clear CSRF token
    this.csrfToken = null;

    // Trigger logout event for components to react
    window.dispatchEvent(new CustomEvent('auth-failure'));

    // Redirect to login if not already there
    if (!window.location.pathname.includes('/login')) {
      window.location.href = '/login';
    }
  }

  async login(email: string, password: string) {
    // OAuth2 ROPC endpoint requires form-encoded body per RFC 6749
    const params = new URLSearchParams();
    params.append('grant_type', 'password');
    params.append('username', email);
    params.append('password', password);

    const response = await axios.post('/oauth/token', params, {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
    });
    return response.data;
  }

  async loginWithFirebase(idToken: string) {
    const response = await axios.post('/api/auth/firebase', { id_token: idToken });
    return response.data;
  }

  async logout() {
    try {
      // Call logout endpoint to clear httpOnly cookies
      await axios.post('/api/auth/logout');
    } catch (error) {
      console.error('Logout API call failed:', error);
      // Don't throw - allow local cleanup to continue
    }
  }

  async register(email: string, password: string, displayName?: string) {
    const response = await axios.post('/api/auth/register', {
      email,
      password,
      display_name: displayName,
    });
    return response.data;
  }

  async refreshToken() {
    const response = await axios.post('/api/auth/refresh');
    return response.data;
  }

  // API Key Management
  async createApiKey(data: {
    name: string;
    description?: string;
    rate_limit_requests: number; // 0 = unlimited
    expires_in_days?: number;
  }) {
    const response = await axios.post('/api/keys', data);
    return response.data;
  }

  async createTrialKey(data: {
    name: string;
    description?: string;
  }) {
    // Create trial key with 1000 requests/month and 14-day expiry
    const trialData = {
      name: data.name,
      description: data.description,
      rate_limit_requests: 1000,
      expires_in_days: 14,
    };
    const response = await axios.post('/api/keys', trialData);
    return response.data;
  }

  async getApiKeys() {
    const response = await axios.get('/api/keys');
    return response.data;
  }

  async deactivateApiKey(keyId: string) {
    const response = await axios.delete(`/api/keys/${keyId}`);
    return response.data;
  }

  async getApiKeyUsage(keyId: string, startDate?: string, endDate?: string) {
    const params = new URLSearchParams();
    if (startDate) params.append('start_date', startDate);
    if (endDate) params.append('end_date', endDate);
    
    const response = await axios.get(`/api/keys/${keyId}/usage?${params}`);
    return response.data;
  }

  // Dashboard endpoints
  async getDashboardOverview() {
    const response = await axios.get('/api/dashboard/overview');
    return response.data;
  }

  async getUsageAnalytics(days: number = 30) {
    const response = await axios.get(`/api/dashboard/analytics?days=${days}`);
    return response.data;
  }

  async getRateLimitOverview() {
    const response = await axios.get('/api/dashboard/rate-limits');
    return response.data;
  }

  // Request monitoring endpoints
  async getRequestLogs(apiKeyId?: string, filter?: {
    timeRange: string;
    status: string;
    tool: string;
  }) {
    const params = new URLSearchParams();
    if (apiKeyId) params.append('api_key_id', apiKeyId);
    if (filter?.timeRange) params.append('time_range', filter.timeRange);
    if (filter?.status && filter.status !== 'all') params.append('status', filter.status);
    if (filter?.tool && filter.tool !== 'all') params.append('tool', filter.tool);
    
    const response = await axios.get(`/api/dashboard/request-logs?${params}`);
    return response.data;
  }

  async getRequestStats(apiKeyId?: string, timeRange: string = '1h') {
    const params = new URLSearchParams();
    if (apiKeyId) params.append('api_key_id', apiKeyId);
    params.append('time_range', timeRange);
    
    const response = await axios.get(`/api/dashboard/request-stats?${params}`);
    return response.data;
  }

  async getToolUsageBreakdown(apiKeyId?: string, timeRange: string = '7d') {
    const params = new URLSearchParams();
    if (apiKeyId) params.append('api_key_id', apiKeyId);
    params.append('time_range', timeRange);
    
    const response = await axios.get(`/api/dashboard/tool-usage?${params}`);
    return response.data;
  }


  // CSRF Token management
  getCsrfToken(): string | null {
    return this.csrfToken;
  }

  setCsrfToken(token: string) {
    this.csrfToken = token;
  }

  clearCsrfToken() {
    this.csrfToken = null;
  }

  // User info management (still using localStorage for user data, not auth tokens)
  getUser(): { id: string; email: string; display_name?: string } | null {
    const user = localStorage.getItem('user');
    return user ? JSON.parse(user) : null;
  }

  setUser(user: { id: string; email: string; display_name?: string }) {
    localStorage.setItem('user', JSON.stringify(user));
  }

  clearUser() {
    localStorage.removeItem('user');
  }

  // A2A (Agent-to-Agent) Protocol endpoints
  async registerA2AClient(data: {
    name: string;
    description: string;
    capabilities: string[];
    redirect_uris?: string[];
    contact_email: string;
    agent_version?: string;
    documentation_url?: string;
  }) {
    const response = await axios.post('/a2a/clients', data);
    return response.data;
  }

  async getA2AClients() {
    const response = await axios.get('/a2a/clients');
    return response.data;
  }

  async getA2AClient(clientId: string) {
    const response = await axios.get(`/a2a/clients/${clientId}`);
    return response.data;
  }

  async updateA2AClient(clientId: string, data: {
    name?: string;
    description?: string;
    capabilities?: string[];
    redirect_uris?: string[];
    contact_email?: string;
    agent_version?: string;
    documentation_url?: string;
  }) {
    const response = await axios.put(`/a2a/clients/${clientId}`, data);
    return response.data;
  }

  async deactivateA2AClient(clientId: string) {
    const response = await axios.delete(`/a2a/clients/${clientId}`);
    return response.data;
  }

  async getA2AClientUsage(clientId: string, startDate?: string, endDate?: string) {
    const params = new URLSearchParams();
    if (startDate) params.append('start_date', startDate);
    if (endDate) params.append('end_date', endDate);
    
    const response = await axios.get(`/a2a/clients/${clientId}/usage?${params}`);
    return response.data;
  }

  async getA2AClientRateLimit(clientId: string) {
    const response = await axios.get(`/a2a/clients/${clientId}/rate-limit`);
    return response.data;
  }

  async getA2ASessions(clientId?: string) {
    const params = new URLSearchParams();
    if (clientId) params.append('client_id', clientId);
    
    const response = await axios.get(`/a2a/sessions?${params}`);
    return response.data;
  }

  async getA2ADashboardOverview() {
    const response = await axios.get('/a2a/dashboard/overview');
    return response.data;
  }

  async getA2AUsageAnalytics(days: number = 30) {
    const response = await axios.get(`/a2a/dashboard/analytics?days=${days}`);
    return response.data;
  }

  async getA2AAgentCard() {
    const response = await axios.get('/a2a/agent-card');
    return response.data;
  }

  async getA2ARequestLogs(clientId?: string, filter?: {
    timeRange: string;
    status: string;
    tool: string;
  }) {
    const params = new URLSearchParams();
    if (clientId) params.append('client_id', clientId);
    if (filter?.timeRange) params.append('time_range', filter.timeRange);
    if (filter?.status && filter.status !== 'all') params.append('status', filter.status);
    if (filter?.tool && filter.tool !== 'all') params.append('tool', filter.tool);
    
    const response = await axios.get(`/a2a/dashboard/request-logs?${params}`);
    return response.data;
  }

  // Setup status endpoint (no authentication required)
  async getSetupStatus() {
    const response = await axios.get('/admin/setup/status');
    return response.data;
  }

  // Admin Token Management endpoints
  async getAdminTokens(params?: {
    include_inactive?: boolean;
  }) {
    const queryParams = new URLSearchParams();
    if (params?.include_inactive !== undefined) {
      queryParams.append('include_inactive', params.include_inactive.toString());
    }

    // Use /api/admin/tokens for JWT cookie auth (web admin), fall back to /admin/tokens for admin token auth
    const queryString = queryParams.toString();
    const url = queryString ? `/api/admin/tokens?${queryString}` : '/api/admin/tokens';
    const response = await axios.get(url);
    return response.data;
  }

  async createAdminToken(data: {
    service_name: string;
    service_description?: string;
    permissions?: string[];
    is_super_admin?: boolean;
    expires_in_days?: number;
  }) {
    const response = await axios.post('/api/admin/tokens', data);
    return response.data;
  }

  async getAdminTokenDetails(tokenId: string) {
    const response = await axios.get(`/api/admin/tokens/${tokenId}`);
    return response.data;
  }

  async revokeAdminToken(tokenId: string) {
    const response = await axios.post(`/api/admin/tokens/${tokenId}/revoke`);
    return response.data;
  }

  async rotateAdminToken(tokenId: string, data?: {
    expires_in_days?: number;
  }) {
    const response = await axios.post(`/api/admin/tokens/${tokenId}/rotate`, data || {});
    return response.data;
  }

  // Admin API Key Provisioning endpoints (for admins to provision API keys for users)
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
  }

  async revokeApiKey(data: {
    key_id?: string;
    user_email?: string;
  }) {
    const response = await axios.post('/admin/revoke-api-key', data);
    return response.data;
  }

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
  }

  async getAdminTokenInfo() {
    const response = await axios.get('/admin/token-info');
    return response.data;
  }

  async getAdminHealth() {
    const response = await axios.get('/admin/health');
    return response.data;
  }

  // Tier configuration - returns system-wide tier limits
  async getTierDefaults(): Promise<Record<string, { daily_limit: number | null; monthly_limit: number | null }>> {
    // Return the default tier limits configured in the system
    // These match the Rust backend tier configurations
    return Promise.resolve({
      trial: { daily_limit: 100, monthly_limit: 1000 },
      starter: { daily_limit: 1000, monthly_limit: 10000 },
      professional: { daily_limit: 10000, monthly_limit: 100000 },
      enterprise: { daily_limit: null, monthly_limit: null },
    });
  }

  async getAdminTokenAudit(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}/audit`);
    return response.data;
  }

  async getAdminTokenUsageStats(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}/usage-stats`);
    return response.data;
  }

  async getAdminTokenProvisionedKeys(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}/provisioned-keys`);
    return response.data;
  }

  // User Management Endpoints
  async getPendingUsers() {
    const response = await axios.get('/api/admin/pending-users');
    // Backend returns { count, users } - extract users array for component compatibility
    return response.data.users || [];
  }

  async approveUser(userId: string, reason?: string) {
    const response = await axios.post(`/api/admin/approve-user/${userId}`, {
      reason
    });
    return response.data;
  }

  async suspendUser(userId: string, reason?: string) {
    const response = await axios.post(`/api/admin/suspend-user/${userId}`, {
      reason
    });
    return response.data;
  }

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
    // Backend returns { users: [...], total_count: n } - extract users array for component compatibility
    return response.data.users || [];
  }

  async resetUserPassword(userId: string): Promise<{
    success: boolean;
    temporary_password: string;
    expires_at: string;
    user_email: string;
  }> {
    const response = await axios.post(`/api/admin/users/${userId}/reset-password`);
    return response.data;
  }

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
    // Backend wraps response in AdminResponse {success, message, data}
    return response.data.data;
  }

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
    // Backend wraps response in AdminResponse {success, message, data}
    return response.data.data;
  }

  // Admin Settings Endpoints
  async getAutoApprovalSetting(): Promise<{
    enabled: boolean;
    description: string;
  }> {
    const response = await axios.get('/api/admin/settings/auto-approval');
    // Backend wraps response in AdminResponse {success, message, data}
    return response.data.data;
  }

  async updateAutoApprovalSetting(enabled: boolean): Promise<{
    enabled: boolean;
    description: string;
  }> {
    const response = await axios.put('/api/admin/settings/auto-approval', { enabled });
    // Backend wraps response in AdminResponse {success, message, data}
    return response.data.data;
  }

  // Admin Configuration Management endpoints
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
  }

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
  }

  async updateConfig(request: {
    parameters: Record<string, unknown>;
    reason?: string;
  }, tenantId?: string): Promise<{
    success: boolean;
    data: {
      updated_count: number;
      requires_restart: boolean;
      validation_errors?: Array<{
        parameter: string;
        error: string;
      }>;
    };
  }> {
    const params = new URLSearchParams();
    if (tenantId) params.append('tenant_id', tenantId);
    const queryString = params.toString();
    const url = queryString ? `/api/admin/config?${queryString}` : '/api/admin/config';
    const response = await axios.put(url, request);
    return response.data;
  }

  async resetConfig(request: {
    category?: string;
    parameters?: string[];
  }, tenantId?: string): Promise<{
    success: boolean;
    data: {
      reset_count: number;
    };
  }> {
    const params = new URLSearchParams();
    if (tenantId) params.append('tenant_id', tenantId);
    const queryString = params.toString();
    const url = queryString ? `/api/admin/config/reset?${queryString}` : '/api/admin/config/reset';
    const response = await axios.post(url, request);
    return response.data;
  }

  // Impersonation endpoints (super admin only)
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
  }

  async endImpersonation(): Promise<{
    success: boolean;
    message: string;
    session_id: string;
    duration_seconds: number;
  }> {
    const response = await axios.post('/api/admin/impersonate/end');
    return response.data;
  }

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
  }

  // User MCP Token Management endpoints
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
  }

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
  }

  async revokeMcpToken(tokenId: string): Promise<{ success: boolean }> {
    const response = await axios.delete(`/api/user/mcp-tokens/${tokenId}`);
    return response.data;
  }

  async updateProfile(data: {
    display_name: string;
  }): Promise<{
    message: string;
    user: { id: string; email: string; display_name?: string };
  }> {
    const response = await axios.put('/api/user/profile', data);
    return response.data;
  }

  async getUserStats(): Promise<{
    connected_providers: number;
    days_active: number;
  }> {
    const response = await axios.get('/api/user/stats');
    return response.data;
  }

  // Chat Conversations endpoints
  async getConversations(limit: number = 50, offset: number = 0): Promise<{
    conversations: Array<{
      id: string;
      title: string;
      model: string;
      system_prompt?: string;
      total_tokens: number;
      message_count: number;
      created_at: string;
      updated_at: string;
    }>;
    total: number;
    limit: number;
    offset: number;
  }> {
    const response = await axios.get(`/api/chat/conversations?limit=${limit}&offset=${offset}`);
    return response.data;
  }

  async createConversation(data: {
    title: string;
    model?: string;
    system_prompt?: string;
  }): Promise<{
    id: string;
    title: string;
    model: string;
    system_prompt?: string;
    total_tokens: number;
    created_at: string;
    updated_at: string;
  }> {
    const response = await axios.post('/api/chat/conversations', data);
    return response.data;
  }

  async getConversation(conversationId: string): Promise<{
    id: string;
    title: string;
    model: string;
    system_prompt?: string;
    total_tokens: number;
    message_count: number;
    created_at: string;
    updated_at: string;
  }> {
    const response = await axios.get(`/api/chat/conversations/${conversationId}`);
    return response.data;
  }

  async updateConversation(conversationId: string, data: {
    title?: string;
  }): Promise<{
    id: string;
    title: string;
    model: string;
    system_prompt?: string;
    total_tokens: number;
    created_at: string;
    updated_at: string;
  }> {
    const response = await axios.put(`/api/chat/conversations/${conversationId}`, data);
    return response.data;
  }

  async deleteConversation(conversationId: string): Promise<void> {
    await axios.delete(`/api/chat/conversations/${conversationId}`);
  }

  async getConversationMessages(conversationId: string): Promise<{
    messages: Array<{
      id: string;
      role: 'user' | 'assistant' | 'system';
      content: string;
      token_count?: number;
      created_at: string;
    }>;
  }> {
    const response = await axios.get(`/api/chat/conversations/${conversationId}/messages`);
    return response.data;
  }

  // User OAuth App Credentials endpoints
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
  }

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
  }

  async deleteUserOAuthApp(provider: string): Promise<void> {
    await axios.delete(`/api/users/oauth-apps/${provider}`);
  }

  // OAuth Provider Connection Status
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
  }

  // Get OAuth authorization URL for a provider
  // Calls the mobile/init endpoint to get the authorization URL from the backend
  // This works through Vite's proxy in development
  async getOAuthAuthorizeUrl(provider: string): Promise<string> {
    const response = await axios.get(`/api/oauth/mobile/init/${provider}`);
    return response.data.authorization_url;
  }

  // Prompt Suggestions API

  async getPromptSuggestions(): Promise<{
    categories: Array<{
      category_key: string;
      category_title: string;
      category_icon: string;
      pillar: 'activity' | 'nutrition' | 'recovery';
      prompts: string[];
    }>;
    welcome_prompt: string;
    metadata: {
      timestamp: string;
      api_version: string;
    };
  }> {
    const response = await axios.get('/api/prompts/suggestions');
    return response.data;
  }

  // User Coaches endpoints
  async getCoaches(options?: {
    category?: string;
    favorites_only?: boolean;
    limit?: number;
    offset?: number;
  }): Promise<{
    coaches: Array<{
      id: string;
      title: string;
      description: string | null;
      system_prompt: string;
      category: string;
      tags: string[];
      token_count: number;
      is_favorite: boolean;
      use_count: number;
      last_used_at: string | null;
      created_at: string;
      updated_at: string;
      is_system: boolean;
      visibility: string;
      is_assigned: boolean;
    }>;
    total: number;
    metadata: {
      timestamp: string;
      api_version: string;
    };
  }> {
    const params = new URLSearchParams();
    if (options?.category) params.append('category', options.category);
    if (options?.favorites_only) params.append('favorites_only', 'true');
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.offset) params.append('offset', options.offset.toString());
    const queryString = params.toString();
    const url = queryString ? `/api/coaches?${queryString}` : '/api/coaches';
    const response = await axios.get(url);
    return response.data;
  }

  async toggleCoachFavorite(coachId: string): Promise<{ is_favorite: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/favorite`);
    return response.data;
  }

  async recordCoachUsage(coachId: string): Promise<{ success: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/usage`);
    return response.data;
  }

  async createCoach(data: {
    title: string;
    description?: string;
    system_prompt: string;
    category?: string;
    tags?: string[];
  }): Promise<{
    id: string;
    title: string;
    description: string | null;
    system_prompt: string;
    category: string;
    tags: string[];
    token_count: number;
    is_favorite: boolean;
    use_count: number;
    last_used_at: string | null;
    created_at: string;
    updated_at: string;
    is_system: boolean;
    visibility: string;
    is_assigned: boolean;
  }> {
    const response = await axios.post('/api/coaches', data);
    return response.data;
  }

  async updateCoach(coachId: string, data: {
    title?: string;
    description?: string;
    system_prompt?: string;
    category?: string;
    tags?: string[];
  }): Promise<{
    id: string;
    title: string;
    description: string | null;
    system_prompt: string;
    category: string;
    tags: string[];
    token_count: number;
    is_favorite: boolean;
    use_count: number;
    last_used_at: string | null;
    created_at: string;
    updated_at: string;
    is_system: boolean;
    visibility: string;
    is_assigned: boolean;
  }> {
    const response = await axios.put(`/api/coaches/${coachId}`, data);
    return response.data;
  }

  async deleteCoach(coachId: string): Promise<void> {
    await axios.delete(`/api/coaches/${coachId}`);
  }

  async hideCoach(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/hide`);
    return response.data;
  }

  async showCoach(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
    const response = await axios.delete(`/api/coaches/${coachId}/hide`);
    return response.data;
  }

  async getHiddenCoaches(): Promise<{
    coaches: Array<{
      id: string;
      title: string;
      description: string | null;
      system_prompt: string;
      category: string;
      tags: string[];
      token_count: number;
      is_favorite: boolean;
      use_count: number;
      last_used_at: string | null;
      created_at: string;
      updated_at: string;
      is_system: boolean;
      visibility: string;
      is_assigned: boolean;
    }>;
  }> {
    const response = await axios.get('/api/coaches/hidden');
    return response.data;
  }

  // Coach Version History endpoints (ASY-153)
  async getCoachVersions(coachId: string, limit?: number): Promise<{
    versions: Array<{
      version: number;
      content_snapshot: Record<string, unknown>;
      change_summary: string | null;
      created_at: string;
      created_by_name: string | null;
    }>;
    current_version: number;
    total: number;
  }> {
    const params = new URLSearchParams();
    if (limit) params.append('limit', limit.toString());
    const url = params.toString()
      ? `/api/coaches/${coachId}/versions?${params}`
      : `/api/coaches/${coachId}/versions`;
    const response = await axios.get(url);
    return response.data;
  }

  async getCoachVersion(coachId: string, version: number): Promise<{
    version: number;
    content_snapshot: Record<string, unknown>;
    change_summary: string | null;
    created_at: string;
    created_by_name: string | null;
  }> {
    const response = await axios.get(`/api/coaches/${coachId}/versions/${version}`);
    return response.data;
  }

  async revertCoachToVersion(coachId: string, version: number): Promise<{
    coach: {
      id: string;
      title: string;
      description: string | null;
      system_prompt: string;
      category: string;
      tags: string[];
      token_count: number;
      is_favorite: boolean;
      use_count: number;
      last_used_at: string | null;
      created_at: string;
      updated_at: string;
      is_system: boolean;
      visibility: string;
      is_assigned: boolean;
    };
    reverted_to_version: number;
    new_version: number;
  }> {
    const response = await axios.post(`/api/coaches/${coachId}/versions/${version}/revert`);
    return response.data;
  }

  async getCoachVersionDiff(coachId: string, fromVersion: number, toVersion: number): Promise<{
    from_version: number;
    to_version: number;
    changes: Array<{
      field: string;
      old_value: unknown | null;
      new_value: unknown | null;
    }>;
  }> {
    const response = await axios.get(`/api/coaches/${coachId}/versions/${fromVersion}/diff/${toVersion}`);
    return response.data;
  }

  // LLM Settings endpoints
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
  }

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
  }

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
  }

  async deleteLlmCredentials(provider: string): Promise<{
    success: boolean;
    message: string;
  }> {
    const response = await axios.delete(`/api/user/llm-settings/${provider}`);
    return response.data;
  }

// Admin Coach Management endpoints (System Coaches)
  async getSystemCoaches(): Promise<{
    coaches: Array<{
      id: string;
      title: string;
      description: string | null;
      system_prompt: string;
      category: string;
      tags: string[];
      token_count: number;
      is_favorite: boolean;
      use_count: number;
      last_used_at: string | null;
      created_at: string;
      updated_at: string;
      is_system: boolean;
      visibility: string;
      is_assigned: boolean;
    }>;
    total: number;
    metadata: {
      timestamp: string;
      api_version: string;
    };
  }> {
    const response = await axios.get('/api/admin/coaches');
    return response.data;
  }

  async createSystemCoach(data: {
    title: string;
    description?: string;
    system_prompt: string;
    category?: string;
    tags?: string[];
    visibility?: string;
  }): Promise<{
    id: string;
    title: string;
    description: string | null;
    system_prompt: string;
    category: string;
    tags: string[];
    token_count: number;
    is_favorite: boolean;
    use_count: number;
    last_used_at: string | null;
    created_at: string;
    updated_at: string;
    is_system: boolean;
    visibility: string;
    is_assigned: boolean;
  }> {
    const response = await axios.post('/api/admin/coaches', data);
    return response.data;
  }

  async getSystemCoach(coachId: string): Promise<{
    id: string;
    title: string;
    description: string | null;
    system_prompt: string;
    category: string;
    tags: string[];
    token_count: number;
    is_favorite: boolean;
    use_count: number;
    last_used_at: string | null;
    created_at: string;
    updated_at: string;
    is_system: boolean;
    visibility: string;
    is_assigned: boolean;
  }> {
    const response = await axios.get(`/api/admin/coaches/${coachId}`);
    return response.data;
  }

  async updateSystemCoach(coachId: string, data: {
    title?: string;
    description?: string;
    system_prompt?: string;
    category?: string;
    tags?: string[];
  }): Promise<{
    id: string;
    title: string;
    description: string | null;
    system_prompt: string;
    category: string;
    tags: string[];
    token_count: number;
    is_favorite: boolean;
    use_count: number;
    last_used_at: string | null;
    created_at: string;
    updated_at: string;
    is_system: boolean;
    visibility: string;
    is_assigned: boolean;
  }> {
    const response = await axios.put(`/api/admin/coaches/${coachId}`, data);
    return response.data;
  }

  async deleteSystemCoach(coachId: string): Promise<void> {
    await axios.delete(`/api/admin/coaches/${coachId}`);
  }

  async assignCoachToUsers(coachId: string, userIds: string[]): Promise<{
    coach_id: string;
    assigned_count: number;
    total_requested: number;
  }> {
    const response = await axios.post(`/api/admin/coaches/${coachId}/assign`, {
      user_ids: userIds,
    });
    return response.data;
  }

  async unassignCoachFromUsers(coachId: string, userIds: string[]): Promise<{
    coach_id: string;
    removed_count: number;
    total_requested: number;
  }> {
    const response = await axios.delete(`/api/admin/coaches/${coachId}/assign`, {
      data: { user_ids: userIds },
    });
    return response.data;
  }

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
  }

  // Tool Selection Admin API endpoints
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
  }

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
  }

  async getGlobalDisabledTools(): Promise<{
    success: boolean;
    message: string;
    data: {
      disabled_tools: string[];
      count: number;
    };
  }> {
    const response = await axios.get('/api/admin/tools/global-disabled');
    return response.data;
  }

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
  }

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
  }

  async removeToolOverride(tenantId: string, toolName: string): Promise<{
    success: boolean;
    message: string;
  }> {
    const response = await axios.delete(`/api/admin/tools/tenant/${tenantId}/override/${toolName}`);
    return response.data;
  }

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
  }

  // Coach Store endpoints
  async browseStoreCoaches(options?: {
    category?: string;
    sort_by?: 'newest' | 'popular' | 'title';
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
    }>;
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    if (options?.category) params.append('category', options.category);
    if (options?.sort_by) params.append('sort_by', options.sort_by);
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.offset) params.append('offset', options.offset.toString());
    const queryString = params.toString();
    const url = queryString ? `/api/store/coaches?${queryString}` : '/api/store/coaches';
    const response = await axios.get(url);
    return response.data;
  }

  async searchStoreCoaches(query: string, limit?: number): Promise<{
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
    }>;
    query: string;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    params.append('q', query);
    if (limit) params.append('limit', limit.toString());
    const response = await axios.get(`/api/store/search?${params}`);
    return response.data;
  }

  async getStoreCoach(coachId: string): Promise<{
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
    system_prompt: string;
    created_at: string;
    publish_status: string;
  }> {
    const response = await axios.get(`/api/store/coaches/${coachId}`);
    return response.data;
  }

  async getStoreCategories(): Promise<{
    categories: Array<{
      category: string;
      name: string;
      count: number;
    }>;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/store/categories');
    return response.data;
  }

  async installStoreCoach(coachId: string): Promise<{
    message: string;
    coach: {
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
    };
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.post(`/api/store/coaches/${coachId}/install`);
    return response.data;
  }

  async uninstallStoreCoach(coachId: string): Promise<{
    message: string;
    source_coach_id: string;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.delete(`/api/store/coaches/${coachId}/install`);
    return response.data;
  }

  async getStoreInstallations(): Promise<{
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
    }>;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/store/installations');
    return response.data;
  }
}

export const apiService = new ApiService();