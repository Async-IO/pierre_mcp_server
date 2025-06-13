import axios from 'axios';

const API_BASE_URL = 'http://localhost:8081';

class ApiService {
  private token: string | null = null;

  constructor() {
    // Set up axios defaults
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.headers.common['Content-Type'] = 'application/json';
  }

  setAuthToken(token: string | null) {
    this.token = token;
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
}

export const apiService = new ApiService();