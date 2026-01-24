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
    database::social::SocialManager,
    errors::{AppError, ErrorCode},
    mcp::resources::ServerResources,
    models::{
        AdaptedInsight, FriendConnection, FriendStatus, InsightReaction, InsightType, ReactionType,
        ShareVisibility, SharedInsight, TrainingPhase, UserSocialSettings,
    },
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

/// Response for listing friends
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListFriendsResponse {
    /// List of friend connections
    pub friends: Vec<FriendConnectionResponse>,
    /// Total count
    pub total: usize,
    /// Metadata
    pub metadata: SocialMetadata,
}

/// Response for pending friend requests
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PendingRequestsResponse {
    /// Requests sent by the user
    pub sent: Vec<FriendConnectionResponse>,
    /// Requests received by the user
    pub received: Vec<FriendConnectionResponse>,
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
}

/// Request to update helpful status
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateHelpfulBody {
    /// Whether the adaptation was helpful
    pub was_helpful: bool,
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

    // ========================================================================
    // Friend Connections
    // ========================================================================

    /// Handle GET /api/social/friends - List friends
    async fn handle_list_friends(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let friends = social.get_friends(auth.user_id).await?;

        let response = ListFriendsResponse {
            total: friends.len(),
            friends: friends.into_iter().map(Into::into).collect(),
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

        let (sent, received): (Vec<_>, Vec<_>) = pending
            .into_iter()
            .partition(|conn| conn.initiator_id == auth.user_id);

        let response = PendingRequestsResponse {
            sent: sent.into_iter().map(Into::into).collect(),
            received: received.into_iter().map(Into::into).collect(),
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

        let insights = social
            .get_user_shared_insights(
                auth.user_id,
                insight_type,
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await?;

        let response = ListInsightsResponse {
            total: insights.len(),
            insights: insights.into_iter().map(Into::into).collect(),
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

        let mut insight = SharedInsight::new(auth.user_id, insight_type, body.content, visibility);
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

        // Check if user already reacted with this type
        let existing = social.get_user_reaction(insight_id, auth.user_id).await?;
        if existing.is_some() {
            return Err(AppError::invalid_input(
                "You have already reacted to this insight",
            ));
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

        let insights = social
            .get_friend_insights_feed(
                auth.user_id,
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await?;

        let response = ListInsightsResponse {
            total: insights.len(),
            insights: insights.into_iter().map(Into::into).collect(),
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

        // For now, create a simple adaptation (the actual AI adaptation would be done
        // by the intelligence layer in a later ticket)
        let adapted_content = format!("Adapted for your training: {}", source_insight.content);

        let mut adapted = AdaptedInsight::new(insight_id, auth.user_id, adapted_content);
        adapted.adaptation_context = body.context;

        social.create_adapted_insight(&adapted).await?;

        let response: AdaptedInsightResponse = adapted.into();
        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle GET /api/social/adapted - List user's adapted insights
    async fn handle_list_adapted(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let social = Self::get_social_manager(&resources)?;

        let adapted = social.get_user_adapted_insights(auth.user_id).await?;

        let response = ListAdaptedInsightsResponse {
            total: adapted.len(),
            adapted_insights: adapted.into_iter().map(Into::into).collect(),
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
