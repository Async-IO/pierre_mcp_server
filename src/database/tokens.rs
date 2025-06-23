//! OAuth token management database operations

use super::{Database, EncryptionHelper};
use crate::models::{DecryptedToken, EncryptedToken};
use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

impl Database {
    /// Update Strava OAuth token for a user
    pub async fn update_strava_token(&self, user_id: Uuid, token: &DecryptedToken) -> Result<()> {
        let encrypted = EncryptedToken::new(
            &token.access_token,
            &token.refresh_token,
            token.expires_at,
            token.scope.clone(),
            self.encryption_key(),
        )?;

        sqlx::query(
            r#"
            UPDATE users SET
                strava_access_token = $2,
                strava_refresh_token = $3,
                strava_expires_at = $4,
                strava_scope = $5,
                strava_nonce = $6
            WHERE id = $1
            "#,
        )
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

    /// Get Strava OAuth token for a user
    pub async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r#"
            SELECT strava_access_token, strava_refresh_token, strava_expires_at, 
                   strava_scope, strava_nonce
            FROM users WHERE id = $1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            if let (Some(access), Some(refresh), Some(expires_at)) = (
                row.get::<Option<String>, _>("strava_access_token"),
                row.get::<Option<String>, _>("strava_refresh_token"),
                row.get::<Option<i64>, _>("strava_expires_at"),
            ) {
                let scope: Option<String> = row.get("strava_scope");
                let nonce: Option<String> = row.get("strava_nonce");

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

    /// Update Fitbit OAuth token for a user
    pub async fn update_fitbit_token(&self, user_id: Uuid, token: &DecryptedToken) -> Result<()> {
        let encrypted = EncryptedToken::new(
            &token.access_token,
            &token.refresh_token,
            token.expires_at,
            token.scope.clone(),
            self.encryption_key(),
        )?;

        sqlx::query(
            r#"
            UPDATE users SET
                fitbit_access_token = $2,
                fitbit_refresh_token = $3,
                fitbit_expires_at = $4,
                fitbit_scope = $5,
                fitbit_nonce = $6
            WHERE id = $1
            "#,
        )
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

    /// Get Fitbit OAuth token for a user
    pub async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r#"
            SELECT fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, 
                   fitbit_scope, fitbit_nonce
            FROM users WHERE id = $1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            if let (Some(access), Some(refresh), Some(expires_at)) = (
                row.get::<Option<String>, _>("fitbit_access_token"),
                row.get::<Option<String>, _>("fitbit_refresh_token"),
                row.get::<Option<i64>, _>("fitbit_expires_at"),
            ) {
                let scope: Option<String> = row.get("fitbit_scope");
                let nonce: Option<String> = row.get("fitbit_nonce");

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

    /// Clear Strava token for a user (logout/disconnect)
    pub async fn clear_strava_token(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users SET
                strava_access_token = NULL,
                strava_refresh_token = NULL,
                strava_expires_at = NULL,
                strava_scope = NULL,
                strava_nonce = NULL
            WHERE id = $1
            "#,
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clear Fitbit token for a user (logout/disconnect)
    pub async fn clear_fitbit_token(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users SET
                fitbit_access_token = NULL,
                fitbit_refresh_token = NULL,
                fitbit_expires_at = NULL,
                fitbit_scope = NULL,
                fitbit_nonce = NULL
            WHERE id = $1
            "#,
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
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
