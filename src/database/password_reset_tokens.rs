// ABOUTME: Database operations for password reset tokens
// ABOUTME: CRUD methods for one-time password reset tokens issued by admins
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database::Database;
use crate::errors::{AppError, AppResult};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

/// Duration before a password reset token expires (1 hour)
const RESET_TOKEN_TTL_HOURS: i64 = 1;

impl Database {
    /// Store a password reset token
    ///
    /// The `token_hash` should be a SHA-256 hash of the raw token — the raw token
    /// is returned to the admin and never stored in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails.
    pub async fn store_password_reset_token_impl(
        &self,
        user_id: Uuid,
        token_hash: &str,
        created_by: &str,
    ) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(RESET_TOKEN_TTL_HOURS);

        sqlx::query(
            r"
            INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, created_by, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(token_hash)
        .bind(expires_at.to_rfc3339())
        .bind(created_by)
        .bind(now.to_rfc3339())
        .execute(self.pool())
        .await
        .map_err(|e| AppError::database(format!("Failed to store password reset token: {e}")))?;

        Ok(id)
    }

    /// Consume a password reset token by its hash
    ///
    /// Returns the `user_id` if the token is valid: exists, not expired, and not yet used.
    /// Marks the token as used atomically to prevent replay.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the token doesn't exist or is already used/expired.
    pub async fn consume_password_reset_token_impl(&self, token_hash: &str) -> AppResult<Uuid> {
        let now = Utc::now().to_rfc3339();

        // Atomically find and mark the token as used
        let row = sqlx::query(
            r"
            UPDATE password_reset_tokens
            SET used_at = ?1
            WHERE token_hash = ?2
              AND used_at IS NULL
              AND expires_at > ?1
            RETURNING user_id
            ",
        )
        .bind(&now)
        .bind(token_hash)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::database(format!("Failed to consume reset token: {e}")))?;

        row.map_or_else(
            || {
                Err(AppError::not_found(
                    "Password reset token is invalid, expired, or already used",
                ))
            },
            |row| {
                let user_id_str: String = row.get("user_id");
                Uuid::parse_str(&user_id_str)
                    .map_err(|e| AppError::internal(format!("Invalid user_id in reset token: {e}")))
            },
        )
    }

    /// Invalidate all unused reset tokens for a user
    ///
    /// Called after a successful password change to prevent old tokens from being used.
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub async fn invalidate_user_reset_tokens_impl(&self, user_id: Uuid) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r"
            UPDATE password_reset_tokens
            SET used_at = ?1
            WHERE user_id = ?2
              AND used_at IS NULL
            ",
        )
        .bind(&now)
        .bind(user_id.to_string())
        .execute(self.pool())
        .await
        .map_err(|e| AppError::database(format!("Failed to invalidate reset tokens: {e}")))?;

        Ok(())
    }
}
