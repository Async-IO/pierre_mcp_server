// ABOUTME: API service for Pierre Mobile app
// ABOUTME: Handles all HTTP requests with JWT auth and AsyncStorage for persistence

import axios, { type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';
import AsyncStorage from '@react-native-async-storage/async-storage';
import type {
  User,
  Conversation,
  Message,
  ProviderStatus,
  McpToken,
  PromptSuggestionsResponse,
  LoginResponse,
  RegisterResponse,
} from '../types';

// Configuration - should be set via environment or config
// For iOS Simulator, localhost works directly. For Android, use 10.0.2.2
const API_BASE_URL = process.env.EXPO_PUBLIC_API_URL || 'http://localhost:8081';

// Timeout for API requests (5 minutes to accommodate slower local LLM responses)
const API_TIMEOUT_MS = 300000;

// Storage keys
const STORAGE_KEYS = {
  JWT_TOKEN: '@pierre/jwt_token',
  REFRESH_TOKEN: '@pierre/refresh_token',
  CSRF_TOKEN: '@pierre/csrf_token',
  USER: '@pierre/user',
} as const;

// Event emitter for auth failures (React Native compatible)
type AuthFailureListener = () => void;
const authFailureListeners: AuthFailureListener[] = [];

export const onAuthFailure = (listener: AuthFailureListener) => {
  authFailureListeners.push(listener);
  return () => {
    const index = authFailureListeners.indexOf(listener);
    if (index > -1) authFailureListeners.splice(index, 1);
  };
};

const emitAuthFailure = () => {
  authFailureListeners.forEach(listener => listener());
};

class ApiService {
  private csrfToken: string | null = null;
  private jwtToken: string | null = null;
  private userId: string | null = null;

  constructor() {
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.headers.common['Content-Type'] = 'application/json';
    axios.defaults.timeout = API_TIMEOUT_MS;
    this.setupInterceptors();
  }

  private setupInterceptors() {
    // Request interceptor to add auth headers
    axios.interceptors.request.use(
      async (config: InternalAxiosRequestConfig) => {
        // Add JWT token
        if (this.jwtToken && config.headers) {
          config.headers['Authorization'] = `Bearer ${this.jwtToken}`;
        }

        // Add CSRF token for state-changing operations
        if (this.csrfToken && config.headers &&
            ['POST', 'PUT', 'DELETE', 'PATCH'].includes(config.method?.toUpperCase() || '')) {
          config.headers['X-CSRF-Token'] = this.csrfToken;
        }
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor to handle errors and extract human-readable messages
    axios.interceptors.response.use(
      (response: AxiosResponse) => response,
      async (error) => {
        if (error.response?.status === 401) {
          await this.handleAuthFailure();
        }

        // Extract human-readable error message from server response
        // Server returns: { code: "...", message: "human readable", timestamp: "..." }
        const serverMessage = error.response?.data?.message;
        if (serverMessage && typeof serverMessage === 'string') {
          // Create a new error with the server's message for better UX
          const enhancedError = new Error(serverMessage);
          // Preserve original error info for debugging
          (enhancedError as Error & { originalError?: unknown }).originalError = error;
          (enhancedError as Error & { statusCode?: number }).statusCode = error.response?.status;
          return Promise.reject(enhancedError);
        }

        return Promise.reject(error);
      }
    );
  }

  private async handleAuthFailure() {
    this.jwtToken = null;
    this.csrfToken = null;
    await this.clearStoredAuth();
    emitAuthFailure();
  }

  // Token management
  async initializeAuth(): Promise<boolean> {
    try {
      const [token, csrfToken, userJson] = await Promise.all([
        AsyncStorage.getItem(STORAGE_KEYS.JWT_TOKEN),
        AsyncStorage.getItem(STORAGE_KEYS.CSRF_TOKEN),
        AsyncStorage.getItem(STORAGE_KEYS.USER),
      ]);

      if (token) {
        this.jwtToken = token;
        this.csrfToken = csrfToken;
        if (userJson) {
          const user = JSON.parse(userJson);
          this.userId = user.id;
        }
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }

  async storeAuth(token: string, csrfToken: string, user: User) {
    this.jwtToken = token;
    this.csrfToken = csrfToken;
    this.userId = user.user_id;
    await Promise.all([
      AsyncStorage.setItem(STORAGE_KEYS.JWT_TOKEN, token),
      AsyncStorage.setItem(STORAGE_KEYS.CSRF_TOKEN, csrfToken),
      AsyncStorage.setItem(STORAGE_KEYS.USER, JSON.stringify(user)),
    ]);
  }

  async clearStoredAuth() {
    await Promise.all([
      AsyncStorage.removeItem(STORAGE_KEYS.JWT_TOKEN),
      AsyncStorage.removeItem(STORAGE_KEYS.REFRESH_TOKEN),
      AsyncStorage.removeItem(STORAGE_KEYS.CSRF_TOKEN),
      AsyncStorage.removeItem(STORAGE_KEYS.USER),
    ]);
    this.jwtToken = null;
    this.csrfToken = null;
    this.userId = null;
  }

  async getStoredUser(): Promise<User | null> {
    try {
      const userJson = await AsyncStorage.getItem(STORAGE_KEYS.USER);
      return userJson ? JSON.parse(userJson) : null;
    } catch {
      return null;
    }
  }

  // Auth endpoints
  async login(email: string, password: string): Promise<LoginResponse> {
    const params = new URLSearchParams();
    params.append('grant_type', 'password');
    params.append('username', email);
    params.append('password', password);

    const response = await axios.post('/oauth/token', params.toString(), {
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
    });
    return response.data;
  }

  async logout() {
    try {
      await axios.post('/api/auth/logout');
    } catch (error) {
      console.error('Logout API call failed:', error);
    }
    await this.clearStoredAuth();
  }

  async register(email: string, password: string, displayName?: string): Promise<RegisterResponse> {
    const response = await axios.post('/api/auth/register', {
      email,
      password,
      display_name: displayName,
    });
    return response.data;
  }

  async refreshToken(): Promise<LoginResponse> {
    const response = await axios.post('/api/auth/refresh');
    return response.data;
  }

  // Chat endpoints
  async getConversations(limit = 50, offset = 0): Promise<{
    conversations: Conversation[];
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
  }): Promise<Conversation> {
    const response = await axios.post('/api/chat/conversations', data);
    return response.data;
  }

  async getConversation(conversationId: string): Promise<Conversation> {
    const response = await axios.get(`/api/chat/conversations/${conversationId}`);
    return response.data;
  }

  async updateConversation(conversationId: string, data: { title?: string }): Promise<Conversation> {
    const response = await axios.put(`/api/chat/conversations/${conversationId}`, data);
    return response.data;
  }

  async deleteConversation(conversationId: string): Promise<void> {
    await axios.delete(`/api/chat/conversations/${conversationId}`);
  }

  async getConversationMessages(conversationId: string): Promise<{ messages: Message[] }> {
    const response = await axios.get(`/api/chat/conversations/${conversationId}/messages`);
    return response.data;
  }

  async sendMessage(conversationId: string, content: string): Promise<{
    user_message: Message;
    assistant_message: Message;
    conversation_updated_at: string;
    model: string;
    execution_time_ms: number;
  }> {
    const response = await axios.post(`/api/chat/conversations/${conversationId}/messages`, {
      content,
      stream: false,
    });
    return response.data;
  }

  // OAuth/Provider endpoints
  async getOAuthStatus(): Promise<{ providers: ProviderStatus[] }> {
    const response = await axios.get('/api/oauth/status');
    return { providers: response.data };
  }

  /**
   * Initialize mobile OAuth flow for a provider
   * Returns the authorization URL to open in an in-app browser
   * @param provider - Provider name (e.g., 'strava', 'fitbit')
   * @param redirectUrl - Optional redirect URL for deep linking back to the app
   */
  async initMobileOAuth(
    provider: string,
    redirectUrl?: string
  ): Promise<{
    authorization_url: string;
    provider: string;
    state: string;
    message: string;
  }> {
    const params = redirectUrl ? `?redirect_url=${encodeURIComponent(redirectUrl)}` : '';
    const response = await axios.get(`/api/oauth/mobile/init/${provider}${params}`);
    return response.data;
  }

  // MCP Token endpoints
  async getMcpTokens(): Promise<{ tokens: McpToken[] }> {
    const response = await axios.get('/api/user/mcp-tokens');
    return response.data;
  }

  async createMcpToken(data: { name: string; expires_in_days?: number }): Promise<McpToken> {
    const response = await axios.post('/api/user/mcp-tokens', data);
    return response.data;
  }

  async revokeMcpToken(tokenId: string): Promise<{ success: boolean }> {
    const response = await axios.delete(`/api/user/mcp-tokens/${tokenId}`);
    return response.data;
  }

  // User profile endpoints
  async updateProfile(data: { display_name: string }): Promise<{
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

  // Prompt suggestions
  async getPromptSuggestions(): Promise<PromptSuggestionsResponse> {
    const response = await axios.get('/api/prompts/suggestions');
    return response.data;
  }

  // Password change (for user settings)
  async changePassword(currentPassword: string, newPassword: string): Promise<{ success: boolean }> {
    const response = await axios.post('/api/user/change-password', {
      current_password: currentPassword,
      new_password: newPassword,
    });
    return response.data;
  }

  // WebSocket URL for chat streaming
  getWebSocketUrl(conversationId: string): string {
    const wsBase = API_BASE_URL.replace(/^http/, 'ws');
    return `${wsBase}/api/chat/ws/${conversationId}?token=${this.jwtToken}`;
  }
}

export const apiService = new ApiService();
