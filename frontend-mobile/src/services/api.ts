// ABOUTME: API service for Pierre Mobile app
// ABOUTME: Handles all HTTP requests with JWT auth and AsyncStorage for persistence

import axios, { type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';
import { Platform } from 'react-native';
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
  OAuthApp,
  OAuthAppCredentials,
  FirebaseLoginResponse,
  Coach,
  CreateCoachRequest,
  UpdateCoachRequest,
  ListCoachesResponse,
  ForkCoachResponse,
} from '../types';

// Configuration - should be set via environment or config
// For iOS Simulator, localhost works directly. For Android emulator, use 10.0.2.2
const getDefaultApiUrl = (): string => {
  // First, check for explicit environment configuration
  if (process.env.EXPO_PUBLIC_API_URL) {
    return process.env.EXPO_PUBLIC_API_URL;
  }
  // Android emulator cannot access localhost - it needs 10.0.2.2 to reach host machine
  // This applies to both debug and release builds running on emulator without explicit URL config
  if (Platform.OS === 'android') {
    return 'http://10.0.2.2:8081';
  }
  return 'http://localhost:8081';
};

const API_BASE_URL = getDefaultApiUrl();

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

  async loginWithFirebase(idToken: string): Promise<FirebaseLoginResponse> {
    const response = await axios.post('/api/auth/firebase', { id_token: idToken });
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
    // Handle both array and object response formats
    const data = response.data;
    if (Array.isArray(data)) {
      return { providers: data };
    }
    // If data is an object with providers field, use that
    if (data && Array.isArray(data.providers)) {
      return { providers: data.providers };
    }
    // Default to empty array
    return { providers: [] };
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

  // OAuth Apps endpoints (custom provider credentials)
  async getUserOAuthApps(): Promise<{ apps: OAuthApp[] }> {
    const response = await axios.get('/api/users/oauth-apps');
    return response.data;
  }

  async registerUserOAuthApp(data: OAuthAppCredentials): Promise<{
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

  // Coach endpoints for AI coaching personas
  /**
   * List user's coaches with optional filtering
   * @param options - Optional filtering parameters
   */
  async listCoaches(options?: {
    category?: string;
    favorites_only?: boolean;
    include_hidden?: boolean;
  }): Promise<ListCoachesResponse> {
    const params = new URLSearchParams();
    if (options?.category) params.append('category', options.category);
    if (options?.favorites_only) params.append('favorites_only', 'true');
    if (options?.include_hidden) params.append('include_hidden', 'true');
    const queryString = params.toString();
    const url = queryString ? `/api/coaches?${queryString}` : '/api/coaches';
    const response = await axios.get(url);
    return response.data;
  }

  /**
   * Create a new coach
   */
  async createCoach(request: CreateCoachRequest): Promise<Coach> {
    const response = await axios.post('/api/coaches', request);
    return response.data;
  }

  /**
   * Get a specific coach by ID
   */
  async getCoach(coachId: string): Promise<Coach> {
    const response = await axios.get(`/api/coaches/${coachId}`);
    return response.data;
  }

  /**
   * Update an existing coach
   */
  async updateCoach(coachId: string, request: UpdateCoachRequest): Promise<Coach> {
    const response = await axios.put(`/api/coaches/${coachId}`, request);
    return response.data;
  }

  /**
   * Delete a coach
   */
  async deleteCoach(coachId: string): Promise<void> {
    await axios.delete(`/api/coaches/${coachId}`);
  }

  /**
   * Record coach usage (call when user selects a coach)
   * This is fire-and-forget - errors are silently ignored
   */
  async recordCoachUsage(coachId: string): Promise<void> {
    try {
      await axios.post(`/api/coaches/${coachId}/use`);
    } catch (error) {
      // Silent failure - usage tracking is non-critical
      console.debug('Failed to record coach usage:', error);
    }
  }

  /**
   * Toggle coach favorite status
   * @returns The new favorite status
   */
  async toggleCoachFavorite(coachId: string): Promise<{ is_favorite: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/favorite`);
    return response.data;
  }

  /**
   * Hide a system or assigned coach from user's view
   * Only system or assigned coaches can be hidden (not personal coaches)
   */
  async hideCoach(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
    const response = await axios.post(`/api/coaches/${coachId}/hide`);
    return response.data;
  }

  /**
   * Show (unhide) a previously hidden coach
   */
  async showCoach(coachId: string): Promise<{ success: boolean; is_hidden: boolean }> {
    const response = await axios.delete(`/api/coaches/${coachId}/hide`);
    return response.data;
  }

  /**
   * List hidden coaches for the user
   */
  async listHiddenCoaches(): Promise<ListCoachesResponse> {
    const response = await axios.get('/api/coaches?include_hidden=true');
    return response.data;
  }

  /**
   * Fork a system coach to create a user-owned copy
   * Only system coaches (is_system=true) can be forked
   */
  async forkCoach(coachId: string): Promise<ForkCoachResponse> {
    const response = await axios.post(`/api/coaches/${coachId}/fork`);
    return response.data;
  }

  /**
   * Get version history for a coach (ASY-153)
   */
  async getCoachVersions(
    coachId: string,
    limit?: number
  ): Promise<{
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

  /**
   * Get a specific version of a coach
   */
  async getCoachVersion(
    coachId: string,
    version: number
  ): Promise<{
    version: number;
    content_snapshot: Record<string, unknown>;
    change_summary: string | null;
    created_at: string;
    created_by_name: string | null;
  }> {
    const response = await axios.get(`/api/coaches/${coachId}/versions/${version}`);
    return response.data;
  }

  /**
   * Revert a coach to a previous version (creates new version with old content)
   */
  async revertCoachToVersion(
    coachId: string,
    version: number
  ): Promise<{
    coach: Coach;
    reverted_to_version: number;
    new_version: number;
  }> {
    const response = await axios.post(`/api/coaches/${coachId}/versions/${version}/revert`);
    return response.data;
  }

  /**
   * Get diff between two versions of a coach
   */
  async getCoachVersionDiff(
    coachId: string,
    fromVersion: number,
    toVersion: number
  ): Promise<{
    from_version: number;
    to_version: number;
    changes: Array<{
      field: string;
      old_value: unknown | null;
      new_value: unknown | null;
    }>;
  }> {
    const response = await axios.get(
      `/api/coaches/${coachId}/versions/${fromVersion}/diff/${toVersion}`
    );
    return response.data;
  }

  // WebSocket URL for chat streaming
  getWebSocketUrl(conversationId: string): string {
    const wsBase = API_BASE_URL.replace(/^http/, 'ws');
    return `${wsBase}/api/chat/ws/${conversationId}?token=${this.jwtToken}`;
  }
}

export const apiService = new ApiService();
