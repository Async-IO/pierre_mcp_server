//! User management database operations

use super::Database;
use crate::models::{EncryptedToken, User};
use anyhow::{anyhow, Result};
use sqlx::Row;
use uuid::Uuid;

impl Database {
    /// Create users and profiles tables
    pub(super) async fn migrate_users(&self) -> Result<()> {
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
                strava_access_token TEXT,
                strava_refresh_token TEXT,
                strava_expires_at INTEGER,
                strava_scope TEXT,
                strava_nonce TEXT,
                strava_last_sync DATETIME,
                fitbit_access_token TEXT,
                fitbit_refresh_token TEXT,
                fitbit_expires_at INTEGER,
                fitbit_scope TEXT,
                fitbit_nonce TEXT,
                fitbit_last_sync DATETIME,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_active DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user_profiles table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_profiles (
                user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                profile_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create or update a user
    pub async fn create_user(&self, user: &User) -> Result<Uuid> {
        // Check if user exists by email
        let existing = self.get_user_by_email(&user.email).await?;
        if let Some(existing_user) = existing {
            if existing_user.id != user.id {
                return Err(anyhow!("Email already in use by another user"));
            }
            // Update existing user (including tokens)
            let (strava_access, strava_refresh, strava_expires, strava_scope, strava_nonce) =
                if let Some(ref token) = user.strava_token {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                        Some(&token.nonce),
                    )
                } else {
                    (None, None, None, None, None)
                };

            let (fitbit_access, fitbit_refresh, fitbit_expires, fitbit_scope, fitbit_nonce) =
                if let Some(ref token) = user.fitbit_token {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                        Some(&token.nonce),
                    )
                } else {
                    (None, None, None, None, None)
                };

            sqlx::query(
                r#"
                UPDATE users SET
                    display_name = $2,
                    password_hash = $3,
                    tier = $4,
                    strava_access_token = $5,
                    strava_refresh_token = $6,
                    strava_expires_at = $7,
                    strava_scope = $8,
                    strava_nonce = $9,
                    fitbit_access_token = $10,
                    fitbit_refresh_token = $11,
                    fitbit_expires_at = $12,
                    fitbit_scope = $13,
                    fitbit_nonce = $14,
                    is_active = $15,
                    last_active = CURRENT_TIMESTAMP
                WHERE id = $1
                "#,
            )
            .bind(user.id.to_string())
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(user.tier.as_str())
            .bind(strava_access)
            .bind(strava_refresh)
            .bind(strava_expires)
            .bind(strava_scope)
            .bind(strava_nonce)
            .bind(fitbit_access)
            .bind(fitbit_refresh)
            .bind(fitbit_expires)
            .bind(fitbit_scope)
            .bind(fitbit_nonce)
            .bind(user.is_active)
            .execute(&self.pool)
            .await?;
        } else {
            // Insert new user (including tokens)
            let (strava_access, strava_refresh, strava_expires, strava_scope, strava_nonce) =
                if let Some(ref token) = user.strava_token {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                        Some(&token.nonce),
                    )
                } else {
                    (None, None, None, None, None)
                };

            let (fitbit_access, fitbit_refresh, fitbit_expires, fitbit_scope, fitbit_nonce) =
                if let Some(ref token) = user.fitbit_token {
                    (
                        Some(&token.access_token),
                        Some(&token.refresh_token),
                        Some(token.expires_at.timestamp()),
                        Some(&token.scope),
                        Some(&token.nonce),
                    )
                } else {
                    (None, None, None, None, None)
                };

            sqlx::query(
                r#"
                INSERT INTO users (
                    id, email, display_name, password_hash, tier, 
                    strava_access_token, strava_refresh_token, strava_expires_at, strava_scope, strava_nonce,
                    fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope, fitbit_nonce,
                    is_active, created_at, last_active
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                "#,
            )
            .bind(user.id.to_string())
            .bind(&user.email)
            .bind(&user.display_name)
            .bind(&user.password_hash)
            .bind(user.tier.as_str())
            .bind(strava_access)
            .bind(strava_refresh)
            .bind(strava_expires)
            .bind(strava_scope)
            .bind(strava_nonce)
            .bind(fitbit_access)
            .bind(fitbit_refresh)
            .bind(fitbit_expires)
            .bind(fitbit_scope)
            .bind(fitbit_nonce)
            .bind(user.is_active)
            .bind(user.created_at)
            .bind(user.last_active)
            .execute(&self.pool)
            .await?;
        }

        Ok(user.id)
    }

