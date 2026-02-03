// ABOUTME: Route handlers for Social Features REST API (coach-mediated sharing)
// ABOUTME: Friend connections, shared insights, reactions, and social feed
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Social routes
//!
//! This module handles social feature endpoints for coach-mediated sharing.
//! All endpoints require JWT authentication to identify the user.

use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthResult,
    config::{
        environment::{default_provider, get_oauth_config},
        SocialInsightsConfig,
    },
    database::social::SocialManager,
    database_plugins::DatabaseProvider,
    errors::{AppError, ErrorCode},
    intelligence::{
        insight_adapter::{InsightAdapter, UserTrainingContext},
        insight_validation::{validate_insight_with_policy, ValidationVerdict},
        social_insights::{
            InsightContextBuilder, InsightGenerationContext, InsightSuggestion,
            SharedInsightGenerator,
        },
    },
    llm::{get_insight_generation_prompt, ChatMessage, ChatProvider, ChatRequest},
    mcp::resources::ServerResources,
    models::{
        Activity, AdaptedInsight, FriendConnection, FriendStatus, InsightReaction, InsightType,
        ReactionType, ShareVisibility, SharedInsight, TrainingPhase, UserSocialSettings,
    },
    protocols::universal::auth_service::{AuthService, TokenData},
    providers::{core::FitnessProvider, OAuth2Credentials, ProviderRegistry},
    security::cookies::get_cookie_value,
};

// ============================================================================
// Response Types
// ============================================================================

/// Response for a friend connection
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FriendConnectionResponse {
    /// Connection ID
    pub id: String,
    /// User who initiated the request
    pub initiator_id: String,
    /// User who received the request
    pub receiver_id: String,
    /// Current status
    pub status: String,
    /// When the request was created
    pub created_at: String,
    /// When the connection was last updated
    pub updated_at: String,
    /// When the request was accepted (if accepted)
    pub accepted_at: Option<String>,
}

