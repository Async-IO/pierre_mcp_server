// ABOUTME: Social features database operations for coach-mediated sharing
// ABOUTME: Friend connections, shared insights, reactions, and adapted insights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::intelligence::InsightSharingPolicy;
use crate::models::{
    AdaptedInsight, FeedItem, FriendConnection, FriendInfo, FriendStatus, InsightReaction,
    InsightType, NotificationPreferences, ReactionSummary, ReactionType, SharedInsight,
    TrainingPhase, UserSocialSettings,
};

/// Social features database operations manager
///
/// Wraps a `SqlitePool` to provide social feature database operations.
/// Similar to `CoachesManager`, this struct can be used with `sqlite_pool()`
/// from the `DatabaseProvider` trait.
pub struct SocialManager {
    pool: SqlitePool,
}

impl SocialManager {
    /// Create a new social manager
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Friend Connections
    // ========================================================================

    /// Create a new friend connection request
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn create_friend_connection(&self, connection: &FriendConnection) -> AppResult<Uuid> {
        sqlx::query(
            r"
            INSERT INTO friend_connections (id, initiator_id, receiver_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ",
        )
        .bind(connection.id.to_string())
        .bind(connection.initiator_id.to_string())
        .bind(connection.receiver_id.to_string())
        .bind(connection.status.as_str())
        .bind(connection.created_at.to_rfc3339())
        .bind(connection.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create friend connection: {e}")))?;

        Ok(connection.id)
    }

    /// Get a friend connection by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_friend_connection(&self, id: Uuid) -> AppResult<Option<FriendConnection>> {
        let row = sqlx::query(
            r"
            SELECT id, initiator_id, receiver_id, status, created_at, updated_at, accepted_at
            FROM friend_connections
            WHERE id = $1
            ",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get friend connection: {e}")))?;

