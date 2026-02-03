// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Social domain API - friends, feed, insights, reactions
// ABOUTME: Handles social features including friend management and insight sharing

import type { AxiosInstance } from 'axios';
import type {
  FriendWithInfo,
  FriendConnection,
  FeedItem,
  SharedInsight,
  AdaptedInsight,
  InsightReaction,
  UserSocialSettings,
  DiscoverableUser,
  InsightType,
  ListFriendsResponse,
  PendingRequestsResponse as SharedPendingRequestsResponse,
  FeedResponse as SharedFeedResponse,
  ListInsightsResponse,
  ListAdaptedInsightsResponse,
  SocialSettingsResponse as SharedSocialSettingsResponse,
  SearchUsersResponse,
  ShareInsightRequest,
  AdaptInsightResponse,
  ListSuggestionsResponse,
  ShareFromActivityRequest,
  GetSuggestionsParams,
} from '@pierre/shared-types';
import { ENDPOINTS } from '../core/endpoints';

// Re-export types for consumers
export type {
  FriendWithInfo,
  FriendConnection,
  FeedItem,
  SharedInsight,
  AdaptedInsight,
  InsightReaction,
  UserSocialSettings,
  DiscoverableUser,
  InsightType,
  ShareInsightRequest,
};

// Type aliases for backward compatibility
type Friend = FriendWithInfo;
type FriendRequest = FriendConnection;
type Reaction = InsightReaction;
type SocialSettings = UserSocialSettings;

// Response type aliases using shared-types
export type FriendsResponse = ListFriendsResponse;
export type FriendRequestsResponse = SharedPendingRequestsResponse;
export type FeedResponse = SharedFeedResponse;
export type InsightsResponse = ListInsightsResponse;
export type AdaptedInsightsResponse = ListAdaptedInsightsResponse;
export type SocialSettingsResponse = SharedSocialSettingsResponse;
export type UserSearchResponse = SearchUsersResponse;

// Custom response types for specific operations
export interface InsightResponse {
  insight: SharedInsight;
}

/**
 * Creates the social API methods bound to an axios instance.
 */
