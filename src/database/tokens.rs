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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{User, UserTier};

    #[tokio::test]
    async fn test_strava_token_storage() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        // Create a test user
        let user = User {
            id: Uuid::new_v4(),
            email: format!("strava_{}@example.com", Uuid::new_v4()),
            display_name: None,
            password_hash: "hashed".into(),
            tier: UserTier::Starter,
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        };

        db.create_user(&user).await.expect("Failed to create user");

        // Create test token with timestamp precision truncated to seconds
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(3600);
        let expires_at_truncated =
            chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).expect("Valid timestamp");
        let token = DecryptedToken {
            access_token: "test_access_token".into(),
            refresh_token: "test_refresh_token".into(),
            expires_at: expires_at_truncated,
            scope: "read,activity:read_all".into(),
        };

        // Store token
        db.update_strava_token(user.id, &token)
            .await
            .expect("Failed to update Strava token");

        // Retrieve token
        let retrieved = db
            .get_strava_token(user.id)
            .await
            .expect("Failed to get Strava token")
            .expect("Token not found");

        assert_eq!(retrieved.access_token, token.access_token);
        assert_eq!(retrieved.refresh_token, token.refresh_token);
        assert_eq!(retrieved.expires_at, token.expires_at);
        assert_eq!(retrieved.scope, token.scope);

        // Clear token
        db.clear_strava_token(user.id)
            .await
            .expect("Failed to clear Strava token");

        // Verify cleared
        let cleared = db
            .get_strava_token(user.id)
            .await
            .expect("Failed to get Strava token");
        assert!(cleared.is_none());
    }

    #[tokio::test]
    async fn test_fitbit_token_storage() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        // Create a test user
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: format!("fitbit_{user_id}@example.com"),
            display_name: None,
            password_hash: "hashed".into(),
            tier: UserTier::Professional,
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        };

        db.create_user(&user).await.expect("Failed to create user");

        // Create test token with timestamp precision truncated to seconds
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(7200);
        let expires_at_truncated =
            chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).expect("Valid timestamp");
        let token = DecryptedToken {
            access_token: "fitbit_access_token".into(),
            refresh_token: "fitbit_refresh_token".into(),
            expires_at: expires_at_truncated,
            scope: "activity heartrate location".into(),
        };

        // Store token
        db.update_fitbit_token(user.id, &token)
            .await
            .expect("Failed to update Fitbit token");

        // Retrieve token
        let retrieved = db
            .get_fitbit_token(user.id)
            .await
            .expect("Failed to get Fitbit token")
            .expect("Token not found");

        assert_eq!(retrieved.access_token, token.access_token);
        assert_eq!(retrieved.refresh_token, token.refresh_token);
        assert_eq!(retrieved.expires_at, token.expires_at);
        assert_eq!(retrieved.scope, token.scope);
    }
}
