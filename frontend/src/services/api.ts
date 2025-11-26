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
    const response = await axios.post('/api/auth/login', { email, password });
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
    
    const response = await axios.get(`/admin/tokens?${queryParams}`);
    return response.data;
  }

  async createAdminToken(data: {
    service_name: string;
    service_description?: string;
    permissions?: string[];
    is_super_admin?: boolean;
    expires_in_days?: number;
  }) {
    const response = await axios.post('/admin/tokens', data);
    return response.data;
  }

  async getAdminTokenDetails(tokenId: string) {
    const response = await axios.get(`/admin/tokens/${tokenId}`);
    return response.data;
  }

  async revokeAdminToken(tokenId: string) {
    const response = await axios.post(`/admin/tokens/${tokenId}/revoke`);
    return response.data;
  }

  async rotateAdminToken(tokenId: string, data?: {
    expires_in_days?: number;
  }) {
    const response = await axios.post(`/admin/tokens/${tokenId}/rotate`, data || {});
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
    try {
      const response = await axios.get('/admin/pending-users');
      return response.data;
    } catch (error) {
      // Admin endpoint requires admin token authentication
      // Return empty array for non-admin users to prevent dashboard errors
      return [];
    }
  }

  async approveUser(userId: string, reason?: string) {
    const response = await axios.post(`/admin/approve-user/${userId}`, {
      reason
    });
    return response.data;
  }

  async suspendUser(userId: string, reason?: string) {
    const response = await axios.post(`/admin/suspend-user/${userId}`, {
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
    
    const response = await axios.get(`/admin/users?${queryParams}`);
    return response.data;
  }
}

export const apiService = new ApiService();