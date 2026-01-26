// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: TypeScript types for social features (coach-mediated sharing)
// ABOUTME: Friend connections, shared insights, reactions, and adapted insights

// ========== ENUMS ==========

/** Status of a friend connection request */
export type FriendStatus = 'pending' | 'accepted' | 'declined' | 'blocked';

/** Visibility setting for shared insights */
export type ShareVisibility = 'friends_only' | 'public';

/** Type of coach insight that can be shared */
export type InsightType = 'achievement' | 'milestone' | 'training_tip' | 'recovery' | 'motivation' | 'coaching_insight';

/** Types of reactions users can give to shared insights */
export type ReactionType = 'like' | 'celebrate' | 'inspire' | 'support';

/** Training phase context for insights */
export type TrainingPhase = 'base' | 'build' | 'peak' | 'recovery';

// ========== FRIEND TYPES ==========

/** Represents a friend connection between two users */
export interface FriendConnection {
  id: string;
  initiator_id: string;
  receiver_id: string;
  status: FriendStatus;
  created_at: string;
  updated_at: string;
  accepted_at: string | null;
}

/** Friend connection with display info for the other user */
export interface FriendWithInfo extends FriendConnection {
  friend_display_name: string | null;
  friend_email: string;
  friend_user_id: string;
}

/** User discoverable in friend search */
export interface DiscoverableUser {
  user_id: string;
  display_name: string | null;
  email: string;
  mutual_friends_count: number;
  is_friend: boolean;
  pending_request: boolean;
}

// ========== SOCIAL SETTINGS ==========

/** Notification preferences for social features */
export interface NotificationPreferences {
  friend_requests: boolean;
  insight_reactions: boolean;
  adapted_insights: boolean;
}

/** User's social and privacy settings */
export interface UserSocialSettings {
  user_id: string;
  discoverable: boolean;
  default_visibility: ShareVisibility;
  share_activity_types: string[];
  notifications: NotificationPreferences;
  created_at: string;
  updated_at: string;
}

// ========== INSIGHT TYPES ==========

/** A coach insight shared by a user */
export interface SharedInsight {
  id: string;
  user_id: string;
  visibility: ShareVisibility;
  insight_type: InsightType;
  sport_type: string | null;
  content: string;
  title: string | null;
  training_phase: TrainingPhase | null;
  reaction_count: number;
  adapt_count: number;
  created_at: string;
  updated_at: string;
  expires_at: string | null;
  /** Source activity ID (for coach-generated insights) */
  source_activity_id: string | null;
  /** Whether this insight was coach-generated */
  coach_generated: boolean;
}

/** A reaction on a shared insight */
export interface InsightReaction {
  id: string;
  insight_id: string;
  user_id: string;
  reaction_type: ReactionType;
  created_at: string;
}

/** An adapted version of another user's insight */
export interface AdaptedInsight {
  id: string;
  user_id: string;
  source_insight_id: string;
  adapted_content: string;
  adaptation_context: string | null;
  created_at: string;
}

// ========== FEED TYPES ==========

/** Author info for feed display */
export interface FeedAuthor {
  user_id: string;
  display_name: string | null;
  email: string;
}

/** Reaction counts by type */
export interface ReactionCounts {
  like: number;
  celebrate: number;
  inspire: number;
  support: number;
  total: number;
}

/** A feed item combining insight with author and reaction info */
export interface FeedItem {
  insight: SharedInsight;
  author: FeedAuthor;
  reactions: ReactionCounts;
  user_reaction: ReactionType | null;
  user_has_adapted: boolean;
}

// ========== API RESPONSE TYPES ==========

/** Standard metadata for social API responses */
export interface SocialMetadata {
  timestamp: string;
  api_version: string;
}

/** Response for listing friends */
export interface ListFriendsResponse {
  friends: FriendWithInfo[];
  total: number;
  metadata: SocialMetadata;
}

/** Response for pending friend requests */
export interface PendingRequestsResponse {
  sent: FriendConnection[];
  received: FriendConnection[];
  metadata: SocialMetadata;
}

/** Response for a friend connection operation */
export interface FriendConnectionResponse {
  id: string;
  initiator_id: string;
  receiver_id: string;
  status: string;
  created_at: string;
  updated_at: string;
  accepted_at: string | null;
}