    /// Get a user by ID
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        self.get_user_impl("id", &user_id.to_string()).await
    }

    /// Get a user by ID (alias for compatibility)
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        self.get_user(user_id).await
    }

    /// Get a user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.get_user_impl("email", email).await
    }

    /// Get a user by email, returning an error if not found
    pub async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        self.get_user_by_email(email)
            .await?
            .ok_or_else(|| anyhow!("User not found with email: {}", email))
    }

    /// Internal implementation for getting a user
    async fn get_user_impl(&self, field: &str, value: &str) -> Result<Option<User>> {
        let query = format!(
            r#"
            SELECT id, email, display_name, password_hash, tier,
                   strava_access_token, strava_refresh_token, strava_expires_at, strava_scope, strava_nonce,
                   fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope, fitbit_nonce,
                   is_active, created_at, last_active
            FROM users WHERE {} = $1
            "#,
            field
        );

        let row = sqlx::query(&query)
            .bind(value)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let user = self.row_to_user(row)?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Convert a database row to a User struct
    fn row_to_user(&self, row: sqlx::sqlite::SqliteRow) -> Result<User> {
        let id: String = row.get("id");
        let email: String = row.get("email");
        let display_name: Option<String> = row.get("display_name");
        let password_hash: String = row.get("password_hash");
        let tier: String = row.get("tier");
        let is_active: bool = row.get("is_active");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let last_active: chrono::DateTime<chrono::Utc> = row.get("last_active");

        // Handle Strava token
        let strava_token = if let (Some(access), Some(refresh), Some(expires_at)) = (
            row.get::<Option<String>, _>("strava_access_token"),
            row.get::<Option<String>, _>("strava_refresh_token"),
            row.get::<Option<i64>, _>("strava_expires_at"),
        ) {
            let scope: Option<String> = row.get("strava_scope");
            let nonce: Option<String> = row.get("strava_nonce");

            Some(EncryptedToken {
                access_token: access,
                refresh_token: refresh,
                expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_default(),
                scope: scope.unwrap_or_default(),
                nonce: nonce.unwrap_or_else(|| "legacy".to_string()),
            })
        } else {
            None
        };

        // Handle Fitbit token
        let fitbit_token = if let (Some(access), Some(refresh), Some(expires_at)) = (
            row.get::<Option<String>, _>("fitbit_access_token"),
            row.get::<Option<String>, _>("fitbit_refresh_token"),
            row.get::<Option<i64>, _>("fitbit_expires_at"),
        ) {
            let scope: Option<String> = row.get("fitbit_scope");
            let nonce: Option<String> = row.get("fitbit_nonce");

            Some(EncryptedToken {
                access_token: access,
                refresh_token: refresh,
                expires_at: chrono::DateTime::from_timestamp(expires_at, 0).unwrap_or_default(),
                scope: scope.unwrap_or_default(),
                nonce: nonce.unwrap_or_else(|| "legacy".to_string()),
            })
        } else {
            None
        };

        Ok(User {
            id: Uuid::parse_str(&id)?,
            email,
            display_name,
            password_hash,
            tier: tier.parse()?,
            strava_token,
            fitbit_token,
            is_active,
            created_at,
            last_active,
        })
    }

    /// Update user's last active timestamp
    pub async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE users SET last_active = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get total user count
    pub async fn get_user_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Update or insert user profile data
    pub async fn upsert_user_profile(
        &self,
        user_id: Uuid,
        profile_data: serde_json::Value,
    ) -> Result<()> {
        let profile_json = serde_json::to_string(&profile_data)?;

        sqlx::query(
            r#"
            INSERT INTO user_profiles (user_id, profile_data, updated_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET
                profile_data = $2,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(user_id.to_string())
        .bind(profile_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user profile data
    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query(
            r#"
            SELECT profile_data FROM user_profiles WHERE user_id = $1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let profile_json: String = row.get("profile_data");
            let profile_data: serde_json::Value = serde_json::from_str(&profile_json)?;
            Ok(Some(profile_data))
        } else {
            Ok(None)
        }
    }

    /// Get last sync timestamp for a provider
    pub async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        let column = match provider {
            "strava" => "strava_last_sync",
            "fitbit" => "fitbit_last_sync",
            _ => return Err(anyhow!("Unsupported provider: {}", provider)),
        };

        let query = format!("SELECT {} FROM users WHERE id = $1", column);
        let last_sync: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(&query)
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(last_sync)
    }

    /// Update last sync timestamp for a provider
    pub async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let column = match provider {
            "strava" => "strava_last_sync",
            "fitbit" => "fitbit_last_sync",
            _ => return Err(anyhow!("Unsupported provider: {}", provider)),
        };

        let query = format!("UPDATE users SET {} = $1 WHERE id = $2", column);
        sqlx::query(&query)
            .bind(sync_time)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserTier;

    #[tokio::test]
    async fn test_create_and_get_user() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let user = User {
            id: Uuid::new_v4(),
            email: format!("test_{}@example.com", Uuid::new_v4()),
            display_name: Some("Test User".to_string()),
            password_hash: "hashed_password".to_string(),
            tier: UserTier::Starter,
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        };

        // Create user
        let user_id = db.create_user(&user).await.expect("Failed to create user");
        assert_eq!(user_id, user.id);

        // Get user by ID
        let retrieved = db
            .get_user(user.id)
            .await
            .expect("Failed to get user")
            .expect("User not found");
        assert_eq!(retrieved.email, user.email);
        assert_eq!(retrieved.display_name, user.display_name);
        assert_eq!(retrieved.tier, user.tier);

        // Get user by email
        let retrieved_by_email = db
            .get_user_by_email(&user.email)
            .await
            .expect("Failed to get user by email")
            .expect("User not found");
        assert_eq!(retrieved_by_email.id, user.id);
    }

    #[tokio::test]
    async fn test_last_active_update() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: format!("active_{}@example.com", user_id),
            display_name: None,
            password_hash: "hashed".to_string(),
            tier: UserTier::Starter,
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now() - chrono::Duration::hours(1),
        };

        db.create_user(&user).await.expect("Failed to create user");

        // Update last active
        db.update_last_active(user.id)
            .await
            .expect("Failed to update last active");

        // Verify update
        let updated = db
            .get_user(user.id)
            .await
            .expect("Failed to get user")
            .expect("User not found");

        assert!(updated.last_active > user.last_active);
    }
}
