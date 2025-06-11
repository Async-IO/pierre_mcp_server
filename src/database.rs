// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Database Management
//!
//! This module provides database functionality for the multi-tenant Pierre MCP Server.
//! It handles user storage, token encryption, and secure data access patterns.

use crate::models::{User, EncryptedToken, DecryptedToken};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite, SqlitePool, Row};
use uuid::Uuid;

/// Database manager for user and token storage
#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
    encryption_key: Vec<u8>,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Ensure SQLite creates the database file if it doesn't exist
        let connection_options = if database_url.starts_with("sqlite:") {
            format!("{database_url}?mode=rwc")
        } else {
            database_url.to_string()
        };
        
        let pool = SqlitePool::connect(&connection_options).await?;
        
        let db = Self {
            pool,
            encryption_key,
        };
        
        // Run migrations
        db.migrate().await?;
        
        Ok(db)
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                strava_access_token TEXT,
                strava_refresh_token TEXT,
                strava_expires_at TEXT,
                strava_scope TEXT,
                strava_nonce TEXT,
                fitbit_access_token TEXT,
                fitbit_refresh_token TEXT,
                fitbit_expires_at TEXT,
                fitbit_scope TEXT,
                fitbit_nonce TEXT,
                created_at TEXT NOT NULL,
                last_active TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on email for fast lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a new user
    pub async fn create_user(&self, user: &User) -> Result<Uuid> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, display_name, password_hash, created_at, last_active, is_active)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(user.id.to_string())
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(user.created_at.to_rfc3339())
        .bind(user.last_active.to_rfc3339())
        .bind(user.is_active)
        .execute(&self.pool)
        .await?;

        Ok(user.id)
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?1")
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query("SELECT * FROM users WHERE email = ?1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row)?)),
            None => Ok(None),
        }
    }

    /// Get user by email, returning error if not found (for authentication)
    pub async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        match self.get_user_by_email(email).await? {
            Some(user) => Ok(user),
            None => Err(anyhow::anyhow!("User not found")),
        }
    }

    /// Update user's Strava token
    pub async fn update_strava_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let encrypted_token = EncryptedToken::new(
            access_token,
            refresh_token,
            expires_at,
            scope,
            &self.encryption_key,
        )?;

        sqlx::query(
            r#"
            UPDATE users 
            SET strava_access_token = ?1, strava_refresh_token = ?2, strava_expires_at = ?3, 
                strava_scope = ?4, strava_nonce = ?5, last_active = ?6
            WHERE id = ?7
            "#,
        )
        .bind(&encrypted_token.access_token)
        .bind(&encrypted_token.refresh_token)
        .bind(encrypted_token.expires_at.to_rfc3339())
        .bind(&encrypted_token.scope)
        .bind(&encrypted_token.nonce)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get decrypted Strava token for user
    pub async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r#"
            SELECT strava_access_token, strava_refresh_token, strava_expires_at, 
                   strava_scope, strava_nonce 
            FROM users WHERE id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let access_token: Option<String> = row.try_get("strava_access_token")?;
                let refresh_token: Option<String> = row.try_get("strava_refresh_token")?;
                let expires_at: Option<String> = row.try_get("strava_expires_at")?;
                let scope: Option<String> = row.try_get("strava_scope")?;
                let nonce: Option<String> = row.try_get("strava_nonce")?;

                if let (Some(access), Some(refresh), Some(expires), Some(scope), Some(nonce)) =
                    (access_token, refresh_token, expires_at, scope, nonce)
                {
                    let encrypted_token = EncryptedToken {
                        access_token: access,
                        refresh_token: refresh,
                        expires_at: DateTime::parse_from_rfc3339(&expires)?.with_timezone(&Utc),
                        scope,
                        nonce,
                    };

                    let decrypted = encrypted_token.decrypt(&self.encryption_key)?;
                    Ok(Some(decrypted))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Update user's Fitbit token
    pub async fn update_fitbit_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let encrypted_token = EncryptedToken::new(
            access_token,
            refresh_token,
            expires_at,
            scope,
            &self.encryption_key,
        )?;

        sqlx::query(
            r#"
            UPDATE users 
            SET fitbit_access_token = ?1, fitbit_refresh_token = ?2, fitbit_expires_at = ?3, 
                fitbit_scope = ?4, fitbit_nonce = ?5, last_active = ?6
            WHERE id = ?7
            "#,
        )
        .bind(&encrypted_token.access_token)
        .bind(&encrypted_token.refresh_token)
        .bind(encrypted_token.expires_at.to_rfc3339())
        .bind(&encrypted_token.scope)
        .bind(&encrypted_token.nonce)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get decrypted Fitbit token for user
    pub async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r#"
            SELECT fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, 
                   fitbit_scope, fitbit_nonce 
            FROM users WHERE id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let access_token: Option<String> = row.try_get("fitbit_access_token")?;
                let refresh_token: Option<String> = row.try_get("fitbit_refresh_token")?;
                let expires_at: Option<String> = row.try_get("fitbit_expires_at")?;
                let scope: Option<String> = row.try_get("fitbit_scope")?;
                let nonce: Option<String> = row.try_get("fitbit_nonce")?;

                if let (Some(access), Some(refresh), Some(expires), Some(scope), Some(nonce)) =
                    (access_token, refresh_token, expires_at, scope, nonce)
                {
                    let encrypted_token = EncryptedToken {
                        access_token: access,
                        refresh_token: refresh,
                        expires_at: DateTime::parse_from_rfc3339(&expires)?.with_timezone(&Utc),
                        scope,
                        nonce,
                    };

                    let decrypted = encrypted_token.decrypt(&self.encryption_key)?;
                    Ok(Some(decrypted))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Update user's last active timestamp
    pub async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE users SET last_active = ?1 WHERE id = ?2")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Convert database row to User model
    fn row_to_user(&self, row: sqlx::sqlite::SqliteRow) -> Result<User> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str)?;
        
        let email: String = row.try_get("email")?;
        let display_name: Option<String> = row.try_get("display_name")?;
        let password_hash: String = row.try_get("password_hash")?;
        
        let created_at_str: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        
        let last_active_str: String = row.try_get("last_active")?;
        let last_active = DateTime::parse_from_rfc3339(&last_active_str)?.with_timezone(&Utc);
        
        let is_active: bool = row.try_get("is_active")?;

        // Build encrypted tokens if they exist
        let strava_token = self.build_encrypted_token(&row, "strava")?;
        let fitbit_token = self.build_encrypted_token(&row, "fitbit")?;

        Ok(User {
            id,
            email,
            display_name,
            password_hash,
            strava_token,
            fitbit_token,
            created_at,
            last_active,
            is_active,
        })
    }

    /// Build encrypted token from database row
    fn build_encrypted_token(
        &self,
        row: &sqlx::sqlite::SqliteRow,
        provider: &str,
    ) -> Result<Option<EncryptedToken>> {
        let access_token: Option<String> = match provider {
            "strava" => row.try_get("strava_access_token")?,
            "fitbit" => row.try_get("fitbit_access_token")?,
            _ => None,
        };
        let refresh_token: Option<String> = match provider {
            "strava" => row.try_get("strava_refresh_token")?,
            "fitbit" => row.try_get("fitbit_refresh_token")?,
            _ => None,
        };
        let expires_at: Option<String> = match provider {
            "strava" => row.try_get("strava_expires_at")?,
            "fitbit" => row.try_get("fitbit_expires_at")?,
            _ => None,
        };
        let scope: Option<String> = match provider {
            "strava" => row.try_get("strava_scope")?,
            "fitbit" => row.try_get("fitbit_scope")?,
            _ => None,
        };
        let nonce: Option<String> = match provider {
            "strava" => row.try_get("strava_nonce")?,
            "fitbit" => row.try_get("fitbit_nonce")?,
            _ => None,
        };

        if let (Some(access), Some(refresh), Some(expires), Some(scope), Some(nonce)) =
            (access_token, refresh_token, expires_at, scope, nonce)
        {
            Ok(Some(EncryptedToken {
                access_token: access,
                refresh_token: refresh,
                expires_at: DateTime::parse_from_rfc3339(&expires)?.with_timezone(&Utc),
                scope,
                nonce,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Generate a random encryption key for token storage
pub fn generate_encryption_key() -> [u8; 32] {
    use ring::rand::{SecureRandom, SystemRandom};
    
    let rng = SystemRandom::new();
    let mut key = [0u8; 32];
    rng.fill(&mut key).expect("Failed to generate encryption key");
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_db() -> Database {
        let database_url = "sqlite::memory:";
        let encryption_key = generate_encryption_key().to_vec();
        
        Database::new(database_url, encryption_key).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let db = create_test_db().await;
        
        let user = User::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            Some("Test User".to_string())
        );
        let user_id = db.create_user(&user).await.unwrap();
        
        let retrieved = db.get_user(user_id).await.unwrap().unwrap();
        assert_eq!(retrieved.email, "test@example.com");
        assert_eq!(retrieved.display_name, Some("Test User".to_string()));
        assert_eq!(retrieved.password_hash, "hashed_password");
        assert!(retrieved.is_active);
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let db = create_test_db().await;
        
        let user = User::new(
            "email@example.com".to_string(),
            "hashed_password".to_string(),
            None
        );
        let user_id = db.create_user(&user).await.unwrap();
        
        let retrieved = db.get_user_by_email("email@example.com").await.unwrap().unwrap();
        assert_eq!(retrieved.id, user_id);
        assert_eq!(retrieved.email, "email@example.com");
    }

    #[tokio::test]
    async fn test_strava_token_storage() {
        let db = create_test_db().await;
        
        let user = User::new(
            "strava@example.com".to_string(),
            "hashed_password".to_string(),
            None
        );
        let user_id = db.create_user(&user).await.unwrap();
        
        let expires_at = Utc::now() + chrono::Duration::hours(6);
        
        // Store token
        db.update_strava_token(
            user_id,
            "access_token_123",
            "refresh_token_456",
            expires_at,
            "read,activity:read_all".to_string(),
        ).await.unwrap();
        
        // Retrieve token
        let token = db.get_strava_token(user_id).await.unwrap().unwrap();
        assert_eq!(token.access_token, "access_token_123");
        assert_eq!(token.refresh_token, "refresh_token_456");
        assert_eq!(token.scope, "read,activity:read_all");
        
        // Check token expiry is close to what we set
        let diff = (token.expires_at - expires_at).num_seconds().abs();
        assert!(diff < 2); // Within 2 seconds
    }

    #[tokio::test]
    async fn test_fitbit_token_storage() {
        let db = create_test_db().await;
        
        let user = User::new(
            "fitbit@example.com".to_string(),
            "hashed_password".to_string(),
            None
        );
        let user_id = db.create_user(&user).await.unwrap();
        
        let expires_at = Utc::now() + chrono::Duration::hours(8);
        
        // Store token
        db.update_fitbit_token(
            user_id,
            "fitbit_access_789",
            "fitbit_refresh_101112",
            expires_at,
            "activity heartrate profile".to_string(),
        ).await.unwrap();
        
        // Retrieve token
        let token = db.get_fitbit_token(user_id).await.unwrap().unwrap();
        assert_eq!(token.access_token, "fitbit_access_789");
        assert_eq!(token.refresh_token, "fitbit_refresh_101112");
        assert_eq!(token.scope, "activity heartrate profile");
    }

    #[tokio::test]
    async fn test_last_active_update() {
        let db = create_test_db().await;
        
        let user = User::new(
            "active@example.com".to_string(),
            "hashed_password".to_string(),
            None
        );
        let initial_active = user.last_active;
        let user_id = db.create_user(&user).await.unwrap();
        
        // Wait a bit and update
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        db.update_last_active(user_id).await.unwrap();
        
        let updated_user = db.get_user(user_id).await.unwrap().unwrap();
        assert!(updated_user.last_active > initial_active);
    }
}