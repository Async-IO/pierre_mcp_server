// ABOUTME: Notification repository implementation
// ABOUTME: Handles OAuth completion notifications for MCP resource delivery
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::NotificationRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `NotificationRepository`
pub struct NotificationRepositoryImpl {
    db: Database,
}

impl NotificationRepositoryImpl {
    /// Create a new `NotificationRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NotificationRepository for NotificationRepositoryImpl {
    async fn store(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String, DatabaseError> {
        self.db
            .store_oauth_notification(user_id, provider, success, message, expires_at)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_unread(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>, DatabaseError> {
        self.db
            .get_unread_oauth_notifications(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn mark_read(&self, notification_id: &str, user_id: Uuid) -> Result<bool, DatabaseError> {
        self.db
            .mark_oauth_notification_read(notification_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, DatabaseError> {
        self.db
            .mark_all_oauth_notifications_read(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_all(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>, DatabaseError> {
        self.db
            .get_all_oauth_notifications(user_id, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
