import axios from 'axios';

const API_BASE_URL = 'http://localhost:8081';

class ApiService {

  constructor() {
    // Set up axios defaults
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.headers.common['Content-Type'] = 'application/json';
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
    return localStorage.getItem('token');
  }

  setToken(token: string) {
    localStorage.setItem('token', token);
  }

  clearToken() {
    localStorage.removeItem('token');
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