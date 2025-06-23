//! OAuth token management database operations

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
    fn column_prefix(&self) -> &'static str {
        match self {
            OAuthProvider::Strava => "strava",
            OAuthProvider::Fitbit => "fitbit",
        }
    }
}

impl Database {
    /// Generic function to update OAuth token for any provider
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
            r#"
            UPDATE users SET
                {}_access_token = $2,
                {}_refresh_token = $3,
                {}_expires_at = $4,
                {}_scope = $5,
                {}_nonce = $6
            WHERE id = $1
            "#,
            prefix, prefix, prefix, prefix, prefix
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
    pub async fn get_oauth_token(
        &self,
        user_id: Uuid,
        provider: OAuthProvider,
    ) -> Result<Option<DecryptedToken>> {
        let prefix = provider.column_prefix();
        let query = format!(
            r#"
            SELECT {}_access_token, {}_refresh_token, {}_expires_at, 
                   {}_scope, {}_nonce
            FROM users WHERE id = $1
            "#,
            prefix, prefix, prefix, prefix, prefix
        );

        let row = sqlx::query(&query)
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let access_col = format!("{}_access_token", prefix);
            let refresh_col = format!("{}_refresh_token", prefix);
            let expires_col = format!("{}_expires_at", prefix);
            let scope_col = format!("{}_scope", prefix);
            let nonce_col = format!("{}_nonce", prefix);

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
                    expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_default(),
                    scope: scope.unwrap_or_default(),
                    nonce: nonce.unwrap_or_else(|| "legacy".to_string()),
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
    pub async fn clear_oauth_token(&self, user_id: Uuid, provider: OAuthProvider) -> Result<()> {
        let prefix = provider.column_prefix();
        let query = format!(
            r#"
            UPDATE users SET
                {}_access_token = NULL,
                {}_refresh_token = NULL,
                {}_expires_at = NULL,
                {}_scope = NULL,
                {}_nonce = NULL
            WHERE id = $1
            "#,
            prefix, prefix, prefix, prefix, prefix
        );

        sqlx::query(&query)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update Strava OAuth token for a user (legacy wrapper)
    pub async fn update_strava_token(&self, user_id: Uuid, token: &DecryptedToken) -> Result<()> {
        self.update_oauth_token(user_id, OAuthProvider::Strava, token)
            .await
    }

    /// Get Strava OAuth token for a user (legacy wrapper)
    pub async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        self.get_oauth_token(user_id, OAuthProvider::Strava).await
    }

    /// Clear Strava OAuth token for a user (legacy wrapper)
    pub async fn clear_strava_token(&self, user_id: Uuid) -> Result<()> {
        self.clear_oauth_token(user_id, OAuthProvider::Strava).await
    }

    /// Update Fitbit OAuth token for a user (legacy wrapper)
    pub async fn update_fitbit_token(&self, user_id: Uuid, token: &DecryptedToken) -> Result<()> {
        self.update_oauth_token(user_id, OAuthProvider::Fitbit, token)
            .await
    }

    /// Get Fitbit OAuth token for a user (legacy wrapper)
    pub async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        self.get_oauth_token(user_id, OAuthProvider::Fitbit).await
    }

    /// Clear Fitbit OAuth token for a user (legacy wrapper)
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
            password_hash: "hashed".to_string(),
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
            chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).unwrap();
        let token = DecryptedToken {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            expires_at: expires_at_truncated,
            scope: "read,activity:read_all".to_string(),
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
            email: format!("fitbit_{}@example.com", user_id),
            display_name: None,
            password_hash: "hashed".to_string(),
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
            chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).unwrap();
        let token = DecryptedToken {
            access_token: "fitbit_access_token".to_string(),
            refresh_token: "fitbit_refresh_token".to_string(),
            expires_at: expires_at_truncated,
            scope: "activity heartrate location".to_string(),
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
