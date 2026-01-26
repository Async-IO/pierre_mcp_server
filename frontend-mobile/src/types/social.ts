// ABOUTME: Re-exports shared social types and constants from @pierre packages
// ABOUTME: Maintains backwards compatibility with existing imports

// Re-export all social types from shared package
export type {
  FriendStatus,
  ShareVisibility,
  InsightType,
  ReactionType,
  TrainingPhase,
  FriendConnection,
  FriendWithInfo,
  DiscoverableUser,
  NotificationPreferences,
  UserSocialSettings,
  SharedInsight,
  InsightReaction,
  AdaptedInsight,
  FeedAuthor,
  ReactionCounts,
  FeedItem,
  SocialMetadata,
  ListFriendsResponse,
  PendingRequestsResponse,
  FriendConnectionResponse,
  SearchUsersResponse,
  FeedResponse,
  ShareInsightResponse,
  ListInsightsResponse,
  ListAdaptedInsightsResponse,
  ReactionResponse,
  AdaptInsightResponse,
  SocialSettingsResponse,
  ShareInsightRequest,
  UpdateSocialSettingsRequest,
  ListInsightsParams,
  InsightSuggestion,
  ListSuggestionsResponse,
  ShareFromActivityRequest,
  GetSuggestionsParams,
} from '@pierre/shared-types';

// Re-export all social constants from shared package
export {
  REACTION_EMOJIS,
  INSIGHT_TYPE_COLORS,
  INSIGHT_TYPE_LABELS,
} from '@pierre/shared-constants';
