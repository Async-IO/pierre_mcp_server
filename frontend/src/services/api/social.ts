// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Social features API methods - friends, feed, insights, reactions
// ABOUTME: Handles social sharing and community features

import { axios } from './client';

export const socialApi = {
  async listFriends(cursor?: string, limit?: number): Promise<{
    friends: Array<{
      id: string;
      initiator_id: string;
      receiver_id: string;
      status: string;
      created_at: string;
      updated_at: string;
      accepted_at: string | null;
      friend_display_name: string | null;
      friend_email: string;
      friend_user_id: string;
    }>;
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    if (cursor) params.append('cursor', cursor);
    if (limit) params.append('limit', limit.toString());
    const query = params.toString();
    const response = await axios.get(`/api/social/friends${query ? `?${query}` : ''}`);
    return response.data;
  },

  async searchUsers(query: string): Promise<{
    users: Array<{
      id: string;
      display_name: string | null;
      email: string;
      is_friend: boolean;
      has_pending_request: boolean;
    }>;
    total: number;
  }> {
    const response = await axios.get(`/api/social/users/search?q=${encodeURIComponent(query)}`);
    return response.data;
  },

  async getPendingFriendRequests(): Promise<{
    sent: Array<{
      id: string;
      initiator_id: string;
      receiver_id: string;
      status: string;
      created_at: string;
      updated_at: string;
      accepted_at: string | null;
      user_display_name: string | null;
      user_email: string;
      user_id: string;
    }>;
    received: Array<{
      id: string;
      initiator_id: string;
      receiver_id: string;
      status: string;
      created_at: string;
      updated_at: string;
      accepted_at: string | null;
      user_display_name: string | null;
      user_email: string;
      user_id: string;
    }>;
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.get('/api/social/friends/pending');
    return response.data;
  },

  async sendFriendRequest(userId: string): Promise<{
    id: string;
    initiator_id: string;
    receiver_id: string;
    status: string;
    created_at: string;
    updated_at: string;
    accepted_at: string | null;
  }> {
    const response = await axios.post('/api/social/friends', { receiver_id: userId });
    return response.data;
  },

  async acceptFriendRequest(connectionId: string): Promise<{
    id: string;
    initiator_id: string;
    receiver_id: string;
    status: string;
    created_at: string;
    updated_at: string;
    accepted_at: string | null;
  }> {
    const response = await axios.post(`/api/social/friends/${connectionId}/accept`);
    return response.data;
  },

  async rejectFriendRequest(connectionId: string): Promise<void> {
    await axios.post(`/api/social/friends/${connectionId}/decline`);
  },

  async removeFriend(userId: string): Promise<void> {
    await axios.delete(`/api/social/friends/${userId}`);
  },

  async blockUser(userId: string): Promise<void> {
    await axios.post(`/api/social/friends/${userId}/block`);
  },

  async getSocialFeed(cursor?: string, limit?: number): Promise<{
    items: Array<{
      insight: {
        id: string;
        user_id: string;
        visibility: string;
        insight_type: string;
        sport_type: string | null;
        content: string;
        title: string | null;
        training_phase: string | null;
        reaction_count: number;
        adapt_count: number;
        created_at: string;
        updated_at: string;
        expires_at: string | null;
      };
      author: {
        user_id: string;
        display_name: string | null;
        email: string;
      };
      reactions: {
        like: number;
        celebrate: number;
        inspire: number;
        support: number;
        total: number;
      };
      user_reaction: string | null;
      user_has_adapted: boolean;
    }>;
    next_cursor: string | null;
    has_more: boolean;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    if (cursor) params.append('cursor', cursor);
    if (limit) params.append('limit', limit.toString());
    const query = params.toString();
    const response = await axios.get(`/api/social/feed${query ? `?${query}` : ''}`);
    // Transform backend response (insights array) to frontend format (items array)
    const data = response.data;
    return {
      items: (data.insights || []).map((insight: Record<string, unknown>) => ({
        insight: {
          ...insight,
          // Ensure required fields have default values
          source_activity_id: insight.source_activity_id ?? null,
          coach_generated: insight.coach_generated ?? false,
        },
        author: {
          user_id: insight.user_id as string,
          display_name: null,
          email: 'user@example.com', // Placeholder - backend should include author info
        },
        reactions: {
          like: 0,
          celebrate: 0,
          inspire: 0,
          support: 0,
          total: (insight.reaction_count as number) || 0,
        },
        user_reaction: null,
        user_has_adapted: false,
      })),
      next_cursor: null,
      has_more: false,
      metadata: data.metadata,
    };
  },

  async shareInsight(request: {
    insight_type: string;
    content: string;
    title?: string;
    visibility?: string;
    sport_type?: string;
    training_phase?: string;
  }): Promise<{
    insight: {
      id: string;
      user_id: string;
      visibility: string;
      insight_type: string;
      sport_type: string | null;
      content: string;
      title: string | null;
      training_phase: string | null;
      reaction_count: number;
      adapt_count: number;
      created_at: string;
      updated_at: string;
      expires_at: string | null;
    };
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.post('/api/social/share', request);
    return response.data;
  },

  async deleteSharedInsight(insightId: string): Promise<void> {
    await axios.delete(`/api/social/share/${insightId}`);
  },

  async addReaction(insightId: string, reactionType: string): Promise<{
    reaction: {
      id: string;
      insight_id: string;
      user_id: string;
      reaction_type: string;
      created_at: string;
    };
    updated_counts: {
      like: number;
      celebrate: number;
      inspire: number;
      support: number;
      total: number;
    };
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.post(`/api/social/insights/${insightId}/reactions`, {
      reaction_type: reactionType,
    });
    return response.data;
  },

  async removeReaction(insightId: string): Promise<void> {
    await axios.delete(`/api/social/insights/${insightId}/reactions`);
  },

  async adaptInsight(insightId: string): Promise<{
    adapted: {
      id: string;
      user_id: string;
      source_insight_id: string;
      adapted_content: string;
      adaptation_context: string | null;
      created_at: string;
    };
    source_insight: {
      id: string;
      user_id: string;
      visibility: string;
      insight_type: string;
      sport_type: string | null;
      content: string;
      title: string | null;
      training_phase: string | null;
      reaction_count: number;
      adapt_count: number;
      created_at: string;
      updated_at: string;
      expires_at: string | null;
    };
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.post(`/api/social/adapt/${insightId}`);
    return response.data;
  },

  async getAdaptedInsights(cursor?: string, limit?: number): Promise<{
    insights: Array<{
      id: string;
      user_id: string;
      source_insight_id: string;
      adapted_content: string;
      adaptation_context: string | null;
      created_at: string;
    }>;
    next_cursor: string | null;
    has_more: boolean;
    metadata: { timestamp: string; api_version: string };
  }> {
    const params = new URLSearchParams();
    if (cursor) params.append('cursor', cursor);
    if (limit) params.append('limit', limit.toString());
    const query = params.toString();
    const response = await axios.get(`/api/social/adapted${query ? `?${query}` : ''}`);
    return response.data;
  },

  async getInsightSuggestions(params?: {
    activity_id?: string;
    limit?: number;
    provider?: string;
    tenant_id?: string;
  }): Promise<{
    suggestions: Array<{
      insight_type: string;
      suggested_content: string;
      suggested_title?: string;
      relevance_score: number;
      sport_type?: string;
      training_phase?: string;
      source_activity_id?: string;
    }>;
    total: number;
    metadata: { timestamp: string; api_version: string };
  }> {
    const urlParams = new URLSearchParams();
    if (params?.activity_id) urlParams.append('activity_id', params.activity_id);
    if (params?.limit) urlParams.append('limit', params.limit.toString());
    if (params?.provider) urlParams.append('provider', params.provider);
    if (params?.tenant_id) urlParams.append('tenant_id', params.tenant_id);
    const query = urlParams.toString();
    const url = query ? `/api/social/insights/suggestions?${query}` : '/api/social/insights/suggestions';
    const response = await axios.get(url);
    return response.data;
  },

  async shareFromActivity(data: {
    activity_id?: string;
    insight_type: string;
    content?: string;
    visibility?: string;
    provider?: string;
    tenant_id?: string;
  }): Promise<{
    insight: {
      id: string;
      user_id: string;
      visibility: string;
      insight_type: string;
      sport_type: string | null;
      content: string;
      title: string | null;
      training_phase: string | null;
      reaction_count: number;
      adapt_count: number;
      created_at: string;
      updated_at: string;
      expires_at: string | null;
    };
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.post('/api/social/insights/from-activity', data);
    return response.data;
  },

  async getSocialSettings(): Promise<{
    settings: {
      user_id: string;
      discoverable: boolean;
      default_visibility: string;
      share_activity_types: string[];
      notifications: {
        friend_requests: boolean;
        insight_reactions: boolean;
        adapted_insights: boolean;
      };
      created_at: string;
      updated_at: string;
    };
    metadata: { timestamp: string; api_version: string };
  }> {
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
  },

  async updateSocialSettings(request: {
    discoverable?: boolean;
    default_visibility?: string;
    share_activity_types?: string[];
    notifications?: {
      friend_requests?: boolean;
      insight_reactions?: boolean;
      adapted_insights?: boolean;
    };
  }): Promise<{
    settings: {
      user_id: string;
      discoverable: boolean;
      default_visibility: string;
      share_activity_types: string[];
      notifications: {
        friend_requests: boolean;
        insight_reactions: boolean;
        adapted_insights: boolean;
      };
      created_at: string;
      updated_at: string;
    };
    metadata: { timestamp: string; api_version: string };
  }> {
    const response = await axios.put('/api/social/settings', request);
    return response.data;
  },
};
