// ABOUTME: OAuth notification database operations for storing completion events
// ABOUTME: Handles MCP notification delivery tracking for OAuth flows
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::Database;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// OAuth notification data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthNotification {
    /// Unique notification ID
    pub id: String,
    /// User ID this notification belongs to
    pub user_id: String,
    /// Provider name (e.g., "strava", "fitbit")
    pub provider: String,
    /// Whether OAuth flow succeeded
    pub success: bool,
    /// Notification message text
    pub message: String,
    /// Optional expiration timestamp as ISO 8601 string
    pub expires_at: Option<String>,
    /// When the notification was created
    pub created_at: DateTime<Utc>,
    /// When the notification was read (if read)
    pub read_at: Option<DateTime<Utc>>,
}

impl Database {
    /// Create `oauth_notifications` table
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database schema migration fails
    /// - Table creation fails
    /// - Index creation fails
    pub(super) async fn migrate_oauth_notifications(&self) -> Result<()> {
        // Create oauth_notifications table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth_notifications (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                success INTEGER NOT NULL DEFAULT 1,
                message TEXT NOT NULL,
                expires_at TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                read_at DATETIME,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indices for efficient queries
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_id 
            ON oauth_notifications (user_id)
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_unread 
            ON oauth_notifications (user_id, read_at) 
            WHERE read_at IS NULL
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store OAuth completion notification
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operation fails
    /// - User ID is invalid
    pub async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String> {
        let notification_id = Uuid::new_v4().to_string();

        sqlx::query(
            r"
            INSERT INTO oauth_notifications (id, user_id, provider, success, message, expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
        )
        .bind(&notification_id)
        .bind(user_id.to_string())
        .bind(provider)
        .bind(success)
        .bind(message)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(notification_id)
    }

    /// Get unread OAuth notifications for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - User ID parsing fails
    pub async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OAuthNotification>> {
        tracing::debug!("Querying unread notifications for user_id: {}", user_id);
        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, success, message, expires_at, created_at, read_at
            FROM oauth_notifications
            WHERE user_id = ?1 AND read_at IS NULL
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        tracing::debug!(
            "Found {} unread notification rows for user {}",
            rows.len(),
            user_id
        );

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(OAuthNotification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                provider: row.get("provider"),
                success: row.get::<i64, _>("success") != 0,
                message: row.get("message"),
                expires_at: row.get("expires_at"),
                created_at: row.get("created_at"),
                read_at: row.get("read_at"),
            });
        }

        Ok(notifications)
    }

    /// Mark OAuth notification as read
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Notification ID is invalid
    pub async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        let result = sqlx::query(
            r"
            UPDATE oauth_notifications 
            SET read_at = CURRENT_TIMESTAMP
            WHERE id = ?1 AND user_id = ?2 AND read_at IS NULL
            ",
        )
        .bind(notification_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Mark all OAuth notifications as read for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    pub async fn mark_all_oauth_notifications_read_impl(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query(
            r"
            UPDATE oauth_notifications 
            SET read_at = CURRENT_TIMESTAMP
            WHERE user_id = ?1 AND read_at IS NULL
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get all OAuth notifications for a user (read and unread)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    pub async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<OAuthNotification>> {
        let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {l}"));

        let query = format!(
            r"
            SELECT id, user_id, provider, success, message, expires_at, created_at, read_at
            FROM oauth_notifications
            WHERE user_id = ?1
            ORDER BY created_at DESC
            {limit_clause}
            "
        );

        let rows = sqlx::query(&query)
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(OAuthNotification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                provider: row.get("provider"),
                success: row.get::<i64, _>("success") != 0,
                message: row.get("message"),
                expires_at: row.get("expires_at"),
                created_at: row.get("created_at"),
                read_at: row.get("read_at"),
            });
        }

        Ok(notifications)
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Mark all OAuth notifications as read (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        self.mark_all_oauth_notifications_read_impl(user_id).await
    }
}