/** Response for user search */
export interface SearchUsersResponse {
  users: DiscoverableUser[];
  query: string;
  metadata: SocialMetadata;
}

/** Response for social feed */
export interface FeedResponse {
  items: FeedItem[];
  next_cursor: string | null;
  has_more: boolean;
  metadata: SocialMetadata;
}

/** Response for sharing an insight */
export interface ShareInsightResponse {
  insight: SharedInsight;
  metadata: SocialMetadata;
}

/** Response for listing insights */
export interface ListInsightsResponse {
  insights: SharedInsight[];
  next_cursor: string | null;
  has_more: boolean;
  metadata: SocialMetadata;
}

/** Response for listing adapted insights */
export interface ListAdaptedInsightsResponse {
  insights: AdaptedInsight[];
  next_cursor: string | null;
  has_more: boolean;
  metadata: SocialMetadata;
}

/** Response for adding a reaction */
export interface ReactionResponse {
  reaction: InsightReaction;
  updated_counts: ReactionCounts;
  metadata: SocialMetadata;
}

/** Response for adapting an insight */
export interface AdaptInsightResponse {
  adapted: AdaptedInsight;
  source_insight: SharedInsight;
  metadata: SocialMetadata;
}

/** Response for social settings */
export interface SocialSettingsResponse {
  settings: UserSocialSettings;
  metadata: SocialMetadata;
}

// ========== COACH SUGGESTION TYPES ==========

/** A coach-generated insight suggestion based on user's activities */
export interface InsightSuggestion {
  insight_type: InsightType;
  suggested_content: string;
  suggested_title?: string;
  relevance_score: number;
  sport_type?: string;
  training_phase?: TrainingPhase;
  source_activity_id?: string;
}

/** Response for listing coach suggestions */
export interface ListSuggestionsResponse {
  suggestions: InsightSuggestion[];
  total: number;
  metadata: SocialMetadata;
}

/** Request to share an insight from a coach suggestion */
export interface ShareFromActivityRequest {
  activity_id?: string;
  insight_type: InsightType;
  content?: string;
  visibility?: ShareVisibility;
  provider?: string;
  tenant_id?: string;
}

/** Parameters for getting insight suggestions */
export interface GetSuggestionsParams {
  activity_id?: string;
  limit?: number;
  provider?: string;
  tenant_id?: string;
}

// ========== API REQUEST TYPES ==========

/** Request to share a new insight */
export interface ShareInsightRequest {
  insight_type: InsightType;
  content: string;
  title?: string;
  visibility?: ShareVisibility;
  sport_type?: string;
  training_phase?: TrainingPhase;
}

/** Request to update social settings */
export interface UpdateSocialSettingsRequest {
  discoverable?: boolean;
  default_visibility?: ShareVisibility;
  share_activity_types?: string[];
  notifications?: Partial<NotificationPreferences>;
}

// ========== HELPER CONSTANTS ==========

/** Emoji mapping for reaction types */
export const REACTION_EMOJIS: Record<ReactionType, string> = {
  like: '👍',
  celebrate: '🎉',
  inspire: '💪',
  support: '🤗',
};

/** Color mapping for insight types (Pierre brand colors) */
export const INSIGHT_TYPE_COLORS: Record<InsightType, string> = {
  achievement: '#10B981', // emerald
  milestone: '#F59E0B', // amber
  training_tip: '#6366F1', // indigo
  recovery: '#8B5CF6', // violet
  motivation: '#F97316', // orange
  coaching_insight: '#7C3AED', // pierre-violet
};

/** Labels for insight types */
export const INSIGHT_TYPE_LABELS: Record<InsightType, string> = {
  achievement: 'Achievement',
  milestone: 'Milestone',
  training_tip: 'Training Tip',
  recovery: 'Recovery',
  motivation: 'Motivation',
  coaching_insight: 'Coach Chat',
};

/** Icon names for insight types (for SVG rendering) */
export const INSIGHT_TYPE_ICONS: Record<InsightType, string> = {
  achievement: 'award',
  milestone: 'flag',
  training_tip: 'zap',
  recovery: 'moon',
  motivation: 'sun',
  coaching_insight: 'message-circle',
};
