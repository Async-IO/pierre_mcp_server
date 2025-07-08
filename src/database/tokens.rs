// ABOUTME: OAuth token management database operations
// ABOUTME: Handles encryption, storage, and retrieval of OAuth tokens

use super::{Database, EncryptionHelper};
use crate::models::{DecryptedToken, EncryptedToken};
use anyhow::Result;
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
    ) -> Result<()> {
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
                {prefix}_scope = $5,
                {prefix}_nonce = $6
            WHERE id = $1
            "
        );

        sqlx::query(&query)
            .bind(user_id.to_string())
            .bind(&encrypted.access_token)
            .bind(&encrypted.refresh_token)
            .bind(encrypted.expires_at.timestamp())
            .bind(&encrypted.scope)
            .bind(&encrypted.nonce)
            .execute(&self.pool)
            .await?;

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
    ) -> Result<Option<DecryptedToken>> {
        let prefix = provider.column_prefix();
        let query = format!(
            r"
            SELECT {prefix}_access_token, {prefix}_refresh_token, {prefix}_expires_at, 
                   {prefix}_scope, {prefix}_nonce
            FROM users WHERE id = $1
            "
        );

        let row = sqlx::query(&query)
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let access_col = format!("{prefix}_access_token");
            let refresh_col = format!("{prefix}_refresh_token");
            let expires_col = format!("{prefix}_expires_at");
            let scope_col = format!("{prefix}_scope");
            let nonce_col = format!("{prefix}_nonce");

            if let (Some(access), Some(refresh), Some(expires_at)) = (
                row.get::<Option<String>, _>(access_col.as_str()),
                row.get::<Option<String>, _>(refresh_col.as_str()),
                row.get::<Option<i64>, _>(expires_col.as_str()),
            ) {
                let scope: Option<String> = row.get(scope_col.as_str());
                let nonce: Option<String> = row.get(nonce_col.as_str());

                let encrypted = EncryptedToken {
                    access_token: access,
                    refresh_token: refresh,
                    expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {expires_at}"))?,
                    scope: scope.unwrap_or_default(),
                    nonce: nonce.unwrap_or_else(|| "legacy".into()),
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
    pub async fn clear_oauth_token(&self, user_id: Uuid, provider: OAuthProvider) -> Result<()> {
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
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update Strava OAuth token for a user (legacy wrapper)
    ///
    /// # Errors
    /// Returns an error if the underlying `update_oauth_token` call fails
    pub async fn update_strava_token(&self, user_id: Uuid, token: &DecryptedToken) -> Result<()> {
        self.update_oauth_token(user_id, OAuthProvider::Strava, token)
            .await
    }

    /// Get Strava OAuth token for a user (legacy wrapper)
    ///
    /// # Errors
    /// Returns an error if the underlying `get_oauth_token` call fails
    pub async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        self.get_oauth_token(user_id, OAuthProvider::Strava).await
    }

    /// Clear Strava OAuth token for a user (legacy wrapper)
    ///
    /// # Errors
    /// Returns an error if the underlying `clear_oauth_token` call fails
    pub async fn clear_strava_token(&self, user_id: Uuid) -> Result<()> {
        self.clear_oauth_token(user_id, OAuthProvider::Strava).await
    }

    /// Update Fitbit OAuth token for a user (legacy wrapper)
    ///
    /// # Errors
    /// Returns an error if the underlying `update_oauth_token` call fails
    pub async fn update_fitbit_token(&self, user_id: Uuid, token: &DecryptedToken) -> Result<()> {
        self.update_oauth_token(user_id, OAuthProvider::Fitbit, token)
            .await
    }

    /// Get Fitbit OAuth token for a user (legacy wrapper)
    ///
    /// # Errors
    /// Returns an error if the underlying `get_oauth_token` call fails
    pub async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        self.get_oauth_token(user_id, OAuthProvider::Fitbit).await
    }

    /// Clear Fitbit OAuth token for a user (legacy wrapper)
    ///
    /// # Errors
    /// Returns an error if the underlying `clear_oauth_token` call fails
    pub async fn clear_fitbit_token(&self, user_id: Uuid) -> Result<()> {
        self.clear_oauth_token(user_id, OAuthProvider::Fitbit).await
    }
}
