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
  BrowseCoachesResponse,
  SearchCoachesResponse,
  CategoriesResponse,
  StoreCoachDetail,
  InstallCoachResponse,
  UninstallCoachResponse,
  InstallationsResponse,
  // Social types
  ListFriendsResponse,
  PendingRequestsResponse,
  FriendConnectionResponse,
  SearchUsersResponse,
  FeedResponse,
  SharedInsight,
  ShareInsightRequest,
  ShareInsightResponse,
  ListInsightsResponse,
  ListAdaptedInsightsResponse,
  ListInsightsParams,
  ReactionType,
  ReactionResponse,
  AdaptInsightResponse,
  SocialSettingsResponse,
  UpdateSocialSettingsRequest,
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
    // Validate token before storing - prevents cryptic AsyncStorage errors
    if (!token) {
      throw new Error('Authentication failed: Server did not return an access token. Please check that the Pierre server is running on the correct port.');
    }
    this.jwtToken = token;
    this.csrfToken = csrfToken;
    this.userId = user.user_id;
    await Promise.all([
      AsyncStorage.setItem(STORAGE_KEYS.JWT_TOKEN, token),
      AsyncStorage.setItem(STORAGE_KEYS.CSRF_TOKEN, csrfToken || ''),
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
   * List hidden coaches for the user (includes all coaches with hidden ones)
   */
  async listHiddenCoaches(): Promise<ListCoachesResponse> {
    const response = await axios.get('/api/coaches?include_hidden=true');
    return response.data;
  }

  /**
   * Get only the hidden coaches (for hidden coaches count/filter)
   */
  async getHiddenCoaches(): Promise<ListCoachesResponse> {
    const response = await axios.get('/api/coaches/hidden');
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

  // ==========================================
  // Store API endpoints (Coach Store)
  // ==========================================

  /**
   * Browse published coaches in the Store with cursor-based pagination
   */
  async browseStoreCoaches(options?: {
    category?: string;
    sort_by?: 'newest' | 'popular' | 'title';
    limit?: number;
    cursor?: string;
  }): Promise<BrowseCoachesResponse> {
    const params = new URLSearchParams();
    if (options?.category) params.append('category', options.category);
    if (options?.sort_by) params.append('sort_by', options.sort_by);
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.cursor) params.append('cursor', options.cursor);
    const url = params.toString()
      ? `/api/store/coaches?${params}`
      : '/api/store/coaches';
    const response = await axios.get(url);
    return response.data;
  }

  /**
   * Search published coaches in the Store
   */
  async searchStoreCoaches(
    query: string,
    limit?: number
  ): Promise<SearchCoachesResponse> {
    const params = new URLSearchParams({ q: query });
    if (limit) params.append('limit', limit.toString());
    const response = await axios.get(`/api/store/search?${params}`);
    return response.data;
  }

  /**
   * Get Store categories with coach counts
   */
  async getStoreCategories(): Promise<CategoriesResponse> {
    const response = await axios.get('/api/store/categories');
    return response.data;
  }

  /**
   * Get details of a Store coach by ID
   */
  async getStoreCoach(coachId: string): Promise<StoreCoachDetail> {
    const response = await axios.get(`/api/store/coaches/${coachId}`);
    return response.data;
  }

  /**
   * Install a coach from the Store (creates user's copy)
   */
  async installStoreCoach(coachId: string): Promise<InstallCoachResponse> {
    const response = await axios.post(`/api/store/coaches/${coachId}/install`);
    return response.data;
  }

  /**
   * Uninstall a coach (delete user's installed copy)
   */
  async uninstallStoreCoach(coachId: string): Promise<UninstallCoachResponse> {
    const response = await axios.delete(`/api/store/coaches/${coachId}/install`);
    return response.data;
  }

  /**
   * Get user's installed coaches from the Store
   */
  async getInstalledCoaches(): Promise<InstallationsResponse> {
    const response = await axios.get('/api/store/installations');
    return response.data;
  }

  // WebSocket URL for chat streaming
  getWebSocketUrl(conversationId: string): string {
    const wsBase = API_BASE_URL.replace(/^http/, 'ws');
    return `${wsBase}/api/chat/ws/${conversationId}?token=${this.jwtToken}`;
  }

  // ==========================================
  // Social API endpoints (Coach-Mediated Sharing)
  // ==========================================

  // ---------- Friends ----------

  /**
   * List current user's friends
   */
  async listFriends(): Promise<ListFriendsResponse> {
    const response = await axios.get('/api/social/friends');
    return response.data;
  }

  /**
   * Get pending friend requests (sent and received)
   */
  async getPendingRequests(): Promise<PendingRequestsResponse> {
    const response = await axios.get('/api/social/friends/pending');
    return response.data;
  }

  /**
   * Send a friend request to another user
   */
  async sendFriendRequest(receiverId: string): Promise<FriendConnectionResponse> {
    const response = await axios.post('/api/social/friends', {
      receiver_id: receiverId,
    });
    return response.data;
  }

  /**
   * Accept a friend request
   */
  async acceptFriendRequest(connectionId: string): Promise<FriendConnectionResponse> {
    const response = await axios.put(`/api/social/friends/${connectionId}`, {
      action: 'accept',
    });
    return response.data;
  }

  /**
   * Decline a friend request
   */
  async declineFriendRequest(connectionId: string): Promise<void> {
    await axios.put(`/api/social/friends/${connectionId}`, {
      action: 'decline',
    });
  }

  /**
   * Remove a friend (unfriend)
   */
  async removeFriend(connectionId: string): Promise<void> {
    await axios.delete(`/api/social/friends/${connectionId}`);
  }

  /**
   * Block a user
   */
  async blockUser(connectionId: string): Promise<void> {
    await axios.put(`/api/social/friends/${connectionId}`, {
      action: 'block',
    });
  }

  /**
   * Search for users to add as friends
   */
  async searchUsers(query: string, limit?: number): Promise<SearchUsersResponse> {
    const params = new URLSearchParams({ q: query });
    if (limit) params.append('limit', limit.toString());
    const response = await axios.get(`/api/social/users/search?${params}`);
    return response.data;
  }

  // ---------- Feed ----------

  /**
   * Get social feed of friends' shared insights
   */
  async getSocialFeed(options?: {
    limit?: number;
    cursor?: string;
  }): Promise<FeedResponse> {
    const params = new URLSearchParams();
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.cursor) params.append('cursor', options.cursor);
    const url = params.toString() ? `/api/social/feed?${params}` : '/api/social/feed';
    const response = await axios.get(url);
    // Backend returns { insights, total, metadata } but frontend expects { items, ... }
    const data = response.data;
    return {
      items: (data.insights || []).map((insight: SharedInsight) => ({
        insight,
        author: {
          user_id: insight.user_id,
          display_name: null,
          email: 'user@example.com',
        },
        reactions: {
          like: 0,
          celebrate: 0,
          inspire: 0,
          support: 0,
          total: insight.reaction_count || 0,
        },
        user_reaction: null,
        user_has_adapted: false,
      })),
      next_cursor: null,
      has_more: false,
      metadata: data.metadata || { timestamp: new Date().toISOString(), api_version: '1.0' },
    };
  }

  // ---------- Insights ----------

  /**
   * Share a new coach insight
   */
  async shareInsight(data: ShareInsightRequest): Promise<ShareInsightResponse> {
    const response = await axios.post('/api/social/insights', data);
    return response.data;
  }

  /**
   * List user's own shared insights
   */
  async listMyInsights(params?: ListInsightsParams): Promise<ListInsightsResponse> {
    const urlParams = new URLSearchParams();
    if (params?.insight_type) urlParams.append('insight_type', params.insight_type);
    if (params?.visibility) urlParams.append('visibility', params.visibility);
    if (params?.limit) urlParams.append('limit', params.limit.toString());
    if (params?.cursor) urlParams.append('cursor', params.cursor);
    const url = urlParams.toString()
      ? `/api/social/insights?${urlParams}`
      : '/api/social/insights';
    const response = await axios.get(url);
    return response.data;
  }

  /**
   * Delete a shared insight
   */
  async deleteInsight(insightId: string): Promise<void> {
    await axios.delete(`/api/social/insights/${insightId}`);
  }

  // ---------- Reactions ----------

  /**
   * Add a reaction to a shared insight
   */
  async addReaction(insightId: string, reactionType: ReactionType): Promise<ReactionResponse> {
    const response = await axios.post(`/api/social/insights/${insightId}/reactions`, {
      reaction_type: reactionType,
    });
    return response.data;
  }

  /**
   * Remove user's reaction from an insight
   */
  async removeReaction(insightId: string): Promise<void> {
    await axios.delete(`/api/social/insights/${insightId}/reactions`);
  }

  // ---------- Adapt to My Training ----------

  /**
   * Adapt a friend's insight to user's own training context
   */
  async adaptInsight(insightId: string, context?: string): Promise<AdaptInsightResponse> {
    const response = await axios.post(`/api/social/insights/${insightId}/adapt`, {
      context,
    });
    return response.data;
  }

  /**
   * Get user's adapted insights
   */
  async getAdaptedInsights(options?: {
    limit?: number;
    cursor?: string;
  }): Promise<ListAdaptedInsightsResponse> {
    const params = new URLSearchParams();
    if (options?.limit) params.append('limit', options.limit.toString());
    if (options?.cursor) params.append('cursor', options.cursor);
    const url = params.toString()
      ? `/api/social/adapted?${params}`
      : '/api/social/adapted';
    const response = await axios.get(url);
    return response.data;
  }

  // ---------- Social Settings ----------

  /**
   * Get user's social settings
   */
  async getSocialSettings(): Promise<SocialSettingsResponse> {
    const response = await axios.get('/api/social/settings');
    // Backend returns settings directly without wrapper, transform to expected format
    const data = response.data;
    return {
      settings: {
        user_id: data.user_id || '',
        discoverable: data.discoverable ?? true,
        default_visibility: data.default_visibility || 'friends',
        share_activity_types: data.share_activity_types || [],
        notifications: data.notifications || {
          friend_requests: true,
          insight_reactions: true,
          adapted_insights: true,
        },
        created_at: data.created_at || new Date().toISOString(),
        updated_at: data.updated_at || new Date().toISOString(),
      },
      metadata: {
        timestamp: new Date().toISOString(),
        api_version: '1.0',
      },
    };
  }

  /**
   * Update user's social settings
   */
  async updateSocialSettings(data: UpdateSocialSettingsRequest): Promise<SocialSettingsResponse> {
    const response = await axios.put('/api/social/settings', data);
    return response.data;
  }
}

export const apiService = new ApiService();
