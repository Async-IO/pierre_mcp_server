import axios, { AxiosError, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8081';

class ApiService {
  private isRefreshing = false;
  private failedQueue: Array<{
    resolve: (value: string) => void;
    reject: (error: Error) => void;
  }> = [];

  constructor() {
    // Set up axios defaults
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.headers.common['Content-Type'] = 'application/json';

    // Set up response interceptor for automatic token refresh
    this.setupInterceptors();
  }

  private setupInterceptors() {
    // Request interceptor to add token
    axios.interceptors.request.use(
      (config: InternalAxiosRequestConfig) => {
        const token = this.getToken();
        if (token && config.headers) {
          config.headers.Authorization = `Bearer ${token}`;
        }
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor to handle 401s with automatic refresh
    axios.interceptors.response.use(
      (response: AxiosResponse) => response,
      async (error: AxiosError) => {
        const originalRequest = error.config as InternalAxiosRequestConfig & { _retry?: boolean };

        if (error.response?.status === 401 && !originalRequest._retry) {
          if (this.isRefreshing) {
            // If already refreshing, queue the request
            return new Promise((resolve, reject) => {
              this.failedQueue.push({ resolve, reject });
            }).then((token) => {
              if (originalRequest.headers) {
                originalRequest.headers.Authorization = `Bearer ${token}`;
              }
              return axios(originalRequest);
            }).catch((err) => {
              return Promise.reject(err);
            });
          }

          originalRequest._retry = true;
          this.isRefreshing = true;

          try {
            const newToken = await this.refreshToken();
            this.processQueue(null, newToken);
            
            if (originalRequest.headers) {
              originalRequest.headers.Authorization = `Bearer ${newToken}`;
            }
            return axios(originalRequest);
          } catch (refreshError) {
            this.processQueue(refreshError instanceof Error ? refreshError : new Error(String(refreshError)), null);
            // Token refresh failed, redirect to login
            this.handleAuthFailure();
            return Promise.reject(refreshError);
          } finally {
            this.isRefreshing = false;
          }
        }

        return Promise.reject(error);
      }
    );
  }

  private processQueue(error: Error | null, token: string | null) {
    this.failedQueue.forEach(({ resolve, reject }) => {
      if (error) {
        reject(error);
      } else if (token) {
        resolve(token);
      }
    });
    
    this.failedQueue = [];
  }

  private handleAuthFailure() {
    // Clear stored auth data
    this.clearToken();
    this.clearUser();
    
    // Trigger logout event for components to react
    window.dispatchEvent(new CustomEvent('auth-failure'));
    
    // Redirect to login if not already there
    if (!window.location.pathname.includes('/login')) {
      window.location.href = '/login';
    }
  }

  setAuthToken(token: string | null) {
    if (token) {
      axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;
    } else {
      delete axios.defaults.headers.common['Authorization'];
    }
  }

  async login(email: string, password: string) {
    const response = await axios.post('/auth/login', { email, password });
    return response.data;
  }

  async register(email: string, password: string, displayName?: string) {
    const response = await axios.post('/auth/register', {
      email,
      password,
      display_name: displayName,
    });
    return response.data;
  }

  // API Key Management
  async createApiKey(data: {
    name: string;
    description?: string;
    tier: 'starter' | 'professional' | 'enterprise';
    expires_in_days?: number;
  }) {
    const response = await axios.post('/api/keys', data);
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
    const response = await axios.get('/dashboard/overview');
    return response.data;
  }

  async getUsageAnalytics(days: number = 30) {
    const response = await axios.get(`/dashboard/analytics?days=${days}`);
    return response.data;
  }

  async getRateLimitOverview() {
    const response = await axios.get('/dashboard/rate-limits');
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
    
    const response = await axios.get(`/dashboard/request-logs?${params}`);
    return response.data;
  }

  async getRequestStats(apiKeyId?: string, timeRange: string = '1h') {
    const params = new URLSearchParams();
    if (apiKeyId) params.append('api_key_id', apiKeyId);
    params.append('time_range', timeRange);
    
    const response = await axios.get(`/dashboard/request-stats?${params}`);
    return response.data;
  }

  async getToolUsageBreakdown(apiKeyId?: string, timeRange: string = '7d') {
    const params = new URLSearchParams();
    if (apiKeyId) params.append('api_key_id', apiKeyId);
    params.append('time_range', timeRange);
    
    const response = await axios.get(`/dashboard/tool-usage?${params}`);
    return response.data;
  }

  async createTrialKey(data: {
    name: string;
    description?: string;
  }) {
    const response = await axios.post('/api/keys/trial', data);
    return response.data;
  }

  // Token management
  getToken(): string | null {
    return localStorage.getItem('auth_token');
  }

  setToken(token: string) {
    localStorage.setItem('auth_token', token);
    if (axios.defaults.headers.common) {
      axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;
    }
  }

  clearToken() {
    localStorage.removeItem('auth_token');
    if (axios.defaults.headers.common) {
      delete axios.defaults.headers.common['Authorization'];
    }
  }

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

  // JWT Token Refresh
  async refreshToken(): Promise<string> {
    const currentToken = this.getToken();
    const user = this.getUser();
    
    if (!currentToken || !user) {
      throw new Error('No token or user data available for refresh');
    }

    try {
      // Make refresh request without interceptors to avoid infinite loop
      const response = await axios.post('/auth/refresh', {
        token: currentToken,
        user_id: user.id
      }, {
        // Skip the interceptor for this request
        headers: {
          'Authorization': `Bearer ${currentToken}`
        }
      });

      const { jwt_token } = response.data;
      
      // Update stored token
      this.setToken(jwt_token);
      
      // Notify components of token update
      window.dispatchEvent(new CustomEvent('token-updated', { 
        detail: { token: jwt_token } 
      }));
      
      return jwt_token;
    } catch (error) {
      console.error('Token refresh failed:', error);
      throw error;
    }
  }

  // Check if token is expired or will expire soon
  isTokenExpired(): boolean {
    const token = this.getToken();
    if (!token) return true;

    try {
      // Decode JWT payload (this is not secure validation, just for expiry check)
      const payload = JSON.parse(atob(token.split('.')[1]));
      const currentTime = Math.floor(Date.now() / 1000);
      const expiryTime = payload.exp;
      
      // Consider token expired if it expires within 5 minutes
      return currentTime >= (expiryTime - 300);
    } catch (error) {
      console.error('Error checking token expiry:', error);
      return true;
    }
  }

  // Proactively refresh token if it's about to expire
  async checkAndRefreshToken(): Promise<void> {
    if (this.isTokenExpired()) {
      try {
        await this.refreshToken();
      } catch (error) {
        console.error('Proactive token refresh failed:', error);
        this.handleAuthFailure();
      }
    }
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
}

export const apiService = new ApiService();