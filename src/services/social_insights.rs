// ABOUTME: Social insights domain service for multi-step social feature orchestration
// ABOUTME: Extracts friend-request validation, feed aggregation, and insight adaptation from routes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database::social::SocialManager;
use crate::errors::{AppError, AppResult};
use crate::intelligence::insight_adapter::{InsightAdapter, UserTrainingContext};
use crate::models::{AdaptedInsight, FriendConnection, FriendStatus, SharedInsight};
use uuid::Uuid;

/// Result of sending a friend request, including validation
pub struct FriendRequestResult {
    /// The created friend connection
    pub connection: FriendConnection,
}

/// A search result enriched with friend-status information
pub struct EnrichedUserResult {
    /// User ID
    pub user_id: Uuid,
    /// Display name if set
    pub display_name: Option<String>,
    /// Email visible only to connected friends
    pub visible_email: Option<String>,
    /// Whether the searcher and this user are connected friends
    pub is_friend: bool,
    /// Whether there is a pending friend request
    pub has_pending_request: bool,
}

/// Result of adapting a shared insight for a user
pub struct AdaptInsightResult {
    /// The adapted insight (newly created or existing)
    pub adapted: AdaptedInsight,
    /// The source insight that was adapted
    pub source_insight: SharedInsight,
    /// Whether this was an existing adaptation (true) or newly created (false)
    pub already_existed: bool,
}

/// Validate and create a friend connection request.
///
/// Enforces business rules:
/// - Cannot send request to yourself
/// - Cannot create duplicate connections
/// - Creates connection in Pending status
///
/// # Errors
///
/// Returns `AppError::InvalidInput` if sender and receiver are the same user,
/// or if a connection already exists between the two users.
/// Returns database errors on connection creation failure.
pub async fn create_friend_request(
    social: &SocialManager,
    sender_id: Uuid,
    receiver_id: Uuid,
) -> AppResult<FriendRequestResult> {
    // Self-request validation
    if sender_id == receiver_id {
        return Err(AppError::invalid_input(
            "Cannot send friend request to yourself",
        ));
    }

    // Duplicate check
    let existing = social
        .get_friend_connection_between(sender_id, receiver_id)
        .await?;
    if existing.is_some() {
        return Err(AppError::invalid_input(
            "Friend connection already exists between these users",
        ));
    }

    // Create connection
    let connection = FriendConnection::new(sender_id, receiver_id);
    social.create_friend_connection(&connection).await?;

    Ok(FriendRequestResult { connection })
}

/// Search for discoverable users with friend-status enrichment.
///
/// Business rules:
/// - Excludes the searching user from results
/// - Enriches each result with friend connection status
/// - Email is only visible to connected friends (privacy protection)
///
/// # Errors
///
/// Returns database errors on search or connection lookup failure.
pub async fn search_users_with_status(
    social: &SocialManager,
    searcher_id: Uuid,
    query: &str,
    limit: u32,
) -> AppResult<Vec<EnrichedUserResult>> {
    let users = social
        .search_discoverable_users(query, i64::from(limit))
        .await?;

    let mut results = Vec::with_capacity(users.len());
    for (user_id, email, display_name) in users {
        // Filter out self
        if user_id == searcher_id {
            continue;
        }

        let connection = social
            .get_friend_connection_between(searcher_id, user_id)
            .await?;

        let is_friend = connection.as_ref().is_some_and(|c| c.status.is_connected());
        let has_pending_request = connection
            .as_ref()
            .is_some_and(|c| c.status == FriendStatus::Pending);

        // Only expose email to connected friends for privacy
        let visible_email = if is_friend { Some(email) } else { None };

        results.push(EnrichedUserResult {
            user_id,
            display_name,
            visible_email,
            is_friend,
            has_pending_request,
        });
    }

    Ok(results)
}

/// Adapt a shared insight for a user's training context.
///
/// Business rules:
/// - Verifies the source insight exists and is visible to the user
/// - Returns existing adaptation if user already adapted this insight (idempotent)
/// - Builds user training context from their activities
/// - Generates personalized adaptation using `InsightAdapter`
///
/// # Errors
///
/// Returns `AppError::NotFound` if the source insight does not exist.
/// Returns database errors on adaptation lookup or creation failure.
pub async fn adapt_insight_for_user(
    social: &SocialManager,
    user_id: Uuid,
    insight_id: Uuid,
    user_context: &UserTrainingContext,
    additional_context: Option<&str>,
) -> AppResult<AdaptInsightResult> {
    // Verify insight exists and is visible to user
    let source_insight = social
        .get_shared_insight(insight_id, user_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Insight {insight_id}")))?;

    // Check for existing adaptation (idempotent)
    if let Some(existing) = social
        .get_adapted_insight_by_source(insight_id, user_id)
        .await?
    {
        return Ok(AdaptInsightResult {
            adapted: existing,
            source_insight,
            already_existed: true,
        });
    }

    // Generate personalized adaptation
    let insight_adapter = InsightAdapter::new();
    let adaptation_result =
        insight_adapter.adapt(&source_insight, user_context, additional_context);

    // Create the adapted insight
    let adapted_insight =
        InsightAdapter::create_adapted_insight(insight_id, user_id, &adaptation_result);
    social.create_adapted_insight(&adapted_insight).await?;

    Ok(AdaptInsightResult {
        adapted: adapted_insight,
        source_insight,
        already_existed: false,
    })
}
