// ABOUTME: TypeScript types for social features (coach-mediated sharing)
// ABOUTME: Friend connections, shared insights, reactions, and adapted insights

// ========== ENUMS ==========

/** Status of a friend connection request */
export type FriendStatus = 'pending' | 'accepted' | 'declined' | 'blocked';

/** Visibility setting for shared insights */
export type ShareVisibility = 'friends_only' | 'public';

/** Type of coach insight that can be shared */
export type InsightType = 'achievement' | 'milestone' | 'training_tip' | 'recovery' | 'motivation';

/** Types of reactions users can give to shared insights */
export type ReactionType = 'like' | 'celebrate' | 'inspire' | 'support';

/** Training phase context for insights */
export type TrainingPhase = 'base' | 'build' | 'peak' | 'recovery';

// ========== FRIEND TYPES ==========

/** Represents a friend connection between two users */
export interface FriendConnection {
  /** Unique identifier for this connection */
  id: string;
  /** User who initiated the friend request */
  initiator_id: string;
  /** User who received the friend request */
  receiver_id: string;
  /** Current status of the connection */
  status: FriendStatus;
  /** When the request was created */
  created_at: string;
  /** When the connection was last updated */
  updated_at: string;
  /** When the request was accepted (if accepted) */
  accepted_at: string | null;
}

/** Friend connection with display info for the other user */
export interface FriendWithInfo extends FriendConnection {
  /** Display name of the friend */
  friend_display_name: string | null;
  /** Email of the friend (for avatar) */
  friend_email: string;
  /** The friend's user ID (computed from initiator/receiver) */
  friend_user_id: string;
}

/** User discoverable in friend search */
export interface DiscoverableUser {
  /** User ID */
  user_id: string;
  /** Display name (if visible) */
  display_name: string | null;
  /** Email for avatar */
  email: string;
  /** Number of mutual friends */
  mutual_friends_count: number;
  /** Whether user is already a friend */
  is_friend: boolean;
  /** Whether there's a pending request */
  pending_request: boolean;
}

// ========== SOCIAL SETTINGS ==========

/** Notification preferences for social features */
export interface NotificationPreferences {
  /** Receive notifications for friend requests */
  friend_requests: boolean;
  /** Receive notifications for reactions on insights */
  insight_reactions: boolean;
  /** Receive notifications when insights are adapted */
  adapted_insights: boolean;
}

/** User's social and privacy settings */
export interface UserSocialSettings {
  /** User ID these settings belong to */
  user_id: string;
  /** Whether user can be found in friend search */
  discoverable: boolean;
  /** Default visibility for new shared insights */
  default_visibility: ShareVisibility;
  /** Activity types to auto-suggest for sharing */
  share_activity_types: string[];
  /** Notification preferences */
  notifications: NotificationPreferences;
  /** When settings were created */
  created_at: string;
  /** When settings were last updated */
  updated_at: string;
}

// ========== INSIGHT TYPES ==========

/** A coach insight shared by a user */
export interface SharedInsight {
  /** Unique identifier */
  id: string;
  /** User who shared this insight */
  user_id: string;
  /** Visibility setting */
  visibility: ShareVisibility;
  /** Type of insight */
  insight_type: InsightType;
  /** Sport type context (optional) */
  sport_type: string | null;
  /** The shareable content (sanitized, no private data) */
  content: string;
  /** Optional title */
  title: string | null;
  /** Training phase context */
  training_phase: TrainingPhase | null;
  /** Number of reactions received */
  reaction_count: number;
  /** Number of times adapted by others */
  adapt_count: number;
  /** When the insight was shared */
  created_at: string;
  /** When the insight was last updated */
  updated_at: string;
  /** When the insight expires (optional) */
  expires_at: string | null;
}

/** A reaction on a shared insight */
export interface InsightReaction {
  /** Unique identifier */
  id: string;
  /** The insight being reacted to */
  insight_id: string;
  /** User who reacted */
  user_id: string;
  /** Type of reaction */
  reaction_type: ReactionType;
  /** When the reaction was created */
  created_at: string;
}

/** An adapted version of another user's insight */
export interface AdaptedInsight {
  /** Unique identifier */
  id: string;
  /** User who requested the adaptation */
  user_id: string;
  /** The original shared insight ID */
  source_insight_id: string;
  /** The personalized adaptation content */
  adapted_content: string;
  /** Context about how it was adapted */
  adaptation_context: string | null;
  /** When the adaptation was created */
  created_at: string;
}

// ========== FEED TYPES ==========

/** Author info for feed display */
export interface FeedAuthor {
  /** Author's user ID */
  user_id: string;
  /** Display name */
  display_name: string | null;
  /** Email for avatar */
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
  /** The shared insight */
  insight: SharedInsight;
  /** Author information */
  author: FeedAuthor;
  /** Reaction counts */
  reactions: ReactionCounts;
  /** Current user's reaction (if any) */
  user_reaction: ReactionType | null;
  /** Whether current user has adapted this insight */
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

// ========== API REQUEST TYPES ==========

/** Request to share a new insight */
export interface ShareInsightRequest {
  /** Type of insight */
  insight_type: InsightType;
  /** The shareable content */
  content: string;
  /** Optional title */
  title?: string;
  /** Visibility setting */
  visibility?: ShareVisibility;
  /** Sport type context */
  sport_type?: string;
  /** Training phase context */
  training_phase?: TrainingPhase;
}

/** Request to update social settings */
export interface UpdateSocialSettingsRequest {
  /** Whether user can be found in friend search */
  discoverable?: boolean;
  /** Default visibility for new shared insights */
  default_visibility?: ShareVisibility;
  /** Activity types to auto-suggest for sharing */
  share_activity_types?: string[];
  /** Notification preferences */
  notifications?: Partial<NotificationPreferences>;
}

/** Parameters for listing insights */
export interface ListInsightsParams {
  /** Filter by insight type */
  insight_type?: InsightType;
  /** Filter by visibility */
  visibility?: ShareVisibility;
  /** Limit results */
  limit?: number;
  /** Cursor for pagination */
  cursor?: string;
}

// ========== HELPER TYPES ==========

/** Emoji mapping for reaction types */
export const REACTION_EMOJIS: Record<ReactionType, string> = {
  like: '👍',
  celebrate: '🎉',
  inspire: '💪',
  support: '🤗',
};

/** Color mapping for insight types */
export const INSIGHT_TYPE_COLORS: Record<InsightType, string> = {
  achievement: '#10B981', // emerald
  milestone: '#F59E0B', // amber
  training_tip: '#3B82F6', // blue
  recovery: '#6366F1', // indigo
  motivation: '#EC4899', // pink
};

/** Labels for insight types */
export const INSIGHT_TYPE_LABELS: Record<InsightType, string> = {
  achievement: 'Achievement',
  milestone: 'Milestone',
  training_tip: 'Training Tip',
  recovery: 'Recovery',
  motivation: 'Motivation',
};
