import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import axios, { AxiosError } from 'axios';
import { apiService } from '../api';

// Mock axios
vi.mock('axios', () => ({
  default: {
    defaults: { headers: { common: {} } },
    interceptors: {
      request: { use: vi.fn() },
      response: { use: vi.fn() }
    },
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn()
  },
  AxiosError: class extends Error {
    constructor(message: string, code?: string, config?: unknown, request?: unknown, response?: unknown) {
      super(message);
      this.code = code;
      this.config = config;
      this.request = request;
      this.response = response;
    }
    code?: string;
    config?: unknown;
    request?: unknown;
    response?: unknown;
    isAxiosError = true;
  }
}));

const mockedAxios = vi.mocked(axios);

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

// Mock window events
Object.defineProperty(window, 'dispatchEvent', { value: vi.fn() });

describe('API Service - Admin Functionality', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMock.getItem.mockReturnValue('mock-token');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Dashboard Overview Endpoints', () => {
    it('should fetch dashboard overview successfully', async () => {
      const mockOverview = {
        total_api_keys: 15,
        active_api_keys: 12,
        total_requests_today: 1247,
        total_requests_this_month: 45623,
        current_month_usage_by_tier: []
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockOverview });

      const result = await apiService.getDashboardOverview();

      expect(mockedAxios.get).toHaveBeenCalledWith('/api/dashboard/overview');
      expect(result).toEqual(mockOverview);
    });

    it('should fetch usage analytics with correct parameters', async () => {
      const mockAnalytics = {
        time_series: [
          { date: '2025-01-01', request_count: 145 },
          { date: '2025-01-02', request_count: 203 }
        ]
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockAnalytics });

      const result = await apiService.getUsageAnalytics(7);

      expect(mockedAxios.get).toHaveBeenCalledWith('/api/dashboard/analytics?days=7');
      expect(result).toEqual(mockAnalytics);
    });

    it('should fetch rate limit overview', async () => {
      const mockRateLimits = [
        {
          api_key_id: 'key-1',
          api_key_name: 'Test Key',
          tier: 'professional',
          current_usage: 100,
          limit: 1000,
          usage_percentage: 10
        }
      ];

      mockedAxios.get.mockResolvedValueOnce({ data: mockRateLimits });

      const result = await apiService.getRateLimitOverview();

      expect(mockedAxios.get).toHaveBeenCalledWith('/api/dashboard/rate-limits');
      expect(result).toEqual(mockRateLimits);
    });
  });

  describe('A2A Dashboard Endpoints', () => {
    it('should fetch A2A dashboard overview', async () => {
      const mockA2AOverview = {
        total_clients: 5,
        active_clients: 3,
        requests_today: 423,
        requests_this_month: 12543
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockA2AOverview });

      const result = await apiService.getA2ADashboardOverview();

      expect(mockedAxios.get).toHaveBeenCalledWith('/a2a/dashboard/overview');
      expect(result).toEqual(mockA2AOverview);
    });

    it('should fetch A2A usage analytics with parameters', async () => {
      const mockA2AAnalytics = { request_count: 1500, tool_usage: {} };

      mockedAxios.get.mockResolvedValueOnce({ data: mockA2AAnalytics });

      const result = await apiService.getA2AUsageAnalytics(14);

      expect(mockedAxios.get).toHaveBeenCalledWith('/a2a/dashboard/analytics?days=14');
      expect(result).toEqual(mockA2AAnalytics);
    });
  });

  describe('Admin Token Management', () => {
    it('should fetch admin tokens with parameters', async () => {
      const mockTokens = {
        admin_tokens: [
          {
            id: 'token-1',
            service_name: 'Test Service',
            is_active: true,
            created_at: '2025-01-01T00:00:00Z',
            token_prefix: 'at_abc123'
          }
        ],
        total_count: 1
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockTokens });

      const result = await apiService.getAdminTokens({ include_inactive: true });

      expect(mockedAxios.get).toHaveBeenCalledWith('/admin/tokens?include_inactive=true');
      expect(result).toEqual(mockTokens);
    });

    it('should create admin token successfully', async () => {
      const tokenRequest = {
        service_name: 'New Service',
        service_description: 'Test service',
        permissions: ['read_users'],
        is_super_admin: false,
        expires_in_days: 90
      };

      const mockResponse = {
        admin_token: {
          id: 'token-2',
          service_name: 'New Service',
          token_prefix: 'at_def456'
        },
        jwt_token: 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...'
      };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.createAdminToken(tokenRequest);

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/tokens', tokenRequest);
      expect(result).toEqual(mockResponse);
    });

    it('should get admin token details', async () => {
      const mockTokenDetails = {
        id: 'token-1',
        service_name: 'Test Service',
        permissions: ['read_users', 'manage_api_keys'],
        usage_count: 45,
        last_used_at: '2025-01-06T10:30:00Z'
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockTokenDetails });

      const result = await apiService.getAdminTokenDetails('token-1');

      expect(mockedAxios.get).toHaveBeenCalledWith('/admin/tokens/token-1');
      expect(result).toEqual(mockTokenDetails);
    });

    it('should revoke admin token', async () => {
      const mockResponse = { success: true, message: 'Token revoked' };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.revokeAdminToken('token-1');

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/tokens/token-1/revoke');
      expect(result).toEqual(mockResponse);
    });

    it('should rotate admin token', async () => {
      const rotateRequest = { expires_in_days: 180 };
      const mockResponse = {
        admin_token: { id: 'token-1', token_prefix: 'at_new123' },
        jwt_token: 'new.jwt.token'
      };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.rotateAdminToken('token-1', rotateRequest);

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/tokens/token-1/rotate', rotateRequest);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('User Management', () => {
    it('should fetch pending users', async () => {
      const mockPendingUsers = [
        { id: 'user-1', email: 'user1@example.com', display_name: 'User One', created_at: '2025-01-01T00:00:00Z' },
        { id: 'user-2', email: 'user2@example.com', display_name: 'User Two', created_at: '2025-01-02T00:00:00Z' }
      ];

      mockedAxios.get.mockResolvedValueOnce({ data: mockPendingUsers });

      const result = await apiService.getPendingUsers();

      expect(mockedAxios.get).toHaveBeenCalledWith('/admin/pending-users');
      expect(result).toEqual(mockPendingUsers);
    });

    it('should approve user with reason', async () => {
      const mockResponse = { success: true, message: 'User approved' };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.approveUser('user-1', 'Valid business use case');

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/approve-user/user-1', {
        reason: 'Valid business use case'
      });
      expect(result).toEqual(mockResponse);
    });

    it('should suspend user with reason', async () => {
      const mockResponse = { success: true, message: 'User suspended' };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.suspendUser('user-1', 'Policy violation');

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/suspend-user/user-1', {
        reason: 'Policy violation'
      });
      expect(result).toEqual(mockResponse);
    });

    it('should fetch all users with status filter', async () => {
      const mockUsers = {
        users: [
          { id: 'user-1', email: 'user1@example.com', status: 'active' },
          { id: 'user-2', email: 'user2@example.com', status: 'active' }
        ],
        total_count: 2
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockUsers });

      const result = await apiService.getAllUsers({ 
        status: 'active', 
        limit: 50, 
        offset: 0 
      });

      expect(mockedAxios.get).toHaveBeenCalledWith('/admin/users?status=active&limit=50');
      expect(result).toEqual(mockUsers);
    });
  });

  describe('API Key Provisioning', () => {
    it('should provision API key for user', async () => {
      const provisionRequest = {
        user_email: 'user@example.com',
        tier: 'professional',
        description: 'API key for user project',
        expires_in_days: 365,
        rate_limit_requests: 5000,
        rate_limit_period: 'month'
      };

      const mockResponse = {
        api_key: 'pk_live_abc123...',
        key_id: 'key-123',
        tier: 'professional'
      };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.provisionApiKey(provisionRequest);

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/provision-api-key', provisionRequest);
      expect(result).toEqual(mockResponse);
    });

    it('should revoke API key by ID', async () => {
      const revokeRequest = { key_id: 'key-123' };
      const mockResponse = { success: true, message: 'API key revoked' };

      mockedAxios.post.mockResolvedValueOnce({ data: mockResponse });

      const result = await apiService.revokeApiKey(revokeRequest);

      expect(mockedAxios.post).toHaveBeenCalledWith('/admin/revoke-api-key', revokeRequest);
      expect(result).toEqual(mockResponse);
    });

    it('should list API keys for user', async () => {
      const mockApiKeys = {
        api_keys: [
          {
            id: 'key-1',
            name: 'User API Key',
            tier: 'professional',
            is_active: true,
            user_email: 'user@example.com'
          }
        ],
        total_count: 1
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockApiKeys });

      const result = await apiService.listApiKeys({
        user_email: 'user@example.com',
        active_only: true,
        limit: 10
      });

      expect(mockedAxios.get).toHaveBeenCalledWith('/admin/list-api-keys?user_email=user%40example.com&active_only=true&limit=10');
      expect(result).toEqual(mockApiKeys);
    });
  });

  describe('Request Monitoring', () => {
    it('should fetch request logs with filters', async () => {
      const mockLogs = {
        request_logs: [
          {
            id: 'req-1',
            api_key_id: 'key-1',
            method: 'GET',
            path: '/api/fitness/activities',
            status: 200,
            response_time_ms: 145,
            timestamp: '2025-01-07T10:30:00Z'
          }
        ],
        total_count: 1
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockLogs });

      const result = await apiService.getRequestLogs('key-1', {
        timeRange: '1h',
        status: '200',
        tool: 'get_activities'
      });

      expect(mockedAxios.get).toHaveBeenCalledWith('/api/dashboard/request-logs?api_key_id=key-1&time_range=1h&status=200&tool=get_activities');
      expect(result).toEqual(mockLogs);
    });

    it('should fetch request statistics', async () => {
      const mockStats = {
        total_requests: 1250,
        success_rate: 98.4,
        average_response_time: 234,
        error_breakdown: {
          '400': 5,
          '429': 3,
          '500': 2
        }
      };

      mockedAxios.get.mockResolvedValueOnce({ data: mockStats });

      const result = await apiService.getRequestStats('key-1', '24h');

      expect(mockedAxios.get).toHaveBeenCalledWith('/api/dashboard/request-stats?api_key_id=key-1&time_range=24h');
      expect(result).toEqual(mockStats);
    });
  });

  describe('Error Handling', () => {
    it('should handle 404 errors appropriately', async () => {
      const error = new AxiosError('Not Found', '404', undefined, undefined, {
        status: 404,
        data: { error: 'Token not found' }
      });

      mockedAxios.get.mockRejectedValueOnce(error);

      await expect(apiService.getAdminTokenDetails('invalid-token')).rejects.toThrow('Not Found');
    });

    it('should handle network errors', async () => {
      const error = new AxiosError('Network Error', 'NETWORK_ERROR');

      mockedAxios.get.mockRejectedValueOnce(error);

      await expect(apiService.getDashboardOverview()).rejects.toThrow('Network Error');
    });

    it('should handle 403 unauthorized errors', async () => {
      const error = new AxiosError('Forbidden', '403', undefined, undefined, {
        status: 403,
        data: { error: 'Insufficient permissions' }
      });

      mockedAxios.post.mockRejectedValueOnce(error);

      await expect(apiService.createAdminToken({
        service_name: 'Test',
        permissions: ['super_admin']
      })).rejects.toThrow('Forbidden');
    });
  });

  describe('Token Refresh Integration', () => {
    it('should refresh JWT token successfully', async () => {
      const mockRefreshResponse = {
        jwt_token: 'new.jwt.token',
        expires_at: '2025-01-08T10:00:00Z'
      };

      localStorageMock.getItem.mockImplementation((key: string) => {
        if (key === 'auth_token') return 'old.jwt.token';
        if (key === 'user') return JSON.stringify({ id: 'user-1', email: 'test@example.com' });
        return null;
      });

      mockedAxios.post.mockResolvedValueOnce({ data: mockRefreshResponse });

      const result = await apiService.refreshToken();

      expect(mockedAxios.post).toHaveBeenCalledWith('/api/auth/refresh', {
        token: 'old.jwt.token',
        user_id: 'user-1'
      }, {
        headers: {
          'Authorization': 'Bearer old.jwt.token'
        }
      });

      expect(result).toBe('new.jwt.token');
      expect(localStorageMock.setItem).toHaveBeenCalledWith('auth_token', 'new.jwt.token');
    });

    it('should handle token refresh failure', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const error = new AxiosError('Token invalid', '401');
      
      localStorageMock.getItem.mockImplementation((key: string) => {
        if (key === 'auth_token') return 'expired.jwt.token';
        if (key === 'user') return JSON.stringify({ id: 'user-1' });
        return null;
      });

      mockedAxios.post.mockRejectedValueOnce(error);

      await expect(apiService.refreshToken()).rejects.toThrow('Token invalid');
      
      consoleSpy.mockRestore();
    });
  });
});