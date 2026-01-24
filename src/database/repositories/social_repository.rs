// ABOUTME: Social repository implementation for friend connections and shared insights
// ABOUTME: Implements SocialRepository trait by delegating to Database methods
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use async_trait::async_trait;
use uuid::Uuid;

use super::SocialRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::models::{
    AdaptedInsight, FriendConnection, FriendStatus, InsightReaction, SharedInsight,
    UserSocialSettings,
};

/// SQLite/PostgreSQL implementation of `SocialRepository`
pub struct SocialRepositoryImpl {
    db: Database,
}

impl SocialRepositoryImpl {
    /// Create a new `SocialRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SocialRepository for SocialRepositoryImpl {
    async fn create_friend_connection(
        &self,
        connection: &FriendConnection,
    ) -> Result<Uuid, DatabaseError> {
        self.db
            .create_friend_connection(connection)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_friend_connection(
        &self,
        id: Uuid,
    ) -> Result<Option<FriendConnection>, DatabaseError> {
        self.db
            .get_friend_connection(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_friend_connection_between(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Option<FriendConnection>, DatabaseError> {
        self.db
            .get_friend_connection_between(user_a, user_b)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_friend_connection_status(
        &self,
        id: Uuid,
        status: FriendStatus,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_friend_connection_status(id, status)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_friends(&self, user_id: Uuid) -> Result<Vec<FriendConnection>, DatabaseError> {
        self.db
            .get_friends(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_pending_friend_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FriendConnection>, DatabaseError> {
        self.db
            .get_pending_friend_requests(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_sent_friend_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FriendConnection>, DatabaseError> {
        self.db
            .get_sent_friend_requests(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn are_friends(&self, user_a: Uuid, user_b: Uuid) -> Result<bool, DatabaseError> {
        self.db
            .are_friends(user_a, user_b)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_friend_connection(&self, id: Uuid) -> Result<bool, DatabaseError> {
        self.db
            .delete_friend_connection(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_or_create_social_settings(
        &self,
        user_id: Uuid,
    ) -> Result<UserSocialSettings, DatabaseError> {
        self.db
            .get_or_create_social_settings(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_social_settings(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserSocialSettings>, DatabaseError> {
        self.db
            .get_social_settings(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn upsert_social_settings(
        &self,
        settings: &UserSocialSettings,
    ) -> Result<(), DatabaseError> {
        self.db
            .upsert_social_settings(settings)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn create_shared_insight(&self, insight: &SharedInsight) -> Result<Uuid, DatabaseError> {
        self.db
            .create_shared_insight(insight)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_shared_insight(&self, id: Uuid) -> Result<Option<SharedInsight>, DatabaseError> {
        self.db
            .get_shared_insight(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_friends_feed(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SharedInsight>, DatabaseError> {
        self.db
            .get_friends_feed(user_id, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_user_shared_insights(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SharedInsight>, DatabaseError> {
        self.db
            .get_user_shared_insights(user_id, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_shared_insight(&self, id: Uuid, user_id: Uuid) -> Result<bool, DatabaseError> {
        self.db
            .delete_shared_insight(id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn upsert_insight_reaction(
        &self,
        reaction: &InsightReaction,
    ) -> Result<(), DatabaseError> {
        self.db
            .upsert_insight_reaction(reaction)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_insight_reaction(
        &self,
        insight_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<InsightReaction>, DatabaseError> {
        self.db
            .get_insight_reaction(insight_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_insight_reaction(
        &self,
        insight_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, DatabaseError> {
        self.db
            .delete_insight_reaction(insight_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_insight_reactions(
        &self,
        insight_id: Uuid,
    ) -> Result<Vec<InsightReaction>, DatabaseError> {
        self.db
            .get_insight_reactions(insight_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn create_adapted_insight(
        &self,
        insight: &AdaptedInsight,
    ) -> Result<Uuid, DatabaseError> {
        self.db
            .create_adapted_insight(insight)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_adapted_insight(&self, id: Uuid) -> Result<Option<AdaptedInsight>, DatabaseError> {
        self.db
            .get_adapted_insight(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_user_adaptation(
        &self,
        source_insight_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AdaptedInsight>, DatabaseError> {
        self.db
            .get_user_adaptation(source_insight_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_user_adapted_insights(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AdaptedInsight>, DatabaseError> {
        self.db
            .get_user_adapted_insights(user_id, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_adapted_insight_helpful(
        &self,
        id: Uuid,
        user_id: Uuid,
        was_helpful: bool,
    ) -> Result<bool, DatabaseError> {
        self.db
            .update_adapted_insight_helpful(id, user_id, was_helpful)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn search_discoverable_users(
        &self,
        query: &str,
        exclude_user_id: Uuid,
        limit: u32,
    ) -> Result<Vec<(Uuid, String, Option<String>)>, DatabaseError> {
        self.db
            .search_discoverable_users(query, exclude_user_id, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_friend_count(&self, user_id: Uuid) -> Result<i64, DatabaseError> {
        self.db
            .get_friend_count(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