        row.map(|r| Self::row_to_friend_connection(&r)).transpose()
    }

    /// Get friend connection between two users (in either direction)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_friend_connection_between(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> AppResult<Option<FriendConnection>> {
        let row = sqlx::query(
            r"
            SELECT id, initiator_id, receiver_id, status, created_at, updated_at, accepted_at
            FROM friend_connections
            WHERE (initiator_id = $1 AND receiver_id = $2)
               OR (initiator_id = $2 AND receiver_id = $1)
            ",
        )
        .bind(user_a.to_string())
        .bind(user_b.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get friend connection: {e}")))?;

        row.map(|r| Self::row_to_friend_connection(&r)).transpose()
    }

    /// Update friend connection status
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn update_friend_connection_status(
        &self,
        id: Uuid,
        status: FriendStatus,
    ) -> AppResult<()> {
        let now = Utc::now();
        let accepted_at = if status == FriendStatus::Accepted {
            Some(now.to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r"
            UPDATE friend_connections
            SET status = $1, updated_at = $2, accepted_at = $3
            WHERE id = $4
            ",
        )
        .bind(status.as_str())
        .bind(now.to_rfc3339())
        .bind(accepted_at)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update friend connection: {e}")))?;

        Ok(())
    }

    /// Get all friends for a user (accepted connections only)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_friends(&self, user_id: Uuid) -> AppResult<Vec<FriendConnection>> {
        let rows = sqlx::query(
            r"
            SELECT id, initiator_id, receiver_id, status, created_at, updated_at, accepted_at
            FROM friend_connections
            WHERE (initiator_id = $1 OR receiver_id = $1)
              AND status = 'accepted'
            ORDER BY accepted_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get friends: {e}")))?;

        rows.iter().map(Self::row_to_friend_connection).collect()
    }

    /// Get all accepted friends for a user with pagination
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_friends_paginated(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<FriendConnection>> {
        let rows = sqlx::query(
            r"
            SELECT id, initiator_id, receiver_id, status, created_at, updated_at, accepted_at
            FROM friend_connections
            WHERE (initiator_id = $1 OR receiver_id = $1)
              AND status = 'accepted'
            ORDER BY accepted_at DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(user_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get friends: {e}")))?;

        rows.iter().map(Self::row_to_friend_connection).collect()
    }

    /// Get pending friend requests for a user (both sent and received)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_pending_friend_requests(
        &self,
        user_id: Uuid,
    ) -> AppResult<Vec<FriendConnection>> {
        let rows = sqlx::query(
            r"
            SELECT id, initiator_id, receiver_id, status, created_at, updated_at, accepted_at
            FROM friend_connections
            WHERE (initiator_id = $1 OR receiver_id = $1) AND status = 'pending'
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get pending requests: {e}")))?;

        rows.iter().map(Self::row_to_friend_connection).collect()
    }

    /// Delete a friend connection
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn delete_friend_connection(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM friend_connections WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete friend connection: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    fn row_to_friend_connection(row: &SqliteRow) -> AppResult<FriendConnection> {
        let id_str: String = row.get("id");
        let initiator_id_str: String = row.get("initiator_id");
        let receiver_id_str: String = row.get("receiver_id");
        let status_str: String = row.get("status");
        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");
        let accepted_at_str: Option<String> = row.get("accepted_at");

        Ok(FriendConnection {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            initiator_id: Uuid::parse_str(&initiator_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            receiver_id: Uuid::parse_str(&receiver_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            status: status_str
                .parse()
                .map_err(|e: AppError| AppError::database(e.to_string()))?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
            accepted_at: accepted_at_str
                .map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| AppError::database(format!("Invalid date: {e}")))
                })
                .transpose()?,
        })
    }

    // ========================================================================
    // User Social Settings
    // ========================================================================

    /// Get social settings for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_social_settings(
        &self,
        user_id: Uuid,
    ) -> AppResult<Option<UserSocialSettings>> {
        let row = sqlx::query(
            r"
            SELECT user_id, discoverable, default_visibility, share_activity_types,
                   notify_friend_requests, notify_insight_reactions, notify_adapted_insights,
                   insight_sharing_policy, created_at, updated_at
            FROM user_social_settings
            WHERE user_id = $1
            ",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get social settings: {e}")))?;

        row.map(|r| Self::row_to_social_settings(&r)).transpose()
    }

    /// Create or update social settings
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn upsert_user_social_settings(
        &self,
        settings: &UserSocialSettings,
    ) -> AppResult<()> {
        let activity_types_json =
            serde_json::to_string(&settings.share_activity_types).unwrap_or_else(|_| "[]".into());

        sqlx::query(
            r"
            INSERT INTO user_social_settings (
                user_id, discoverable, default_visibility, share_activity_types,
                notify_friend_requests, notify_insight_reactions, notify_adapted_insights,
                insight_sharing_policy, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT(user_id) DO UPDATE SET
                discoverable = excluded.discoverable,
                default_visibility = excluded.default_visibility,
                share_activity_types = excluded.share_activity_types,
                notify_friend_requests = excluded.notify_friend_requests,
                notify_insight_reactions = excluded.notify_insight_reactions,
                notify_adapted_insights = excluded.notify_adapted_insights,
                insight_sharing_policy = excluded.insight_sharing_policy,
                updated_at = excluded.updated_at
            ",
        )
        .bind(settings.user_id.to_string())
        .bind(i32::from(settings.discoverable))
        .bind(settings.default_visibility.as_str())
        .bind(activity_types_json)
        .bind(i32::from(settings.notifications.friend_requests))
        .bind(i32::from(settings.notifications.insight_reactions))
        .bind(i32::from(settings.notifications.adapted_insights))
        .bind(settings.insight_sharing_policy.as_str())
        .bind(settings.created_at.to_rfc3339())
        .bind(settings.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to upsert social settings: {e}")))?;

        Ok(())
    }

    fn row_to_social_settings(row: &SqliteRow) -> AppResult<UserSocialSettings> {
        let user_id_str: String = row.get("user_id");
        let discoverable: i32 = row.get("discoverable");
        let default_visibility_str: String = row.get("default_visibility");
        let activity_types_json: String = row.get("share_activity_types");
        let notify_friend_requests: i32 = row.get("notify_friend_requests");
        let notify_insight_reactions: i32 = row.get("notify_insight_reactions");
        let notify_adapted_insights: i32 = row.get("notify_adapted_insights");
        let insight_sharing_policy_str: String = row.get("insight_sharing_policy");
        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");

        let share_activity_types: Vec<String> =
            serde_json::from_str(&activity_types_json).unwrap_or_default();

        let insight_sharing_policy =
            InsightSharingPolicy::parse(&insight_sharing_policy_str).unwrap_or_default();

        Ok(UserSocialSettings {
            user_id: Uuid::parse_str(&user_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            discoverable: discoverable != 0,
            default_visibility: default_visibility_str
                .parse()
                .map_err(|e: AppError| AppError::database(e.to_string()))?,
            share_activity_types,
            notifications: NotificationPreferences {
                friend_requests: notify_friend_requests != 0,
                insight_reactions: notify_insight_reactions != 0,
                adapted_insights: notify_adapted_insights != 0,
            },
            insight_sharing_policy,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
        })
    }

    // ========================================================================
    // Shared Insights
    // ========================================================================

    /// Create a shared insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn create_shared_insight(&self, insight: &SharedInsight) -> AppResult<Uuid> {
        let training_phase_str = insight.training_phase.as_ref().map(TrainingPhase::as_str);
        let expires_at_str = insight.expires_at.map(|dt| dt.to_rfc3339());

        sqlx::query(
            r"
            INSERT INTO shared_insights (
                id, user_id, visibility, insight_type, sport_type, content, title,
                training_phase, reaction_count, adapt_count, created_at, updated_at, expires_at,
                source_activity_id, coach_generated
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ",
        )
        .bind(insight.id.to_string())
        .bind(insight.user_id.to_string())
        .bind(insight.visibility.as_str())
        .bind(insight.insight_type.as_str())
        .bind(&insight.sport_type)
        .bind(&insight.content)
        .bind(&insight.title)
        .bind(training_phase_str)
        .bind(insight.reaction_count)
        .bind(insight.adapt_count)
        .bind(insight.created_at.to_rfc3339())
        .bind(insight.updated_at.to_rfc3339())
        .bind(expires_at_str)
        .bind(&insight.source_activity_id)
        .bind(i32::from(insight.coach_generated))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create shared insight: {e}")))?;

        Ok(insight.id)
    }

    /// Get a shared insight by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_shared_insight(&self, id: Uuid) -> AppResult<Option<SharedInsight>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, visibility, insight_type, sport_type, content, title,
                   training_phase, reaction_count, adapt_count, created_at, updated_at, expires_at,
                   source_activity_id, coach_generated
            FROM shared_insights
            WHERE id = $1
            ",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get shared insight: {e}")))?;

        row.map(|r| Self::row_to_shared_insight(&r)).transpose()
    }

    /// Check if a user has already shared an insight from a specific activity
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn has_insight_for_activity(
        &self,
        user_id: Uuid,
        activity_id: &str,
    ) -> AppResult<bool> {
        let row = sqlx::query(
            r"
            SELECT 1
            FROM shared_insights
            WHERE user_id = $1 AND source_activity_id = $2
            LIMIT 1
            ",
        )
        .bind(user_id.to_string())
        .bind(activity_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to check for existing insight: {e}")))?;

        Ok(row.is_some())
    }

    /// Get user's shared insights with optional filtering
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_shared_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<InsightType>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<SharedInsight>> {
        let rows = if let Some(itype) = insight_type {
            sqlx::query(
                r"
                SELECT id, user_id, visibility, insight_type, sport_type, content, title,
                       training_phase, reaction_count, adapt_count, created_at, updated_at, expires_at,
                       source_activity_id, coach_generated
                FROM shared_insights
                WHERE user_id = $1 AND insight_type = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                ",
            )
            .bind(user_id.to_string())
            .bind(itype.as_str())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r"
                SELECT id, user_id, visibility, insight_type, sport_type, content, title,
                       training_phase, reaction_count, adapt_count, created_at, updated_at, expires_at,
                       source_activity_id, coach_generated
                FROM shared_insights
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                ",
            )
            .bind(user_id.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to get user insights: {e}")))?;

        rows.iter().map(Self::row_to_shared_insight).collect()
    }

    /// Get basic user profile for feed author display
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or user not found
    pub async fn get_user_profile(&self, user_id: Uuid) -> AppResult<FriendInfo> {
        let row: Option<SqliteRow> =
            sqlx::query("SELECT id, display_name, email, created_at FROM users WHERE id = $1")
                .bind(user_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Failed to get user profile: {e}")))?;

        let row = row.ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

        let id_str: String = row.get("id");
        let display_name: Option<String> = row.get("display_name");
        let email: String = row.get("email");
        let created_at_str: String = row.get("created_at");

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| AppError::database(format!("Invalid datetime: {e}")))?
            .with_timezone(&Utc);

        Ok(FriendInfo {
            user_id: Uuid::parse_str(&id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            display_name,
            email,
            friends_since: created_at, // Use created_at as placeholder for author info
        })
    }

    /// Get friend's insights feed
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_friend_insights_feed(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<SharedInsight>> {
        let rows = sqlx::query(
            r"
            SELECT si.id, si.user_id, si.visibility, si.insight_type, si.sport_type,
                   si.content, si.title, si.training_phase, si.reaction_count, si.adapt_count,
                   si.created_at, si.updated_at, si.expires_at, si.source_activity_id, si.coach_generated
            FROM shared_insights si
            WHERE (
                si.user_id = $1
                OR si.user_id IN (
                    SELECT CASE
                        WHEN fc.initiator_id = $1 THEN fc.receiver_id
                        ELSE fc.initiator_id
                    END
                    FROM friend_connections fc
                    WHERE (fc.initiator_id = $1 OR fc.receiver_id = $1)
                      AND fc.status = 'accepted'
                )
            )
            AND (si.expires_at IS NULL OR si.expires_at > datetime('now'))
            ORDER BY si.created_at DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(user_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get friends feed: {e}")))?;

        rows.iter().map(Self::row_to_shared_insight).collect()
    }

    /// Get friend's insights feed with full item data
    ///
    /// Returns feed items with author info, reaction counts, and user-specific state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_friend_insights_feed_full(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<FeedItem>> {
        // Get insights from friends
        let insights = self
            .get_friend_insights_feed(user_id, limit, offset)
            .await?;

        let mut feed_items = Vec::with_capacity(insights.len());

        for insight in insights {
            // Get author info
            let author = self.get_user_profile(insight.user_id).await?;

            // Get reaction counts for this insight
            let reactions = self.get_insight_reactions(insight.id).await?;
            // Reaction counts are small numbers (per-insight counts), safe to cast to i32
            #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
            let reaction_summary = ReactionSummary {
                like_count: reactions
                    .iter()
                    .filter(|r| r.reaction_type == ReactionType::Like)
                    .count() as i32,
                celebrate_count: reactions
                    .iter()
                    .filter(|r| r.reaction_type == ReactionType::Celebrate)
                    .count() as i32,
                inspire_count: reactions
                    .iter()
                    .filter(|r| r.reaction_type == ReactionType::Inspire)
                    .count() as i32,
                support_count: reactions
                    .iter()
                    .filter(|r| r.reaction_type == ReactionType::Support)
                    .count() as i32,
                total: reactions.len() as i32,
            };

            // Get current user's reaction
            let user_reaction = self.get_user_reaction(insight.id, user_id).await?;

            // Check if user has adapted this insight
            let user_has_adapted = self
                .get_adapted_insight_by_source(insight.id, user_id)
                .await?
                .is_some();

            feed_items.push(FeedItem {
                insight,
                author,
                reactions: reaction_summary,
                user_reaction: user_reaction.map(|r| r.reaction_type),
                user_has_adapted,
            });
        }

        Ok(feed_items)
    }

    /// Delete a shared insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn delete_shared_insight(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM shared_insights WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to delete shared insight: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    fn row_to_shared_insight(row: &SqliteRow) -> AppResult<SharedInsight> {
        let id_str: String = row.get("id");
        let user_id_str: String = row.get("user_id");
        let visibility_str: String = row.get("visibility");
        let insight_type_str: String = row.get("insight_type");
        let sport_type: Option<String> = row.get("sport_type");
        let content: String = row.get("content");
        let title: Option<String> = row.get("title");
        let training_phase_str: Option<String> = row.get("training_phase");
        let reaction_count: i32 = row.get("reaction_count");
        let adapt_count: i32 = row.get("adapt_count");
        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");
        let expires_at_str: Option<String> = row.get("expires_at");
        let source_activity_id: Option<String> = row.get("source_activity_id");
        let coach_generated: i32 = row.get("coach_generated");

        Ok(SharedInsight {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            user_id: Uuid::parse_str(&user_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            visibility: visibility_str
                .parse()
                .map_err(|e: AppError| AppError::database(e.to_string()))?,
            insight_type: insight_type_str
                .parse()
                .map_err(|e: AppError| AppError::database(e.to_string()))?,
            sport_type,
            content,
            title,
            training_phase: training_phase_str
                .map(|s| s.parse())
                .transpose()
                .map_err(|e: AppError| AppError::database(e.to_string()))?,
            reaction_count,
            adapt_count,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
            expires_at: expires_at_str
                .map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| AppError::database(format!("Invalid date: {e}")))
                })
                .transpose()?,
            source_activity_id,
            coach_generated: coach_generated != 0,
        })
    }

    // ========================================================================
    // Insight Reactions
    // ========================================================================

    /// Create a reaction to an insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn create_insight_reaction(&self, reaction: &InsightReaction) -> AppResult<()> {
        sqlx::query(
            r"
            INSERT INTO insight_reactions (id, insight_id, user_id, reaction_type, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ",
        )
        .bind(reaction.id.to_string())
        .bind(reaction.insight_id.to_string())
        .bind(reaction.user_id.to_string())
        .bind(reaction.reaction_type.as_str())
        .bind(reaction.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create reaction: {e}")))?;

        Ok(())
    }

    /// Get a user's reaction to an insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_reaction(
        &self,
        insight_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<InsightReaction>> {
        let row = sqlx::query(
            r"
            SELECT id, insight_id, user_id, reaction_type, created_at
            FROM insight_reactions
            WHERE insight_id = $1 AND user_id = $2
            ",
        )
        .bind(insight_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get reaction: {e}")))?;

        row.map(|r| Self::row_to_insight_reaction(&r)).transpose()
    }

    /// Delete a reaction
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn delete_insight_reaction(
        &self,
        insight_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<bool> {
        let result =
            sqlx::query("DELETE FROM insight_reactions WHERE insight_id = $1 AND user_id = $2")
                .bind(insight_id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Failed to delete reaction: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get all reactions for an insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_insight_reactions(&self, insight_id: Uuid) -> AppResult<Vec<InsightReaction>> {
        let rows = sqlx::query(
            r"
            SELECT id, insight_id, user_id, reaction_type, created_at
            FROM insight_reactions
            WHERE insight_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(insight_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get reactions: {e}")))?;

        rows.iter().map(Self::row_to_insight_reaction).collect()
    }

    fn row_to_insight_reaction(row: &SqliteRow) -> AppResult<InsightReaction> {
        let id_str: String = row.get("id");
        let insight_id_str: String = row.get("insight_id");
        let user_id_str: String = row.get("user_id");
        let reaction_type_str: String = row.get("reaction_type");
        let created_at_str: String = row.get("created_at");

        Ok(InsightReaction {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            insight_id: Uuid::parse_str(&insight_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            user_id: Uuid::parse_str(&user_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            reaction_type: reaction_type_str
                .parse()
                .map_err(|e: AppError| AppError::database(e.to_string()))?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
        })
    }

    // ========================================================================
    // Adapted Insights
    // ========================================================================

    /// Create an adapted insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn create_adapted_insight(&self, insight: &AdaptedInsight) -> AppResult<Uuid> {
        sqlx::query(
            r"
            INSERT INTO adapted_insights (
                id, source_insight_id, user_id, adapted_content, adaptation_context,
                was_helpful, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ",
        )
        .bind(insight.id.to_string())
        .bind(insight.source_insight_id.to_string())
        .bind(insight.user_id.to_string())
        .bind(&insight.adapted_content)
        .bind(&insight.adaptation_context)
        .bind(insight.was_helpful.map(i32::from))
        .bind(insight.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create adapted insight: {e}")))?;

        Ok(insight.id)
    }

    /// Get an adapted insight by source insight ID and user ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_adapted_insight_by_source(
        &self,
        source_insight_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<AdaptedInsight>> {
        let row = sqlx::query(
            r"
            SELECT id, source_insight_id, user_id, adapted_content, adaptation_context,
                   was_helpful, created_at
            FROM adapted_insights
            WHERE source_insight_id = $1 AND user_id = $2
            ",
        )
        .bind(source_insight_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get adapted insight: {e}")))?;

        row.as_ref().map(Self::row_to_adapted_insight).transpose()
    }

    /// Get user's adapted insights
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_adapted_insights(&self, user_id: Uuid) -> AppResult<Vec<AdaptedInsight>> {
        let rows = sqlx::query(
            r"
            SELECT id, source_insight_id, user_id, adapted_content, adaptation_context,
                   was_helpful, created_at
            FROM adapted_insights
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get adapted insights: {e}")))?;

        rows.iter().map(Self::row_to_adapted_insight).collect()
    }

    /// Get adapted insights for a user with pagination
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_user_adapted_insights_paginated(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<AdaptedInsight>> {
        let rows = sqlx::query(
            r"
            SELECT id, source_insight_id, user_id, adapted_content, adaptation_context,
                   was_helpful, created_at
            FROM adapted_insights
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(user_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get adapted insights: {e}")))?;

        rows.iter().map(Self::row_to_adapted_insight).collect()
    }

    /// Update the `was_helpful` field for an adapted insight
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn update_adapted_insight_helpful(
        &self,
        id: Uuid,
        user_id: Uuid,
        was_helpful: bool,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE adapted_insights SET was_helpful = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(i32::from(was_helpful))
        .bind(id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update adapted insight: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    fn row_to_adapted_insight(row: &SqliteRow) -> AppResult<AdaptedInsight> {
        let id_str: String = row.get("id");
        let source_insight_id_str: String = row.get("source_insight_id");
        let user_id_str: String = row.get("user_id");
        let adapted_content: String = row.get("adapted_content");
        let adaptation_context: Option<String> = row.get("adaptation_context");
        let was_helpful: Option<i32> = row.get("was_helpful");
        let created_at_str: String = row.get("created_at");

        Ok(AdaptedInsight {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            source_insight_id: Uuid::parse_str(&source_insight_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            user_id: Uuid::parse_str(&user_id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?,
            adapted_content,
            adaptation_context,
            was_helpful: was_helpful.map(|v| v != 0),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| AppError::database(format!("Invalid date: {e}")))?
                .with_timezone(&Utc),
        })
    }

    // ========================================================================
    // Search and Discovery
    // ========================================================================

    /// Search for users by email or display name (for friend discovery)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn search_discoverable_users(
        &self,
        query: &str,
        limit: i64,
    ) -> AppResult<Vec<(Uuid, String, Option<String>)>> {
        let search_pattern = format!("%{query}%");

        let rows = sqlx::query(
            r"
            SELECT u.id, u.email, u.display_name
            FROM users u
            LEFT JOIN user_social_settings uss ON u.id = uss.user_id
            WHERE (uss.discoverable = 1 OR uss.discoverable IS NULL)
              AND u.user_status = 'active'
              AND (u.email LIKE $1 OR u.display_name LIKE $1)
            ORDER BY u.display_name, u.email
            LIMIT $2
            ",
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to search users: {e}")))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.get("id");
            let email: String = row.get("email");
            let display_name: Option<String> = row.get("display_name");

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| AppError::database(format!("Invalid UUID: {e}")))?;

            results.push((id, email, display_name));
        }

        Ok(results)
    }
}