export function createSocialApi(axios: AxiosInstance) {
  const api = {
    // ==================== FRIENDS ====================

    /**
     * List all friends.
     */
    async listFriends(): Promise<FriendsResponse> {
      const response = await axios.get<FriendsResponse>(ENDPOINTS.SOCIAL.FRIENDS);
      return response.data;
    },

    /**
     * Get pending friend requests (received).
     */
    async getPendingRequests(): Promise<FriendRequestsResponse> {
      const response = await axios.get<FriendRequestsResponse>(ENDPOINTS.SOCIAL.FRIENDS_PENDING);
      return response.data;
    },

    /**
     * Send a friend request.
     */
    async sendFriendRequest(userId: string): Promise<{ request: FriendRequest }> {
      const response = await axios.post<{ request: FriendRequest }>(
        ENDPOINTS.SOCIAL.FRIENDS_REQUESTS,
        { user_id: userId }
      );
      return response.data;
    },

    /**
     * Accept a friend request.
     */
    async acceptFriendRequest(requestId: string): Promise<{ friend: Friend }> {
      const response = await axios.post<{ friend: Friend }>(
        ENDPOINTS.SOCIAL.FRIEND_REQUEST_ACCEPT(requestId)
      );
      return response.data;
    },

    /**
     * Decline/reject a friend request.
     */
    async declineFriendRequest(requestId: string): Promise<void> {
      await axios.post(ENDPOINTS.SOCIAL.FRIEND_REQUEST_REJECT(requestId));
    },

    /**
     * Remove a friend.
     */
    async removeFriend(friendId: string): Promise<void> {
      await axios.delete(ENDPOINTS.SOCIAL.FRIEND(friendId));
    },

    /**
     * Block a user.
     */
    async blockUser(userId: string): Promise<void> {
      await axios.post(ENDPOINTS.SOCIAL.FRIEND_BLOCK(userId));
    },

    /**
     * Search for users.
     */
    async searchUsers(query: string, limit?: number): Promise<UserSearchResponse> {
      const params = new URLSearchParams();
      params.append('q', query);
      if (limit) params.append('limit', limit.toString());
      const response = await axios.get<UserSearchResponse>(
        `${ENDPOINTS.SOCIAL.USER_SEARCH}?${params}`
      );
      return response.data;
    },

    // ==================== FEED ====================

    /**
     * Get the social feed.
     */
    async getFeed(options?: { cursor?: string; limit?: number }): Promise<FeedResponse> {
      const params = new URLSearchParams();
      if (options?.cursor) params.append('cursor', options.cursor);
      if (options?.limit) params.append('limit', options.limit.toString());

      const queryString = params.toString();
      const url = queryString ? `${ENDPOINTS.SOCIAL.FEED}?${queryString}` : ENDPOINTS.SOCIAL.FEED;

      const response = await axios.get<FeedResponse>(url);
      return response.data;
    },

    // ==================== INSIGHTS ====================

    /**
     * Share an insight.
     */
    async shareInsight(request: ShareInsightRequest): Promise<InsightResponse> {
      const response = await axios.post<InsightResponse>(ENDPOINTS.SOCIAL.SHARE, request);
      return response.data;
    },

    /**
     * List insights shared by the current user.
     */
    async listMyInsights(): Promise<InsightsResponse> {
      const response = await axios.get<InsightsResponse>(ENDPOINTS.SOCIAL.INSIGHTS);
      return response.data;
    },

    /**
     * Delete a shared insight.
     */
    async deleteInsight(insightId: string): Promise<void> {
      await axios.delete(ENDPOINTS.SOCIAL.INSIGHT(insightId));
    },

    // ==================== REACTIONS ====================

    /**
     * Add a reaction to an insight.
     */
    async addReaction(
      insightId: string,
      reactionType: string
    ): Promise<{ reaction: Reaction }> {
      const response = await axios.post<{ reaction: Reaction }>(
        ENDPOINTS.SOCIAL.INSIGHT_REACTIONS(insightId),
        { reaction_type: reactionType }
      );
      return response.data;
    },

    /**
     * Remove a reaction from an insight.
     * @param insightId - The insight ID
     * @param _reactionType - Optional reaction type (for backward compatibility, not used by backend)
     */
    async removeReaction(insightId: string, _reactionType?: string): Promise<void> {
      await axios.delete(ENDPOINTS.SOCIAL.INSIGHT_REACTIONS(insightId));
    },

    // ==================== ADAPT ====================

    /**
     * Adapt an insight for personal use.
     */
    async adaptInsight(insightId: string, context?: string): Promise<AdaptInsightResponse> {
      const response = await axios.post<AdaptInsightResponse>(
        ENDPOINTS.SOCIAL.INSIGHT_ADAPT(insightId),
        { context }
      );
      return response.data;
    },

    /**
     * Get adapted insights.
     */
    async getAdaptedInsights(options?: {
      limit?: number;
      cursor?: string;
    }): Promise<AdaptedInsightsResponse> {
      const params = new URLSearchParams();
      if (options?.limit) params.append('limit', options.limit.toString());
      if (options?.cursor) params.append('cursor', options.cursor);
      const queryString = params.toString();
      const url = queryString ? `${ENDPOINTS.SOCIAL.ADAPTED}?${queryString}` : ENDPOINTS.SOCIAL.ADAPTED;
      const response = await axios.get<AdaptedInsightsResponse>(url);
      return response.data;
    },

    // ==================== SETTINGS ====================

    /**
     * Get social settings.
     */
    async getSettings(): Promise<SocialSettingsResponse> {
      const response = await axios.get<SocialSettingsResponse>(ENDPOINTS.SOCIAL.SETTINGS);
      return response.data;
    },

    /**
     * Update social settings.
     */
    async updateSettings(settings: Partial<SocialSettings>): Promise<SocialSettingsResponse> {
      const response = await axios.put<SocialSettingsResponse>(ENDPOINTS.SOCIAL.SETTINGS, settings);
      return response.data;
    },

    // ==================== SUGGESTIONS ====================

    /**
     * Get coach-generated insight suggestions based on user's activities.
     */
    async getInsightSuggestions(params?: GetSuggestionsParams): Promise<ListSuggestionsResponse> {
      const urlParams = new URLSearchParams();
      if (params?.activity_id) urlParams.append('activity_id', params.activity_id);
      if (params?.limit) urlParams.append('limit', params.limit.toString());
      if (params?.provider) urlParams.append('provider', params.provider);
      if (params?.tenant_id) urlParams.append('tenant_id', params.tenant_id);
      const queryString = urlParams.toString();
      const url = queryString
        ? `${ENDPOINTS.SOCIAL.SUGGESTIONS}?${queryString}`
        : ENDPOINTS.SOCIAL.SUGGESTIONS;
      const response = await axios.get<ListSuggestionsResponse>(url);
      return response.data;
    },

    /**
     * Share an insight from an activity (coach-mediated).
     */
    async shareFromActivity(data: ShareFromActivityRequest): Promise<InsightResponse> {
      const response = await axios.post<InsightResponse>(ENDPOINTS.SOCIAL.FROM_ACTIVITY, data);
      return response.data;
    },

    /**
     * Generate a shareable insight from analysis content.
     * Uses the backend insight generation prompt to transform analysis into
     * a concise, inspiring social post format.
     */
    async generateInsight(content: string): Promise<{ content: string }> {
      const response = await axios.post<{ content: string; metadata: unknown }>(
        ENDPOINTS.SOCIAL.GENERATE,
        { content }
      );
      return { content: response.data.content };
    },
  };

  // Add aliases for backward compatibility
  return {
    ...api,
    // Aliases
    getSocialFeed: api.getFeed,
    getPendingFriendRequests: api.getPendingRequests,
    rejectFriendRequest: api.declineFriendRequest,
    deleteSharedInsight: api.deleteInsight,
    getSocialSettings: api.getSettings,
    updateSocialSettings: api.updateSettings,
  };
}

export type SocialApi = ReturnType<typeof createSocialApi>;
