// ABOUTME: OAuth token management database operations
// ABOUTME: Handles encryption, storage, and retrieval of OAuth tokens
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Option<String> ownership for OAuth token scope fields

use super::{Database, EncryptionHelper};
use crate::errors::{AppError, AppResult};
use crate::models::{DecryptedToken, EncryptedToken};
use sqlx::Row;
use uuid::Uuid;

/// OAuth provider types
#[derive(Debug, Clone, Copy)]
pub enum OAuthProvider {
    Strava,
    Fitbit,
}

impl OAuthProvider {
    /// Get the column prefix for this provider
    const fn column_prefix(self) -> &'static str {
        match self {
            Self::Strava => "strava",
            Self::Fitbit => "fitbit",
        }
    }
}

impl Database {
    /// Generic function to update OAuth token for any provider
    ///
    /// # Errors
    /// Returns an error if encryption fails or database update fails
    pub async fn update_oauth_token(
        &self,
        user_id: Uuid,
        provider: OAuthProvider,
        token: &DecryptedToken,
    ) -> AppResult<()> {
        let encrypted = EncryptedToken::new(
            &token.access_token,
            &token.refresh_token,
            token.expires_at,
            token.scope.clone(),
            self.encryption_key(),
        )?;

        let prefix = provider.column_prefix();
        let query = format!(
            r"
            UPDATE users SET
                {prefix}_access_token = $2,
                {prefix}_refresh_token = $3,
                {prefix}_expires_at = $4,
                {prefix}_scope = $5
            WHERE id = $1
            "
        );

        sqlx::query(&query)
            .bind(user_id.to_owned())
            .bind(&encrypted.access_token)
            .bind(&encrypted.refresh_token)
            .bind(encrypted.expires_at.timestamp())
            .bind(&encrypted.scope)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to update OAuth token: {e}")))?;

        Ok(())
    }

    /// Generic function to get OAuth token for any provider
    ///
    /// # Errors
    /// Returns an error if database query fails or decryption fails
    pub async fn get_oauth_token(
        &self,
        user_id: Uuid,
        provider: OAuthProvider,
    ) -> AppResult<Option<DecryptedToken>> {
        let prefix = provider.column_prefix();
        let query = format!(
            r"
            SELECT {prefix}_access_token, {prefix}_refresh_token, {prefix}_expires_at,
                   {prefix}_scope
            FROM users WHERE id = $1
            "
        );

        let row = sqlx::query(&query)
            .bind(user_id.to_owned())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to query OAuth token: {e}")))?;

        if let Some(row) = row {
            let access_col = format!("{prefix}_access_token");
            let refresh_col = format!("{prefix}_refresh_token");
            let expires_col = format!("{prefix}_expires_at");
            let scope_col = format!("{prefix}_scope");

            if let (Some(access), Some(refresh), Some(expires_at)) = (
                row.get::<Option<String>, _>(access_col.as_str()),
                row.get::<Option<String>, _>(refresh_col.as_str()),
                row.get::<Option<i64>, _>(expires_col.as_str()),
            ) {
                let scope: Option<String> = row.get(scope_col.as_str());

                let encrypted = EncryptedToken {
                    access_token: access,
                    refresh_token: refresh,
                    expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                        .ok_or_else(|| AppError::internal(format!("Invalid timestamp: {expires_at}")))?,
                    scope: scope.unwrap_or_default(),
                };

                let decrypted = encrypted.decrypt(self.encryption_key())?;
                Ok(Some(decrypted))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Generic function to clear OAuth token for any provider
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn clear_oauth_token(&self, user_id: Uuid, provider: OAuthProvider) -> AppResult<()> {
        let prefix = provider.column_prefix();
        let query = format!(
            r"
            UPDATE users SET
                {prefix}_access_token = NULL,
                {prefix}_refresh_token = NULL,
                {prefix}_expires_at = NULL,
                {prefix}_scope = NULL,
                {prefix}_nonce = NULL
            WHERE id = $1
            "
        );

        sqlx::query(&query)
            .bind(user_id.to_owned())
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to clear OAuth token: {e}")))?;

        Ok(())
    }






}