impl From<FriendConnection> for FriendConnectionResponse {
    fn from(conn: FriendConnection) -> Self {
        Self {
            id: conn.id.to_string(),
            initiator_id: conn.initiator_id.to_string(),
            receiver_id: conn.receiver_id.to_string(),
            status: conn.status.as_str().to_owned(),
            created_at: conn.created_at.to_rfc3339(),
            updated_at: conn.updated_at.to_rfc3339(),
            accepted_at: conn.accepted_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Response for a friend connection with user info
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FriendWithInfoResponse {
    /// Connection ID
    pub id: String,
    /// User who initiated the request
    pub initiator_id: String,
    /// User who received the request
    pub receiver_id: String,
    /// Current status
    pub status: String,
    /// When the request was created
    pub created_at: String,
    /// When the connection was last updated
    pub updated_at: String,
    /// When the request was accepted (if accepted)
    pub accepted_at: Option<String>,
    /// Friend's display name
    pub friend_display_name: Option<String>,
    /// Friend's email
    pub friend_email: String,
    /// Friend's user ID
    pub friend_user_id: String,
}

/// Response for listing friends
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListFriendsResponse {
    /// List of friend connections with user info
    pub friends: Vec<FriendWithInfoResponse>,
    /// Total count
    pub total: usize,
    /// Cursor for next page (if any)
    pub next_cursor: Option<String>,
    /// Whether more items are available
    pub has_more: bool,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Response for a pending friend request with user info
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PendingRequestWithInfoResponse {
    /// Connection ID
    pub id: String,
    /// User who initiated the request
    pub initiator_id: String,
    /// User who received the request
    pub receiver_id: String,
    /// Current status
    pub status: String,
    /// When the request was created
    pub created_at: String,
    /// When the connection was last updated
    pub updated_at: String,
    /// When the request was accepted (if accepted)
    pub accepted_at: Option<String>,
    /// The other user's display name (initiator for received, receiver for sent)
    pub user_display_name: Option<String>,
    /// The other user's email
    pub user_email: String,
    /// The other user's ID
    pub user_id: String,
}

/// Response for pending friend requests
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PendingRequestsResponse {
    /// Requests sent by the user (includes receiver's info)
    pub sent: Vec<PendingRequestWithInfoResponse>,
    /// Requests received by the user (includes initiator's info)
    pub received: Vec<PendingRequestWithInfoResponse>,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Response for user social settings
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SocialSettingsResponse {
    /// Whether user can be found in search
    pub discoverable: bool,
    /// Default visibility for new insights
    pub default_visibility: String,
    /// Activity types to suggest for sharing
    pub share_activity_types: Vec<String>,
    /// Notification preferences
    pub notifications: NotificationPreferencesResponse,
    /// When settings were created
    pub created_at: String,
    /// When settings were last updated
    pub updated_at: String,
}

/// Notification preferences in response
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct NotificationPreferencesResponse {
    /// Receive notifications for friend requests
    pub friend_requests: bool,
    /// Receive notifications for reactions
    pub insight_reactions: bool,
    /// Receive notifications when insights are adapted
    pub adapted_insights: bool,
}

impl From<UserSocialSettings> for SocialSettingsResponse {
    fn from(settings: UserSocialSettings) -> Self {
        Self {
            discoverable: settings.discoverable,
            default_visibility: settings.default_visibility.as_str().to_owned(),
            share_activity_types: settings.share_activity_types,
            notifications: NotificationPreferencesResponse {
                friend_requests: settings.notifications.friend_requests,
                insight_reactions: settings.notifications.insight_reactions,
                adapted_insights: settings.notifications.adapted_insights,
            },
            created_at: settings.created_at.to_rfc3339(),
            updated_at: settings.updated_at.to_rfc3339(),
        }
    }
}

/// Response for a shared insight
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SharedInsightResponse {
    /// Unique identifier
    pub id: String,
    /// User who shared this insight
    pub user_id: String,
    /// Visibility setting
    pub visibility: String,
    /// Type of insight
    pub insight_type: String,
    /// Sport type context
    pub sport_type: Option<String>,
    /// The shareable content
    pub content: String,
    /// Optional title
    pub title: Option<String>,
    /// Training phase context
    pub training_phase: Option<String>,
    /// Number of reactions received
    pub reaction_count: i32,
    /// Number of times adapted by others
    pub adapt_count: i32,
    /// When the insight was shared
    pub created_at: String,
    /// When the insight was last updated
    pub updated_at: String,
    /// Optional expiry time
    pub expires_at: Option<String>,
    /// Source activity ID that generated this insight (for coach-mediated sharing)
    pub source_activity_id: Option<String>,
    /// Whether this insight was coach-generated (vs manual entry)
    pub coach_generated: bool,
}

impl From<SharedInsight> for SharedInsightResponse {
    fn from(insight: SharedInsight) -> Self {
        Self {
            id: insight.id.to_string(),
            user_id: insight.user_id.to_string(),
            visibility: insight.visibility.as_str().to_owned(),
            insight_type: insight.insight_type.as_str().to_owned(),
            sport_type: insight.sport_type,
            content: insight.content,
            title: insight.title,
            training_phase: insight.training_phase.map(|p| p.as_str().to_owned()),
            reaction_count: insight.reaction_count,
            adapt_count: insight.adapt_count,
            created_at: insight.created_at.to_rfc3339(),
            updated_at: insight.updated_at.to_rfc3339(),
            expires_at: insight.expires_at.map(|dt| dt.to_rfc3339()),
            source_activity_id: insight.source_activity_id,
            coach_generated: insight.coach_generated,
        }
    }
}

/// Response for listing insights
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListInsightsResponse {
    /// List of insights
    pub insights: Vec<SharedInsightResponse>,
    /// Total count
    pub total: usize,
    /// Cursor for next page (if any)
    pub next_cursor: Option<String>,
    /// Whether more items are available
    pub has_more: bool,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Author information for feed display
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FeedAuthorResponse {
    /// User ID
    pub user_id: String,
    /// Display name
    pub display_name: Option<String>,
    /// Email
    pub email: String,
}

/// Reaction counts by type
#[derive(Debug, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ReactionCountsResponse {
    /// Number of likes
    pub like: i32,
    /// Number of celebrations
    pub celebrate: i32,
    /// Number of inspires
    pub inspire: i32,
    /// Number of supports
    pub support: i32,
    /// Total reactions
    pub total: i32,
}

/// A feed item with full metadata
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FeedItemResponse {
    /// The shared insight
    pub insight: SharedInsightResponse,
    /// Author information
    pub author: FeedAuthorResponse,
    /// Reaction counts
    pub reactions: ReactionCountsResponse,
    /// Current user's reaction type (if any)
    pub user_reaction: Option<String>,
    /// Whether current user has adapted this insight
    pub user_has_adapted: bool,
}

/// Response for social feed
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FeedResponse {
    /// Feed items with full metadata
    pub items: Vec<FeedItemResponse>,
    /// Cursor for next page (if any)
    pub next_cursor: Option<String>,
    /// Whether more items are available
    pub has_more: bool,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Response for a reaction
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ReactionResponse {
    /// Reaction ID
    pub id: String,
    /// Insight ID
    pub insight_id: String,
    /// User who reacted
    pub user_id: String,
    /// Type of reaction
    pub reaction_type: String,
    /// When the reaction was created
    pub created_at: String,
}

impl From<InsightReaction> for ReactionResponse {
    fn from(reaction: InsightReaction) -> Self {
        Self {
            id: reaction.id.to_string(),
            insight_id: reaction.insight_id.to_string(),
            user_id: reaction.user_id.to_string(),
            reaction_type: reaction.reaction_type.as_str().to_owned(),
            created_at: reaction.created_at.to_rfc3339(),
        }
    }
}

/// Response for listing reactions
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListReactionsResponse {
    /// List of reactions
    pub reactions: Vec<ReactionResponse>,
    /// Summary by type
    pub summary: ReactionSummaryResponse,
}

/// Summary of reactions by type
#[derive(Debug, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ReactionSummaryResponse {
    /// Number of likes
    pub like_count: i32,
    /// Number of celebrations
    pub celebrate_count: i32,
    /// Number of inspires
    pub inspire_count: i32,
    /// Number of supports
    pub support_count: i32,
    /// Total reactions
    pub total: i32,
}

/// Response for an adapted insight
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AdaptedInsightResponse {
    /// Unique identifier
    pub id: String,
    /// Original insight ID
    pub source_insight_id: String,
    /// User who requested the adaptation
    pub user_id: String,
    /// The personalized content
    pub adapted_content: String,
    /// Context used for adaptation
    pub adaptation_context: Option<String>,
    /// Whether user found this helpful
    pub was_helpful: Option<bool>,
    /// When the adaptation was created
    pub created_at: String,
}

impl From<AdaptedInsight> for AdaptedInsightResponse {
    fn from(insight: AdaptedInsight) -> Self {
        Self {
            id: insight.id.to_string(),
            source_insight_id: insight.source_insight_id.to_string(),
            user_id: insight.user_id.to_string(),
            adapted_content: insight.adapted_content,
            adaptation_context: insight.adaptation_context,
            was_helpful: insight.was_helpful,
            created_at: insight.created_at.to_rfc3339(),
        }
    }
}

/// Response for listing adapted insights
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListAdaptedInsightsResponse {
    /// List of adapted insights
    pub adapted_insights: Vec<AdaptedInsightResponse>,
    /// Total count
    pub total: usize,
    /// Cursor for next page (if any)
    pub next_cursor: Option<String>,
    /// Whether more items are available
    pub has_more: bool,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Response for adapting an insight (includes source insight for context)
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AdaptInsightResultResponse {
    /// The adapted insight
    pub adapted: AdaptedInsightResponse,
    /// The original insight that was adapted
    pub source_insight: SharedInsightResponse,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// User profile for search results
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UserProfileResponse {
    /// User ID
    pub id: String,
    /// Display name
    pub display_name: Option<String>,
    /// Email
    pub email: String,
    /// Whether the current user is friends with this user
    pub is_friend: bool,
    /// Whether there's a pending request
    pub has_pending_request: bool,
}

/// Response for user search
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchUsersResponse {
    /// List of users
    pub users: Vec<UserProfileResponse>,
    /// Total count
    pub total: usize,
}

/// Metadata for social responses
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SocialMetadata {
    /// Response timestamp
    pub timestamp: String,
    /// API version
    pub api_version: String,
}

/// Response for a coach-generated insight suggestion
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct InsightSuggestionResponse {
    /// Type of insight
    pub insight_type: String,
    /// Suggested content (privacy-safe)
    pub suggested_content: String,
    /// Suggested title
    pub suggested_title: Option<String>,
    /// Relevance score (0-100)
    pub relevance_score: u8,
    /// Sport type context
    pub sport_type: Option<String>,
    /// Training phase context
    pub training_phase: Option<String>,
    /// Source activity ID (if suggestion is for specific activity)
    pub source_activity_id: Option<String>,
}

impl From<InsightSuggestion> for InsightSuggestionResponse {
    fn from(suggestion: InsightSuggestion) -> Self {
        Self {
            insight_type: suggestion.insight_type.as_str().to_owned(),
            suggested_content: suggestion.suggested_content,
            suggested_title: suggestion.suggested_title,
            relevance_score: suggestion.relevance_score,
            sport_type: suggestion.sport_type,
            training_phase: suggestion.training_phase.map(|p| p.as_str().to_owned()),
            source_activity_id: None,
        }
    }
}

/// Response for listing insight suggestions
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListSuggestionsResponse {
    /// List of suggestions
    pub suggestions: Vec<InsightSuggestionResponse>,
    /// Total count
    pub total: usize,
    /// Metadata
    pub metadata: SocialMetadata,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request to send a friend request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SendFriendRequestBody {
    /// ID of the user to send request to
    pub receiver_id: String,
}

/// Request to respond to a friend request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RespondFriendRequestBody {
    /// Whether to accept the request
    pub accept: bool,
}

/// Request to update social settings
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateSocialSettingsBody {
    /// Whether user can be found in search
    pub discoverable: Option<bool>,
    /// Default visibility for new insights
    pub default_visibility: Option<String>,
    /// Activity types to suggest for sharing
    pub share_activity_types: Option<Vec<String>>,
    /// Notification preferences
    pub notifications: Option<UpdateNotificationPreferencesBody>,
}

/// Request to update notification preferences
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateNotificationPreferencesBody {
    /// Receive notifications for friend requests
    pub friend_requests: Option<bool>,
    /// Receive notifications for reactions
    pub insight_reactions: Option<bool>,
    /// Receive notifications when insights are adapted
    pub adapted_insights: Option<bool>,
}

/// Request to share an insight
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ShareInsightBody {
    /// Type of insight
    pub insight_type: String,
    /// Content to share
    pub content: String,
    /// Optional title
    pub title: Option<String>,
    /// Visibility setting
    pub visibility: Option<String>,
    /// Sport type context
    pub sport_type: Option<String>,
    /// Training phase context
    pub training_phase: Option<String>,
}

/// Request to generate a shareable insight from analysis content
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GenerateInsightBody {
    /// The analysis content to transform into a shareable insight
    pub content: String,
}

/// Response for generated insight
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GenerateInsightResponse {
    /// The generated shareable content
    pub content: String,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Request to react to an insight
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ReactToInsightBody {
    /// Type of reaction
    pub reaction_type: String,
}

/// Request to adapt an insight
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AdaptInsightBody {
    /// Optional context to include in adaptation
    pub context: Option<String>,
    /// Provider to fetch activities from (defaults to environment default)
    pub provider: Option<String>,
    /// Tenant ID for multi-tenant contexts
    pub tenant_id: Option<String>,
}

/// Request to update helpful status
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateHelpfulBody {
    /// Whether the adaptation was helpful
    pub was_helpful: bool,
}

/// Request to share an insight from an activity (coach-mediated)
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ShareFromActivityBody {
    /// Activity ID that generated the insight (optional for chat-based insights)
    pub activity_id: Option<String>,
    /// Insight type to share
    pub insight_type: String,
    /// User-edited content (optional, uses suggestion if not provided)
    pub content: Option<String>,
    /// Visibility setting
    pub visibility: Option<String>,
    /// Provider to fetch activities from (defaults to environment default)
    pub provider: Option<String>,
    /// Tenant ID for multi-tenant contexts
    pub tenant_id: Option<String>,
}

// ============================================================================
// Query Types
// ============================================================================

/// Query parameters for listing insights
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListInsightsQuery {
    /// Filter by insight type
    pub insight_type: Option<String>,
    /// Maximum results
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Query parameters for user search
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchUsersQuery {
    /// Search query string
    pub q: String,
    /// Maximum results
    pub limit: Option<i64>,
}

/// Query parameters for feed
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FeedQuery {
    /// Maximum results
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Query parameters for insight suggestions
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SuggestionsQuery {
    /// Specific activity ID to generate suggestions for
    pub activity_id: Option<String>,
    /// Maximum suggestions to return
    pub limit: Option<usize>,
    /// Provider to fetch activities from (defaults to environment default)
    pub provider: Option<String>,
    /// Tenant ID for multi-tenant contexts
    pub tenant_id: Option<String>,
    /// Maximum activities to fetch for context generation (capped by server config)
    pub activity_limit: Option<usize>,
}

/// Query parameters for listing friends
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListFriendsQuery {
    /// Maximum results
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Query parameters for listing adapted insights
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListAdaptedQuery {
    /// Maximum results
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

// ============================================================================
// Routes
// ============================================================================

/// Social routes handler
pub struct SocialRoutes;

impl SocialRoutes {
    /// Create all social routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            // Friend connections
            .route("/api/social/friends", get(Self::handle_list_friends))
            .route("/api/social/friends", post(Self::handle_send_request))
            .route(
                "/api/social/friends/pending",
                get(Self::handle_pending_requests),
            )
            .route(
                "/api/social/friends/:id/accept",
                post(Self::handle_accept_request),
            )
            .route(
                "/api/social/friends/:id/decline",
                post(Self::handle_decline_request),
            )
            .route("/api/social/friends/:id", delete(Self::handle_unfriend))
            // Social settings
            .route("/api/social/settings", get(Self::handle_get_settings))
            .route("/api/social/settings", put(Self::handle_update_settings))
            // Insights
            .route("/api/social/insights", get(Self::handle_list_insights))
            .route("/api/social/insights", post(Self::handle_share_insight))
            .route(
                "/api/social/insights/suggestions",
                get(Self::handle_get_suggestions),
            )
            .route(
                "/api/social/insights/from-activity",
                post(Self::handle_share_from_activity),
            )
            .route(
                "/api/social/insights/generate",
                post(Self::handle_generate_insight),
            )
            .route("/api/social/insights/:id", get(Self::handle_get_insight))
            .route(
                "/api/social/insights/:id",
                delete(Self::handle_delete_insight),
            )
            // Reactions
            .route(
                "/api/social/insights/:id/reactions",
                get(Self::handle_list_reactions),
            )
            .route(
                "/api/social/insights/:id/reactions",
                post(Self::handle_add_reaction),
            )
            .route(
                "/api/social/insights/:id/reactions/:reaction_type",
                delete(Self::handle_remove_reaction),
            )
            // Feed
            .route("/api/social/feed", get(Self::handle_get_feed))
            // Adapted insights
            .route(
                "/api/social/insights/:id/adapt",
                post(Self::handle_adapt_insight),
            )
            .route("/api/social/adapted", get(Self::handle_list_adapted))
            .route(
                "/api/social/adapted/:id/helpful",
                put(Self::handle_update_helpful),
            )
            // Discovery
            .route("/api/social/users/search", get(Self::handle_search_users))
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Build metadata for responses
    fn build_metadata() -> SocialMetadata {
        SocialMetadata {
            timestamp: Utc::now().to_rfc3339(),
            api_version: "1.0".to_owned(),
        }
    }

    /// Get social manager from the `SQLite` pool
    fn get_social_manager(resources: &Arc<ServerResources>) -> Result<SocialManager, AppError> {
        let pool = resources
            .database
            .sqlite_pool()
            .ok_or_else(|| AppError::internal("SQLite database required for social features"))?;
        Ok(SocialManager::new(pool.clone()))
    }

    /// Validate content for sharing based on user tier and sharing policy
    ///
    /// Returns the validated (and potentially improved/redacted) content, or an error if rejected.
    async fn validate_content_for_sharing(
        resources: &Arc<ServerResources>,
        social: &SocialManager,
        user_id: Uuid,
        content: &str,
        insight_type: InsightType,
    ) -> Result<String, AppError> {
        // Get user tier
        let user = resources
            .database
            .get_user(user_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        // Get user's sharing policy from social settings
        let policy = social
            .get_user_social_settings(user_id)
            .await?
            .map(|s| s.insight_sharing_policy)
            .unwrap_or_default();

        // Get LLM provider for quality validation
        let llm_provider = ChatProvider::from_env().await?;

        // Run validation
        let result =
            validate_insight_with_policy(&llm_provider, content, insight_type, &user.tier, &policy)
                .await?;

        // Handle validation result
        match result.verdict {
            ValidationVerdict::Valid => Ok(result.final_content),
            ValidationVerdict::Improved { .. } => {
                // Use the improved/redacted content
                Ok(result.final_content)
            }
            ValidationVerdict::Rejected { reason } => Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("Content cannot be shared: {reason}"),
            )),
        }
    }

    // ========================================================================
    // Friend Connections
    // ========================================================================

    /// Handle GET /api/social/friends - List friends
    async fn handle_list_friends(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<ListFriendsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let friends = social
            .get_friends_paginated(auth.user_id, limit, offset)
            .await?;

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        #[allow(clippy::cast_possible_truncation)] // limit is clamped to small values
        let limit_usize = limit as usize;
        let has_more = friends.len() >= limit_usize;
        let next_cursor = if has_more {
            Some((offset + limit).to_string())
        } else {
            None
        };

        // Build response with friend user info
        let mut friends_with_info = Vec::with_capacity(friends.len());
        for conn in friends {
            // Determine who the friend is (the other person in the connection)
            let friend_id = if conn.initiator_id == auth.user_id {
                conn.receiver_id
            } else {
                conn.initiator_id
            };

            // Fetch friend's user info
            let friend_user = resources.database.get_user(friend_id).await?;
            let (friend_display_name, friend_email) = match friend_user {
                Some(user) => (user.display_name, user.email),
                None => (None, format!("user-{friend_id}")),
            };

            friends_with_info.push(FriendWithInfoResponse {
                id: conn.id.to_string(),
                initiator_id: conn.initiator_id.to_string(),
                receiver_id: conn.receiver_id.to_string(),
                status: conn.status.as_str().to_owned(),
                created_at: conn.created_at.to_rfc3339(),
                updated_at: conn.updated_at.to_rfc3339(),
                accepted_at: conn.accepted_at.map(|dt| dt.to_rfc3339()),
                friend_display_name,
                friend_email,
                friend_user_id: friend_id.to_string(),
            });
        }

        let response = ListFriendsResponse {
            total: friends_with_info.len(),
            friends: friends_with_info,
            next_cursor,
            has_more,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/social/friends - Send friend request
    async fn handle_send_request(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<SendFriendRequestBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let receiver_id = Uuid::parse_str(&body.receiver_id)
            .map_err(|_| AppError::invalid_input("Invalid receiver_id format"))?;

        // Check if they're not sending to themselves
        if auth.user_id == receiver_id {
            return Err(AppError::invalid_input(
                "Cannot send friend request to yourself",
            ));
        }

        // Check if connection already exists
        let existing = social
            .get_friend_connection_between(auth.user_id, receiver_id)
            .await?;
        if existing.is_some() {
            return Err(AppError::invalid_input(
                "Friend connection already exists between these users",
            ));
        }

        let connection = FriendConnection::new(auth.user_id, receiver_id);
        social.create_friend_connection(&connection).await?;

        let response: FriendConnectionResponse = connection.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/social/friends/pending - Get pending requests
    async fn handle_pending_requests(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let pending = social.get_pending_friend_requests(auth.user_id).await?;

        let (sent_conns, received_conns): (Vec<_>, Vec<_>) = pending
            .into_iter()
            .partition(|conn| conn.initiator_id == auth.user_id);

        // Build sent requests with receiver's user info
        let mut sent = Vec::with_capacity(sent_conns.len());
        for conn in sent_conns {
            let receiver_id_str = conn.receiver_id.to_string();
            let receiver_user = resources.database.get_user(conn.receiver_id).await?;
            let (user_display_name, user_email) = match receiver_user {
                Some(user) => (user.display_name, user.email),
                None => (None, format!("user-{receiver_id_str}")),
            };

            sent.push(PendingRequestWithInfoResponse {
                id: conn.id.to_string(),
                initiator_id: conn.initiator_id.to_string(),
                receiver_id: conn.receiver_id.to_string(),
                status: conn.status.as_str().to_owned(),
                created_at: conn.created_at.to_rfc3339(),
                updated_at: conn.updated_at.to_rfc3339(),
                accepted_at: conn.accepted_at.map(|dt| dt.to_rfc3339()),
                user_display_name,
                user_email,
                user_id: conn.receiver_id.to_string(),
            });
        }

        // Build received requests with initiator's user info
        let mut received = Vec::with_capacity(received_conns.len());
        for conn in received_conns {
            let initiator_id_str = conn.initiator_id.to_string();
            let initiator_user = resources.database.get_user(conn.initiator_id).await?;
            let (user_display_name, user_email) = match initiator_user {
                Some(user) => (user.display_name, user.email),
                None => (None, format!("user-{initiator_id_str}")),
            };

            received.push(PendingRequestWithInfoResponse {
                id: conn.id.to_string(),
                initiator_id: conn.initiator_id.to_string(),
                receiver_id: conn.receiver_id.to_string(),
                status: conn.status.as_str().to_owned(),
                created_at: conn.created_at.to_rfc3339(),
                updated_at: conn.updated_at.to_rfc3339(),
                accepted_at: conn.accepted_at.map(|dt| dt.to_rfc3339()),
                user_display_name,
                user_email,
                user_id: conn.initiator_id.to_string(),
            });
        }

        let response = PendingRequestsResponse {
            sent,
            received,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/social/friends/:id/accept - Accept friend request
    async fn handle_accept_request(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let connection_id = Uuid::parse_str(&id)
            .map_err(|_| AppError::invalid_input("Invalid connection ID format"))?;

        // Get the connection and verify user can accept it
        let connection = social
            .get_friend_connection(connection_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Friend request {id}")))?;

        // Only receiver can accept
        if connection.receiver_id != auth.user_id {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Only the receiver can accept a friend request",
            ));
        }

        if connection.status != FriendStatus::Pending {
            return Err(AppError::invalid_input(format!(
                "Cannot accept request with status: {}",
                connection.status
            )));
        }

        social
            .update_friend_connection_status(connection_id, FriendStatus::Accepted)
            .await?;

        let updated = social
            .get_friend_connection(connection_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to fetch updated connection"))?;

        let response: FriendConnectionResponse = updated.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/social/friends/:id/decline - Decline friend request
    async fn handle_decline_request(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let connection_id = Uuid::parse_str(&id)
            .map_err(|_| AppError::invalid_input("Invalid connection ID format"))?;

        let connection = social
            .get_friend_connection(connection_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Friend request {id}")))?;

        // Only receiver can decline
        if connection.receiver_id != auth.user_id {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Only the receiver can decline a friend request",
            ));
        }

        if connection.status != FriendStatus::Pending {
            return Err(AppError::invalid_input(format!(
                "Cannot decline request with status: {}",
                connection.status
            )));
        }

        social
            .update_friend_connection_status(connection_id, FriendStatus::Declined)
            .await?;

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    /// Handle DELETE /api/social/friends/:id - Remove friend
    async fn handle_unfriend(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let connection_id = Uuid::parse_str(&id)
            .map_err(|_| AppError::invalid_input("Invalid connection ID format"))?;

        let connection = social
            .get_friend_connection(connection_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Friend connection {id}")))?;

        // Either party can unfriend
        if !connection.involves_user(auth.user_id) {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "You are not part of this connection",
            ));
        }

        social.delete_friend_connection(connection_id).await?;

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    // ========================================================================
    // Social Settings
    // ========================================================================

    /// Handle GET /api/social/settings - Get social settings
    async fn handle_get_settings(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let settings = social
            .get_user_social_settings(auth.user_id)
            .await?
            .unwrap_or_else(|| UserSocialSettings::default_for_user(auth.user_id));

        let response: SocialSettingsResponse = settings.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /api/social/settings - Update social settings
    async fn handle_update_settings(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<UpdateSocialSettingsBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        // Get existing settings or create defaults
        let mut settings = social
            .get_user_social_settings(auth.user_id)
            .await?
            .unwrap_or_else(|| UserSocialSettings::default_for_user(auth.user_id));

        // Apply updates
        if let Some(discoverable) = body.discoverable {
            settings.discoverable = discoverable;
        }
        if let Some(ref visibility) = body.default_visibility {
            settings.default_visibility = ShareVisibility::from_str(visibility)?;
        }
        if let Some(activity_types) = body.share_activity_types {
            settings.share_activity_types = activity_types;
        }
        if let Some(notifications) = body.notifications {
            if let Some(friend_requests) = notifications.friend_requests {
                settings.notifications.friend_requests = friend_requests;
            }
            if let Some(insight_reactions) = notifications.insight_reactions {
                settings.notifications.insight_reactions = insight_reactions;
            }
            if let Some(adapted_insights) = notifications.adapted_insights {
                settings.notifications.adapted_insights = adapted_insights;
            }
        }
        settings.updated_at = Utc::now();

        social.upsert_user_social_settings(&settings).await?;

        let response: SocialSettingsResponse = settings.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    // ========================================================================
    // Shared Insights
    // ========================================================================

    /// Handle GET /api/social/insights - List user's shared insights
    async fn handle_list_insights(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<ListInsightsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_type = query
            .insight_type
            .map(|t| InsightType::from_str(&t))
            .transpose()?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let insights = social
            .get_user_shared_insights(auth.user_id, insight_type, limit, offset)
            .await?;

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        #[allow(clippy::cast_possible_truncation)] // limit is clamped to small values
        let limit_usize = limit as usize;
        let has_more = insights.len() >= limit_usize;
        let next_cursor = if has_more {
            Some((offset + limit).to_string())
        } else {
            None
        };

        let response = ListInsightsResponse {
            total: insights.len(),
            insights: insights.into_iter().map(Into::into).collect(),
            next_cursor,
            has_more,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/social/insights - Share a new insight
    async fn handle_share_insight(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<ShareInsightBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_type = InsightType::from_str(&body.insight_type)?;
        let visibility = body
            .visibility
            .map(|v| ShareVisibility::from_str(&v))
            .transpose()?
            .unwrap_or_default();
        let training_phase = body
            .training_phase
            .map(|p| TrainingPhase::from_str(&p))
            .transpose()?;

        // Validate content before sharing
        let validated_content = Self::validate_content_for_sharing(
            &resources,
            &social,
            auth.user_id,
            &body.content,
            insight_type,
        )
        .await?;

        let mut insight =
            SharedInsight::new(auth.user_id, insight_type, validated_content, visibility);
        insight.title = body.title;
        insight.sport_type = body.sport_type;
        insight.training_phase = training_phase;

        social.create_shared_insight(&insight).await?;

        let response: SharedInsightResponse = insight.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/social/insights/:id - Get a specific insight
    async fn handle_get_insight(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_id =
            Uuid::parse_str(&id).map_err(|_| AppError::invalid_input("Invalid insight ID"))?;

        let insight = social
            .get_shared_insight(insight_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Insight {id}")))?;

        // Check visibility
        let is_friend = if insight.user_id == auth.user_id {
            false
        } else {
            social
                .get_friend_connection_between(auth.user_id, insight.user_id)
                .await?
                .is_some_and(|conn| conn.status.is_connected())
        };

        if !insight.is_visible_to(auth.user_id, is_friend) {
            return Err(AppError::not_found(format!("Insight {id}")));
        }

        let response: SharedInsightResponse = insight.into();
        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle DELETE /api/social/insights/:id - Delete an insight
    async fn handle_delete_insight(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_id =
            Uuid::parse_str(&id).map_err(|_| AppError::invalid_input("Invalid insight ID"))?;

        let insight = social
            .get_shared_insight(insight_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Insight {id}")))?;

        // Only owner can delete
        if insight.user_id != auth.user_id {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "You can only delete your own insights",
            ));
        }

        social.delete_shared_insight(insight_id).await?;

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    // ========================================================================
    // Coach-Mediated Sharing
    // ========================================================================

    /// Handle GET /api/social/insights/suggestions - Get coach suggestions
    ///
    /// Returns suggestions based on user's recent activities. If no activities
    /// can be fetched (e.g., no OAuth token connected), returns an empty list.
    async fn handle_get_suggestions(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<SuggestionsQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        // Use provider from query or fall back to environment default
        let provider_name = query.provider.unwrap_or_else(default_provider);

        // Build insight generation context from user's activities
        // If we can't fetch activities (no OAuth token), return empty suggestions
        let suggestions = match Self::build_insight_context(
            &resources,
            auth.user_id,
            &provider_name,
            query.tenant_id.as_deref(),
            query.activity_limit,
        )
        .await
        {
            Ok(context) => {
                // Generate suggestions using the SharedInsightGenerator
                let generator = SharedInsightGenerator::new();
                let mut suggestions = generator.generate_suggestions(&context);

                // Limit results if requested
                let limit = query.limit.unwrap_or(5);
                suggestions.truncate(limit);

                // Convert to response format, adding activity_id if provided
                suggestions
                    .into_iter()
                    .map(|s| {
                        let mut response: InsightSuggestionResponse = s.into();
                        response.source_activity_id.clone_from(&query.activity_id);
                        response
                    })
                    .collect()
            }
            Err(_) => {
                // No activities available (e.g., no OAuth token) - return empty list
                Vec::new()
            }
        };

        let response = ListSuggestionsResponse {
            total: suggestions.len(),
            suggestions,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/social/insights/from-activity - Share coach-mediated insight
    async fn handle_share_from_activity(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<ShareFromActivityBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        // Generate activity_id for chat-based insights that don't have one
        let activity_id = body
            .activity_id
            .clone()
            .unwrap_or_else(|| format!("chat-insight-{}", Uuid::new_v4()));

        // Check if user has already shared an insight from this activity (skip for generated IDs)
        if body.activity_id.is_some()
            && social
                .has_insight_for_activity(auth.user_id, &activity_id)
                .await?
        {
            return Err(AppError::already_exists(format!(
                "Insight from activity '{activity_id}'"
            )));
        }

        let insight_type = InsightType::from_str(&body.insight_type)?;
        let visibility = body
            .visibility
            .map(|v| ShareVisibility::from_str(&v))
            .transpose()?
            .unwrap_or_default();

        // Use provider from body or fall back to environment default
        let provider_name = body.provider.unwrap_or_else(default_provider);

        // Default message when we can't generate coach content
        let default_message = format!(
            "Sharing a {} from my recent training!",
            insight_type.description()
        );

        // Build content: use custom content if provided, otherwise generate coach content
        // Only validate custom user content - coach-generated content is already quality-controlled
        let (content, is_custom_content) = if let Some(custom_content) = body.content.clone() {
            (custom_content, true)
        } else {
            // Try to generate coach content based on user's activities
            // Falls back to default message if context building fails (e.g., no OAuth token)
            let generated = Self::build_insight_context(
                &resources,
                auth.user_id,
                &provider_name,
                body.tenant_id.as_deref(),
                None, // Use default server limit for share operations
            )
            .await
            .map_or_else(
                |error| {
                    tracing::debug!("Could not build insight context for coach content: {error}");
                    default_message.clone()
                },
                |context| {
                    let generator = SharedInsightGenerator::new();
                    let suggestions = generator.generate_suggestions(&context);

                    // Find a matching suggestion for the insight type
                    suggestions
                        .into_iter()
                        .find(|s| s.insight_type == insight_type)
                        .map_or_else(|| default_message.clone(), |s| s.suggested_content)
                },
            );
            (generated, false)
        };

        // Skip validation for content provided via this endpoint - it's either:
        // 1. Pre-generated by /api/social/insights/generate (already LLM-formatted)
        // 2. Coach-generated from activity context
        // User-written content should use /api/social/insights endpoint which validates
        let validated_content = content;

        // Create the coach-generated insight
        let insight = SharedInsight::coach_generated(
            auth.user_id,
            insight_type,
            validated_content,
            visibility,
            activity_id,
        );

        social.create_shared_insight(&insight).await?;

        let response: SharedInsightResponse = insight.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle POST /api/social/insights/generate - Generate shareable insight from analysis
    ///
    /// Transforms analysis content into a concise, inspiring social post format
    /// using the insight generation prompt. Returns only the shareable content
    /// without any preamble or explanatory text.
    async fn handle_generate_insight(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(body): Json<GenerateInsightBody>,
    ) -> Result<Response, AppError> {
        // Authenticate user
        Self::authenticate(&headers, &resources).await?;

        // Validate input
        if body.content.trim().is_empty() {
            return Err(AppError::invalid_input("Content cannot be empty"));
        }

        // Get LLM provider
        let llm_provider = ChatProvider::from_env().await?;

        // Build the generation request using the insight generation prompt
        let system_prompt = get_insight_generation_prompt();
        let user_message = body.content.clone();

        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_message),
        ];

        // Use low temperature for consistent output format
        let request = ChatRequest::new(messages).with_temperature(0.4);

        // Call LLM to generate the shareable insight
        let llm_response = llm_provider.complete(&request).await?;

        let response = GenerateInsightResponse {
            content: llm_response.content.trim().to_owned(),
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Create a configured provider with OAuth credentials
    ///
    /// Uses the same pattern as protocol handlers for creating providers.
    async fn create_configured_provider(
        provider_name: &str,
        provider_registry: &Arc<ProviderRegistry>,
        token_data: &TokenData,
    ) -> Result<Box<dyn FitnessProvider>, AppError> {
        // Create provider instance
        let provider = provider_registry
            .create_provider(provider_name)
            .map_err(|e| {
                AppError::internal(format!("Failed to create {provider_name} provider: {e}"))
            })?;

        // Load provider-specific OAuth config
        let config = get_oauth_config(provider_name);

        // Build credentials
        let credentials = OAuth2Credentials {
            client_id: config.client_id.clone().unwrap_or_default(),
            client_secret: config.client_secret.clone().unwrap_or_default(),
            access_token: Some(token_data.access_token.clone()),
            refresh_token: Some(token_data.refresh_token.clone()),
            expires_at: Some(token_data.expires_at),
            scopes: config.scopes.clone(),
        };

        // Set credentials on provider
        provider.set_credentials(credentials).await.map_err(|e| {
            AppError::internal(format!(
                "Failed to set {provider_name} provider credentials: {e}"
            ))
        })?;

        Ok(provider)
    }

    /// Fetch activities from the user's connected provider
    ///
    /// Uses `AuthService` to get a valid OAuth token and creates a configured provider
    /// to fetch recent activities.
    async fn fetch_activities_from_provider(
        resources: &Arc<ServerResources>,
        user_id: Uuid,
        provider_name: &str,
        tenant_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Activity>, AppError> {
        let auth_service = AuthService::new(resources.clone());

        // Get valid OAuth token
        let token_data = auth_service
            .get_valid_token(user_id, provider_name, tenant_id)
            .await
            .map_err(|e| AppError::internal(format!("OAuth error: {e}")))?
            .ok_or_else(|| {
                AppError::auth_invalid(format!(
                    "No valid token for provider '{provider_name}'. Please connect your account."
                ))
            })?;

        // Create configured provider with credentials
        let provider = Self::create_configured_provider(
            provider_name,
            &resources.provider_registry,
            &token_data,
        )
        .await?;

        // Fetch activities
        provider
            .get_activities(limit, None)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch activities: {e}")))
    }

    /// Build insight generation context from user's recent activities
    ///
    /// # Arguments
    /// * `activity_limit` - Optional client-requested limit (capped by server config)
    async fn build_insight_context(
        resources: &Arc<ServerResources>,
        user_id: Uuid,
        provider_name: &str,
        tenant_id: Option<&str>,
        activity_limit: Option<usize>,
    ) -> Result<InsightGenerationContext, AppError> {
        let config = SocialInsightsConfig::global();

        // Use client limit if provided, but cap at server's max client limit
        let effective_limit = activity_limit.map_or(
            config.activity_fetch_limits.insight_context_limit,
            |client_limit| client_limit.min(config.activity_fetch_limits.max_client_limit),
        );

        let activities = Self::fetch_activities_from_provider(
            resources,
            user_id,
            provider_name,
            tenant_id,
            Some(effective_limit),
        )
        .await?;

        // Build context using InsightContextBuilder
        let context = InsightContextBuilder::new()
            .with_activities(activities)
            .build();

        Ok(context)
    }

    /// Build user training context for insight adaptation
    async fn build_user_training_context(
        resources: &Arc<ServerResources>,
        user_id: Uuid,
        provider_name: &str,
        tenant_id: Option<&str>,
    ) -> Result<UserTrainingContext, AppError> {
        let config = SocialInsightsConfig::global();
        let activities = Self::fetch_activities_from_provider(
            resources,
            user_id,
            provider_name,
            tenant_id,
            Some(config.activity_fetch_limits.training_context_limit),
        )
        .await?;

        // Calculate fitness metrics from activities
        let recent_activity_count = u32::try_from(activities.len()).unwrap_or(u32::MAX);
        let days_since_last = activities.first().map_or(365, |a| {
            let diff = Utc::now() - a.start_date();
            u32::try_from(diff.num_days().max(0)).unwrap_or(u32::MAX)
        });

        // Calculate weekly volume (approximate) using duration_seconds
        // Safe: duration_seconds is bounded by u64, and we need f64 for hours calculation
        #[allow(clippy::cast_precision_loss)]
        let weekly_volume_hours = activities
            .iter()
            .filter(|a| {
                let age = Utc::now() - a.start_date();
                age.num_days() <= 7
            })
            .map(|a| a.duration_seconds() as f64)
            .sum::<f64>()
            / 3600.0;

        // Determine primary sport
        let primary_sport = activities
            .first()
            .map(|a| a.sport_type().display_name().to_owned());

        Ok(UserTrainingContext {
            fitness_score: None,
            training_phase: None,
            weekly_volume_hours: Some(weekly_volume_hours),
            primary_sport,
            training_goal: None,
            recent_activity_count,
            days_since_last_workout: days_since_last,
        })
    }

    // ========================================================================
    // Reactions
    // ========================================================================

    /// Handle GET /api/social/insights/:id/reactions - List reactions
    async fn handle_list_reactions(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
    ) -> Result<Response, AppError> {
        // Authenticate user (must be logged in to view reactions)
        Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_id =
            Uuid::parse_str(&id).map_err(|_| AppError::invalid_input("Invalid insight ID"))?;

        // Verify insight exists
        social
            .get_shared_insight(insight_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Insight {id}")))?;

        let reactions = social.get_insight_reactions(insight_id).await?;

        // Build summary
        let mut summary = ReactionSummaryResponse::default();
        for reaction in &reactions {
            match reaction.reaction_type {
                ReactionType::Like => summary.like_count += 1,
                ReactionType::Celebrate => summary.celebrate_count += 1,
                ReactionType::Inspire => summary.inspire_count += 1,
                ReactionType::Support => summary.support_count += 1,
            }
            summary.total += 1;
        }

        let response = ListReactionsResponse {
            reactions: reactions.into_iter().map(Into::into).collect(),
            summary,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle POST /api/social/insights/:id/reactions - Add reaction
    async fn handle_add_reaction(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<ReactToInsightBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_id =
            Uuid::parse_str(&id).map_err(|_| AppError::invalid_input("Invalid insight ID"))?;

        let reaction_type = ReactionType::from_str(&body.reaction_type)?;

        // Verify insight exists
        social
            .get_shared_insight(insight_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Insight {id}")))?;

        // Check if user already has a reaction
        let existing = social.get_user_reaction(insight_id, auth.user_id).await?;
        if let Some(existing_reaction) = existing {
            if existing_reaction.reaction_type == reaction_type {
                // Same reaction type - should use remove endpoint to toggle
                return Err(AppError::invalid_input(
                    "You have already reacted with this type. Use remove to toggle.",
                ));
            }
            // Different reaction type - update to new type (delete old, create new)
            social
                .delete_insight_reaction(insight_id, auth.user_id)
                .await?;
        }

        let reaction = InsightReaction::new(insight_id, auth.user_id, reaction_type);
        social.create_insight_reaction(&reaction).await?;

        let response: ReactionResponse = reaction.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle DELETE /api/social/insights/:id/reactions/:type - Remove reaction
    async fn handle_remove_reaction(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path((id, reaction_type_str)): Path<(String, String)>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_id =
            Uuid::parse_str(&id).map_err(|_| AppError::invalid_input("Invalid insight ID"))?;

        // Validate the reaction type from URL (even though delete removes any reaction by user)
        ReactionType::from_str(&reaction_type_str)?;

        social
            .delete_insight_reaction(insight_id, auth.user_id)
            .await?;

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    // ========================================================================
    // Feed
    // ========================================================================

    /// Handle GET /api/social/feed - Get social feed
    async fn handle_get_feed(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<FeedQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        // Get full feed items with author, reactions, and user-specific state
        let feed_items = social
            .get_friend_insights_feed_full(auth.user_id, limit, offset)
            .await?;

        // Convert to response format
        let items: Vec<FeedItemResponse> = feed_items
            .into_iter()
            .map(|item| FeedItemResponse {
                insight: item.insight.into(),
                author: FeedAuthorResponse {
                    user_id: item.author.user_id.to_string(),
                    display_name: item.author.display_name,
                    email: item.author.email,
                },
                reactions: ReactionCountsResponse {
                    like: item.reactions.like_count,
                    celebrate: item.reactions.celebrate_count,
                    inspire: item.reactions.inspire_count,
                    support: item.reactions.support_count,
                    total: item.reactions.total,
                },
                user_reaction: item.user_reaction.map(|r| r.as_str().to_owned()),
                user_has_adapted: item.user_has_adapted,
            })
            .collect();

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        #[allow(clippy::cast_possible_truncation)] // limit is clamped to small values
        let limit_usize = limit as usize;
        let has_more = items.len() >= limit_usize;
        let next_cursor = if has_more {
            Some((offset + limit).to_string())
        } else {
            None
        };

        let response = FeedResponse {
            items,
            next_cursor,
            has_more,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    // ========================================================================
    // Adapted Insights
    // ========================================================================

    /// Handle POST /api/social/insights/:id/adapt - Adapt an insight
    async fn handle_adapt_insight(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<AdaptInsightBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let insight_id =
            Uuid::parse_str(&id).map_err(|_| AppError::invalid_input("Invalid insight ID"))?;

        // Verify insight exists
        let source_insight = social
            .get_shared_insight(insight_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Insight {id}")))?;

        // Check if user has already adapted this insight
        if let Some(existing_adaptation) = social
            .get_adapted_insight_by_source(insight_id, auth.user_id)
            .await?
        {
            // Return existing adaptation instead of creating duplicate
            let response = AdaptInsightResultResponse {
                adapted: existing_adaptation.into(),
                source_insight: source_insight.into(),
                metadata: Self::build_metadata(),
            };
            return Ok((StatusCode::OK, Json(response)).into_response());
        }

        // Use provider from body or fall back to environment default
        let provider_name = body.provider.clone().unwrap_or_else(default_provider);

        // Build user training context from their activities
        // If we can't fetch activities (no OAuth token), use a default context
        let user_context = Self::build_user_training_context(
            &resources,
            auth.user_id,
            &provider_name,
            body.tenant_id.as_deref(),
        )
        .await
        .unwrap_or_else(|_| UserTrainingContext::default());

        // Use the InsightAdapter to generate personalized content
        let insight_adapter = InsightAdapter::new();
        let adaptation_result =
            insight_adapter.adapt(&source_insight, &user_context, body.context.as_deref());

        // Create the adapted insight using the real adaptation
        let adapted_insight =
            InsightAdapter::create_adapted_insight(insight_id, auth.user_id, &adaptation_result);

        social.create_adapted_insight(&adapted_insight).await?;

        let response = AdaptInsightResultResponse {
            adapted: adapted_insight.into(),
            source_insight: source_insight.into(),
            metadata: Self::build_metadata(),
        };
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/social/adapted - List user's adapted insights
    async fn handle_list_adapted(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<ListAdaptedQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let adapted = social
            .get_user_adapted_insights_paginated(auth.user_id, limit, offset)
            .await?;

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        #[allow(clippy::cast_possible_truncation)] // limit is clamped to small values
        let limit_usize = limit as usize;
        let has_more = adapted.len() >= limit_usize;
        let next_cursor = if has_more {
            Some((offset + limit).to_string())
        } else {
            None
        };

        let response = ListAdaptedInsightsResponse {
            total: adapted.len(),
            adapted_insights: adapted.into_iter().map(Into::into).collect(),
            next_cursor,
            has_more,
            metadata: Self::build_metadata(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle PUT /api/social/adapted/:id/helpful - Update helpful status
    async fn handle_update_helpful(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(id): Path<String>,
        Json(body): Json<UpdateHelpfulBody>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let adapted_id = Uuid::parse_str(&id)
            .map_err(|_| AppError::invalid_input("Invalid adapted insight ID"))?;

        let updated = social
            .update_adapted_insight_helpful(adapted_id, auth.user_id, body.was_helpful)
            .await?;

        if !updated {
            return Err(AppError::not_found(format!("Adapted insight {id}")));
        }

        Ok((StatusCode::NO_CONTENT, ()).into_response())
    }

    // ========================================================================
    // Discovery
    // ========================================================================

    /// Handle GET /api/social/users/search - Search for users
    async fn handle_search_users(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Query(query): Query<SearchUsersQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let users = social
            .search_discoverable_users(&query.q, query.limit.unwrap_or(20))
            .await?;

        // Filter out self and build response with friend status
        let mut results = Vec::new();
        for (user_id, email, display_name) in users {
            if user_id == auth.user_id {
                continue;
            }

            let connection = social
                .get_friend_connection_between(auth.user_id, user_id)
                .await?;

            let is_friend = connection.as_ref().is_some_and(|c| c.status.is_connected());
            let has_pending_request = connection
                .as_ref()
                .is_some_and(|c| c.status == FriendStatus::Pending);

            results.push(UserProfileResponse {
                id: user_id.to_string(),
                display_name,
                email,
                is_friend,
                has_pending_request,
            });
        }

        let response = SearchUsersResponse {
            total: results.len(),
            users: results,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
