// ABOUTME: PostgreSQL database implementation for cloud and production deployments
// ABOUTME: Provides enterprise-grade database support with connection pooling and scalability
//! `PostgreSQL` database implementation
//!
//! This module provides `PostgreSQL` support for cloud deployments,
//! implementing the same interface as the `SQLite` version.

use super::DatabaseProvider;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::database::A2AUsage;
use crate::models::{DecryptedToken, EncryptedToken, User, UserTier};
use crate::rate_limiting::JwtUsage;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Pool, Postgres, Row};
use std::fmt::Write;
use uuid::Uuid;

/// `PostgreSQL` database implementation
#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Pool<Postgres>,
    encryption_key: Vec<u8>,
}

impl PostgresDatabase {
    /// Encrypt a token using AES-256-GCM
    fn encrypt_token(&self, token: &DecryptedToken) -> Result<EncryptedToken> {
        // Use the EncryptedToken::new method for encryption
        EncryptedToken::new(
            &token.access_token,
            &token.refresh_token,
            token.expires_at,
            token.scope.clone(),
            &self.encryption_key,
        )
    }

    /// Decrypt a token using AES-256-GCM
    fn decrypt_token(&self, encrypted: &EncryptedToken) -> Result<DecryptedToken> {
        // Use the decrypt method from EncryptedToken
        encrypted.decrypt(&self.encryption_key)
    }
}

#[async_trait]
impl DatabaseProvider for PostgresDatabase {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        self.create_users_table().await?;
        self.create_user_profiles_table().await?;
        self.create_goals_table().await?;
        self.create_insights_table().await?;
        self.create_api_keys_tables().await?;
        self.create_a2a_tables().await?;
        self.create_admin_tables().await?;
        self.create_jwt_usage_table().await?;
        self.create_tenant_tables().await?; // Add tenant tables
        self.create_indexes().await?;
        Ok(())
    }

    async fn create_user(&self, user: &User) -> Result<Uuid> {
        let user_id = Uuid::new_v4();

        sqlx::query(
            r"
            INSERT INTO users (id, email, display_name, password_hash, tier, is_active, created_at, last_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(match user.tier {
            UserTier::Starter => "starter",
            UserTier::Professional => "professional",
            UserTier::Enterprise => "enterprise",
        })
        .bind(user.is_active)
        .bind(user.created_at)
        .bind(user.last_active)
        .execute(&self.pool)
        .await?;

        Ok(user_id)
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, is_active, created_at, last_active
            FROM users
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(User {
                    id: row.get("id"),
                    email: row.get("email"),
                    display_name: row.get("display_name"),
                    password_hash: row.get("password_hash"),
                    tier: {
                        let tier_str: String = row.get("tier");
                        match tier_str.as_str() {
                            "professional" => UserTier::Professional,
                            "enterprise" => UserTier::Enterprise,
                            _ => UserTier::Starter,
                        }
                    },
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                    is_active: row.get("is_active"),
                }))
            },
        )
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, is_active, created_at, last_active
            FROM users
            WHERE email = $1
            ",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(User {
                    id: row.get("id"),
                    email: row.get("email"),
                    display_name: row.get("display_name"),
                    password_hash: row.get("password_hash"),
                    tier: {
                        let tier_str: String = row.get("tier");
                        match tier_str.as_str() {
                            "professional" => UserTier::Professional,
                            "enterprise" => UserTier::Enterprise,
                            _ => UserTier::Starter,
                        }
                    },
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                    is_active: row.get("is_active"),
                }))
            },
        )
    }

    async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        self.get_user_by_email(email)
            .await?
            .ok_or_else(|| anyhow!("User with email {} not found", email))
    }

    async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users
            SET last_active = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    async fn update_strava_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let token = DecryptedToken {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expires_at,
            scope,
        };
        let encrypted = self.encrypt_token(&token)?;

        sqlx::query(
            r"
            UPDATE users
            SET strava_access_token = $1,
                strava_refresh_token = $2,
                strava_expires_at = $3,
                strava_scope = $4,
                strava_nonce = $5
            WHERE id = $6
            ",
        )
        .bind(&encrypted.access_token)
        .bind(&encrypted.refresh_token)
        .bind(expires_at)
        .bind(&token.scope)
        .bind(&encrypted.nonce)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r"
            SELECT strava_access_token, strava_refresh_token, strava_expires_at, strava_scope, strava_nonce
            FROM users
            WHERE id = $1 AND strava_access_token IS NOT NULL
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                let encrypted = EncryptedToken {
                    access_token: row.get("strava_access_token"),
                    refresh_token: row.get("strava_refresh_token"),
                    expires_at: row.get("strava_expires_at"),
                    scope: row.get("strava_scope"),
                    nonce: row.get("strava_nonce"),
                };

                let mut decrypted = self.decrypt_token(&encrypted)?;
                decrypted.expires_at = row.get("strava_expires_at");
                decrypted.scope = row.get("strava_scope");

                Ok(Some(decrypted))
            },
        )
    }

    async fn update_fitbit_token(
        &self,
        user_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
    ) -> Result<()> {
        let token = DecryptedToken {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expires_at,
            scope,
        };
        let encrypted = self.encrypt_token(&token)?;

        sqlx::query(
            r"
            UPDATE users
            SET fitbit_access_token = $1,
                fitbit_refresh_token = $2,
                fitbit_expires_at = $3,
                fitbit_scope = $4,
                fitbit_nonce = $5
            WHERE id = $6
            ",
        )
        .bind(&encrypted.access_token)
        .bind(&encrypted.refresh_token)
        .bind(expires_at)
        .bind(&token.scope)
        .bind(&encrypted.nonce)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        let row = sqlx::query(
            r"
            SELECT fitbit_access_token, fitbit_refresh_token, fitbit_expires_at, fitbit_scope, fitbit_nonce
            FROM users
            WHERE id = $1 AND fitbit_access_token IS NOT NULL
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let encrypted = EncryptedToken {
                access_token: row.get("fitbit_access_token"),
                refresh_token: row.get("fitbit_refresh_token"),
                expires_at: row.get("fitbit_expires_at"),
                scope: row.get("fitbit_scope"),
                nonce: row.get("fitbit_nonce"),
            };

            let mut decrypted = self.decrypt_token(&encrypted)?;
            decrypted.expires_at = row.get("fitbit_expires_at");
            decrypted.scope = row.get("fitbit_scope");

            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    async fn clear_strava_token(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users
            SET strava_access_token = NULL,
                strava_refresh_token = NULL,
                strava_expires_at = NULL,
                strava_scope = NULL,
                strava_nonce = NULL
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn clear_fitbit_token(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users
            SET fitbit_access_token = NULL,
                fitbit_refresh_token = NULL,
                fitbit_expires_at = NULL,
                fitbit_scope = NULL,
                fitbit_nonce = NULL
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO user_profiles (user_id, profile_data, updated_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT (user_id)
            DO UPDATE SET profile_data = $2, updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(user_id)
        .bind(&profile_data)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<Value>> {
        let row = sqlx::query(
            r"
            SELECT profile_data
            FROM user_profiles
            WHERE user_id = $1
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(|| Ok(None), |row| Ok(Some(row.get("profile_data"))))
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String> {
        let goal_id = Uuid::new_v4().to_string();

        sqlx::query(
            r"
            INSERT INTO goals (id, user_id, goal_data)
            VALUES ($1, $2, $3)
            ",
        )
        .bind(&goal_id)
        .bind(user_id)
        .bind(&goal_data)
        .execute(&self.pool)
        .await?;

        Ok(goal_id)
    }

    async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<Value>> {
        let rows = sqlx::query(
            r"
            SELECT goal_data
            FROM goals
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("goal_data")).collect())
    }

    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        // This would need to update the JSONB field - simplified implementation
        // Use const to avoid clippy warning about format-like strings
        const JSON_PATH: &str = "{current_value}";
        sqlx::query(
            r"
            UPDATE goals
            SET goal_data = jsonb_set(goal_data, $3::text, $1::text::jsonb),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            ",
        )
        .bind(current_value)
        .bind(goal_id)
        .bind(JSON_PATH)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        // First ensure the user_configurations table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_configurations (
                user_id TEXT PRIMARY KEY,
                config_data TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        let query = "SELECT config_data FROM user_configurations WHERE user_id = $1";

        let row = sqlx::query(query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(row.try_get("config_data")?))
        } else {
            Ok(None)
        }
    }

    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
        // First ensure the user_configurations table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_configurations (
                user_id TEXT PRIMARY KEY,
                config_data TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Insert or update configuration using PostgreSQL syntax
        let query = r"
            INSERT INTO user_configurations (user_id, config_data, updated_at) 
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET 
                config_data = EXCLUDED.config_data,
                updated_at = CURRENT_TIMESTAMP
        ";

        sqlx::query(query)
            .bind(user_id)
            .bind(config_json)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String> {
        let insight_id = Uuid::new_v4().to_string();

        sqlx::query(
            r"
            INSERT INTO insights (id, user_id, insight_type, content, metadata)
            VALUES ($1, $2, $3, $4, $5)
            ",
        )
        .bind(&insight_id)
        .bind(user_id)
        .bind("general") // Default insight type since it's not provided separately
        .bind(&insight_data)
        .bind(None::<Value>) // No separate metadata
        .execute(&self.pool)
        .await?;

        Ok(insight_id)
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>> {
        let limit = limit.unwrap_or(50);

        let rows = if let Some(insight_type) = insight_type {
            sqlx::query(
                r"
                SELECT content
                FROM insights
                WHERE user_id = $1 AND insight_type = $2
                ORDER BY created_at DESC
                LIMIT $3
                ",
            )
            .bind(user_id)
            .bind(insight_type)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r"
                SELECT content
                FROM insights
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                ",
            )
            .bind(user_id)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(|row| row.get("content")).collect())
    }

    async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO api_keys (id, user_id, name, key_prefix, key_hash, description, tier, is_active, rate_limit_requests, rate_limit_window_seconds, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ",
        )
        .bind(&api_key.id)
        .bind(api_key.user_id)
        .bind(&api_key.name)
        .bind(&api_key.key_prefix)
        .bind(&api_key.key_hash)
        .bind(&api_key.description)
        .bind(format!("{:?}", api_key.tier).to_lowercase())
        .bind(api_key.is_active)
        .bind(i32::try_from(api_key.rate_limit_requests).unwrap_or(i32::MAX))
        .bind(i32::try_from(api_key.rate_limit_window_seconds).unwrap_or(i32::MAX))
        .bind(api_key.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> Result<Option<ApiKey>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, key_prefix, key_hash, description, tier, is_active, rate_limit_requests, 
                   rate_limit_window_seconds, created_at, expires_at, last_used_at, updated_at
            FROM api_keys 
            WHERE id LIKE $1 AND key_hash = $2 AND is_active = true
            ",
        )
        .bind(format!("{prefix}%"))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(ApiKey {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    name: row.get("name"),
                    key_prefix: row.get("key_prefix"),
                    key_hash: row.get("key_hash"),
                    description: row.get("description"),
                    tier: match row.get::<String, _>("tier").to_lowercase().as_str() {
                        "trial" | "starter" => crate::api_keys::ApiKeyTier::Starter,
                        "professional" => crate::api_keys::ApiKeyTier::Professional,
                        "enterprise" => crate::api_keys::ApiKeyTier::Enterprise,
                        _ => crate::api_keys::ApiKeyTier::Trial,
                    },
                    is_active: row.get("is_active"),
                    rate_limit_requests: u32::try_from(
                        row.get::<i32, _>("rate_limit_requests").max(0),
                    )
                    .unwrap_or(0),
                    rate_limit_window_seconds: u32::try_from(
                        row.get::<i32, _>("rate_limit_window_seconds").max(0),
                    )
                    .unwrap_or(0),
                    created_at: row.get("created_at"),
                    expires_at: row.get("expires_at"),
                    last_used_at: row.get("last_used_at"),
                }))
            },
        )
    }

    // Remaining database methods follow the same PostgreSQL implementation pattern

    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, name, key_prefix, key_hash, description, tier, is_active, rate_limit_requests, 
                   rate_limit_window_seconds, created_at, expires_at, last_used_at, updated_at
            FROM api_keys 
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ApiKey {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                key_prefix: row.get("key_prefix"),
                key_hash: row.get("key_hash"),
                description: row.get("description"),
                tier: match row.get::<String, _>("tier").to_lowercase().as_str() {
                    "trial" | "starter" => crate::api_keys::ApiKeyTier::Starter,
                    "professional" => crate::api_keys::ApiKeyTier::Professional,
                    "enterprise" => crate::api_keys::ApiKeyTier::Enterprise,
                    _ => crate::api_keys::ApiKeyTier::Trial,
                },
                is_active: row.get("is_active"),
                rate_limit_requests: u32::try_from(row.get::<i32, _>("rate_limit_requests").max(0))
                    .unwrap_or(0),
                rate_limit_window_seconds: u32::try_from(
                    row.get::<i32, _>("rate_limit_window_seconds").max(0),
                )
                .unwrap_or(0),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_used_at: row.get("last_used_at"),
            })
            .collect())
    }

    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        sqlx::query(
            r"
            UPDATE api_keys 
            SET last_used_at = CURRENT_TIMESTAMP 
            WHERE id = $1
            ",
        )
        .bind(api_key_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r"
            UPDATE api_keys 
            SET is_active = false 
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(api_key_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, description, key_prefix, key_hash, tier, 
                   rate_limit_requests, rate_limit_window_seconds, is_active, 
                   created_at, last_used_at, expires_at, updated_at
            FROM api_keys
            WHERE id = $1
            ",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                use sqlx::Row;
                let tier_str: String = row.get("tier");
                let tier = match tier_str.as_str() {
                    "starter" => crate::api_keys::ApiKeyTier::Starter,
                    "professional" => crate::api_keys::ApiKeyTier::Professional,
                    "enterprise" => crate::api_keys::ApiKeyTier::Enterprise,
                    _ => crate::api_keys::ApiKeyTier::Trial, // Default to trial for unknown values (including "trial")
                };

                Ok(Some(ApiKey {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    name: row.get("name"),
                    key_prefix: row.get("key_prefix"),
                    description: row.get("description"),
                    key_hash: row.get("key_hash"),
                    tier,
                    rate_limit_requests: u32::try_from(
                        row.get::<i32, _>("rate_limit_requests").max(0),
                    )
                    .unwrap_or(0),
                    rate_limit_window_seconds: u32::try_from(
                        row.get::<i32, _>("rate_limit_window_seconds").max(0),
                    )
                    .unwrap_or(0),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                    last_used_at: row.get("last_used_at"),
                    expires_at: row.get("expires_at"),
                }))
            },
        )
    }

    async fn get_api_keys_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>> {
        let mut query: String = "SELECT ak.id, ak.user_id, ak.name, ak.description, ak.key_prefix, ak.key_hash, ak.tier, ak.rate_limit_requests, ak.rate_limit_window_seconds, ak.is_active, ak.created_at, ak.last_used_at, ak.expires_at, ak.updated_at FROM api_keys ak".into();

        let mut conditions = Vec::new();
        let mut param_count = 0;

        if user_email.is_some() {
            query.push_str(" JOIN users u ON ak.user_id = u.id");
            param_count += 1;
            conditions.push(format!("u.email = ${param_count}"));
        }

        if active_only {
            conditions.push("ak.is_active = true".into());
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY ak.created_at DESC");

        if let Some(_limit) = limit {
            param_count += 1;
            write!(&mut query, " LIMIT ${param_count}")
                .map_err(|e| anyhow::anyhow!("Failed to write LIMIT clause: {}", e))?;
            if let Some(_offset) = offset {
                param_count += 1;
                write!(&mut query, " OFFSET ${param_count}")
                    .map_err(|e| anyhow::anyhow!("Failed to write OFFSET clause: {}", e))?;
            }
        }

        let mut sqlx_query = sqlx::query(&query);

        if let Some(email) = user_email {
            sqlx_query = sqlx_query.bind(email);
        }

        if let Some(limit) = limit {
            sqlx_query = sqlx_query.bind(limit);
            if let Some(offset) = offset {
                sqlx_query = sqlx_query.bind(offset);
            }
        }

        let rows = sqlx_query.fetch_all(&self.pool).await?;

        let mut api_keys = Vec::with_capacity(rows.len());
        for row in rows {
            let tier_str: String = row.get("tier");
            let tier = match tier_str.as_str() {
                "starter" => crate::api_keys::ApiKeyTier::Starter,
                "professional" => crate::api_keys::ApiKeyTier::Professional,
                "enterprise" => crate::api_keys::ApiKeyTier::Enterprise,
                _ => crate::api_keys::ApiKeyTier::Trial, // Default to trial for unknown values (including "trial")
            };

            api_keys.push(ApiKey {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                key_prefix: row.get("key_prefix"),
                description: row.get("description"),
                key_hash: row.get("key_hash"),
                tier,
                rate_limit_requests: u32::try_from(row.get::<i32, _>("rate_limit_requests").max(0))
                    .unwrap_or(0),
                rate_limit_window_seconds: u32::try_from(
                    row.get::<i32, _>("rate_limit_window_seconds").max(0),
                )
                .unwrap_or(0),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                last_used_at: row.get("last_used_at"),
                expires_at: row.get("expires_at"),
            });
        }

        Ok(api_keys)
    }

    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        let result = sqlx::query(
            r"
            UPDATE api_keys 
            SET is_active = false 
            WHERE expires_at IS NOT NULL AND expires_at < CURRENT_TIMESTAMP AND is_active = true
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, name, key_prefix, key_hash, description, tier, is_active, rate_limit_requests, 
                   rate_limit_window_seconds, created_at, expires_at, last_used_at, updated_at
            FROM api_keys 
            WHERE expires_at IS NOT NULL AND expires_at < CURRENT_TIMESTAMP
            ORDER BY expires_at ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ApiKey {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                key_prefix: row.get("key_prefix"),
                key_hash: row.get("key_hash"),
                description: row.get("description"),
                tier: match row.get::<String, _>("tier").to_lowercase().as_str() {
                    "trial" | "starter" => crate::api_keys::ApiKeyTier::Starter,
                    "professional" => crate::api_keys::ApiKeyTier::Professional,
                    "enterprise" => crate::api_keys::ApiKeyTier::Enterprise,
                    _ => crate::api_keys::ApiKeyTier::Trial,
                },
                is_active: row.get("is_active"),
                rate_limit_requests: u32::try_from(row.get::<i32, _>("rate_limit_requests").max(0))
                    .unwrap_or(0),
                rate_limit_window_seconds: u32::try_from(
                    row.get::<i32, _>("rate_limit_window_seconds").max(0),
                )
                .unwrap_or(0),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_used_at: row.get("last_used_at"),
            })
            .collect())
    }

    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO api_key_usage (api_key_id, timestamp, endpoint, response_time_ms, status_code, 
                                     method, request_size_bytes, response_size_bytes, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ",
        )
        .bind(&usage.api_key_id)
        .bind(usage.timestamp)
        .bind(&usage.tool_name)
        .bind(usage.response_time_ms.map(|x| i32::try_from(x).unwrap_or(i32::MAX)))
        .bind(i16::try_from(usage.status_code).unwrap_or(i16::MAX))
        .bind(None::<String>)
        .bind(usage.request_size_bytes.map(|x| i32::try_from(x).unwrap_or(i32::MAX)))
        .bind(usage.response_size_bytes.map(|x| i32::try_from(x).unwrap_or(i32::MAX)))
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count
            FROM api_key_usage 
            WHERE api_key_id = $1 AND timestamp >= CURRENT_DATE
            ",
        )
        .bind(api_key_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(u32::try_from(row.get::<i64, _>("count").max(0)).unwrap_or(0))
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats> {
        let row = sqlx::query(
            r"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as failed_requests,
                SUM(response_time_ms) as total_response_time,
                SUM(request_size_bytes) as total_request_size,
                SUM(response_size_bytes) as total_response_size
            FROM api_key_usage 
            WHERE api_key_id = $1 AND timestamp >= $2 AND timestamp <= $3
            ",
        )
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        // Get tool usage aggregation
        let tool_usage_stats = sqlx::query(
            r"
            SELECT tool_name, 
                   COUNT(*) as tool_count,
                   AVG(response_time_ms) as avg_response_time,
                   COUNT(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 END) as success_count
            FROM api_key_usage
            WHERE api_key_id = $1 AND timestamp >= $2 AND timestamp <= $3
            GROUP BY tool_name
            ORDER BY tool_count DESC
            ",
        )
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        let mut tool_usage = serde_json::Map::new();
        for tool_row in tool_usage_stats {
            let tool_name: String = tool_row.get("tool_name");
            let tool_count: i64 = tool_row.get("tool_count");
            let avg_response_time: Option<f64> = tool_row.get("avg_response_time");
            let success_count: i64 = tool_row.get("success_count");

            tool_usage.insert(
                tool_name,
                serde_json::json!({
                    "count": tool_count,
                    "success_count": success_count,
                    "avg_response_time_ms": avg_response_time.unwrap_or(0.0),
                    "success_rate": if tool_count > 0 { 
                        f64::from(u32::try_from(success_count).unwrap_or(0)) / f64::from(u32::try_from(tool_count).unwrap_or(1))
                    } else { 0.0 }
                }),
            );
        }

        Ok(ApiKeyUsageStats {
            api_key_id: api_key_id.to_string(),
            period_start: start_date,
            period_end: end_date,
            total_requests: u32::try_from(row.get::<i64, _>("total_requests").max(0)).unwrap_or(0),
            successful_requests: u32::try_from(row.get::<i64, _>("successful_requests").max(0))
                .unwrap_or(0),
            failed_requests: u32::try_from(row.get::<i64, _>("failed_requests").max(0))
                .unwrap_or(0),
            total_response_time_ms: row
                .get::<Option<i64>, _>("total_response_time")
                .map_or(0u64, |v| u64::try_from(v.max(0)).unwrap_or(0)),
            tool_usage: serde_json::Value::Object(tool_usage),
        })
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO jwt_usage (
                user_id, timestamp, endpoint, response_time_ms, status_code,
                method, request_size_bytes, response_size_bytes, 
                ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ",
        )
        .bind(usage.user_id)
        .bind(usage.timestamp)
        .bind(&usage.endpoint)
        .bind(
            usage
                .response_time_ms
                .map(|t| i32::try_from(t).unwrap_or(i32::MAX)),
        )
        .bind(i32::from(usage.status_code))
        .bind(&usage.method)
        .bind(
            usage
                .request_size_bytes
                .map(|s| i32::try_from(s).unwrap_or(i32::MAX)),
        )
        .bind(
            usage
                .response_size_bytes
                .map(|s| i32::try_from(s).unwrap_or(i32::MAX)),
        )
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count
            FROM jwt_usage 
            WHERE user_id = $1 AND timestamp >= DATE_TRUNC('month', CURRENT_DATE)
            ",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(u32::try_from(row.get::<i64, _>("count").max(0)).unwrap_or(0))
    }

    async fn get_request_logs(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        // Build query with proper column mapping for RequestLog struct
        let base_query = r"
            SELECT 
                uuid_generate_v4()::text as id,
                timestamp,
                api_key_id,
                'Unknown' as api_key_name,
                COALESCE(endpoint, 'unknown') as tool_name,
                status_code::integer as status_code,
                response_time_ms,
                NULL::text as error_message,
                request_size_bytes,
                response_size_bytes
            FROM api_key_usage 
            WHERE 1=1
        ";

        let mut condition_strings = Vec::new();

        let mut param_count = 0;
        if api_key_id.is_some() {
            param_count += 1;
            let condition = format!(" AND api_key_id = ${param_count}");
            condition_strings.push(condition);
        }
        if start_time.is_some() {
            param_count += 1;
            let condition = format!(" AND timestamp >= ${param_count}");
            condition_strings.push(condition);
        }
        if end_time.is_some() {
            param_count += 1;
            let condition = format!(" AND timestamp <= ${param_count}");
            condition_strings.push(condition);
        }
        if status_filter.is_some() {
            param_count += 1;
            let condition = format!(" AND status_code::text LIKE ${param_count}");
            condition_strings.push(condition);
        }
        if tool_filter.is_some() {
            param_count += 1;
            let condition = format!(" AND endpoint ILIKE ${param_count}");
            condition_strings.push(condition);
        }

        let full_query = format!(
            "{}{} ORDER BY timestamp DESC LIMIT 1000",
            base_query,
            condition_strings.join("")
        );

        // Build query with proper parameter binding
        let mut query_builder =
            sqlx::query_as::<_, crate::dashboard_routes::RequestLog>(&full_query);

        if let Some(key_id) = api_key_id {
            query_builder = query_builder.bind(key_id);
        }
        if let Some(start) = start_time {
            query_builder = query_builder.bind(start);
        }
        if let Some(end) = end_time {
            query_builder = query_builder.bind(end);
        }
        if let Some(status) = status_filter {
            query_builder = query_builder.bind(format!("{status}%"));
        }
        if let Some(tool) = tool_filter {
            query_builder = query_builder.bind(format!("%{tool}%"));
        }

        let results = query_builder.fetch_all(&self.pool).await?;
        Ok(results)
    }

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        let user_count_row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await?;

        let api_key_count_row =
            sqlx::query("SELECT COUNT(*) as count FROM api_keys WHERE is_active = true")
                .fetch_one(&self.pool)
                .await?;

        let user_count = u64::try_from(user_count_row.get::<i64, _>("count").max(0)).unwrap_or(0);
        let api_key_count =
            u64::try_from(api_key_count_row.get::<i64, _>("count").max(0)).unwrap_or(0);

        Ok((user_count, api_key_count))
    }

    // A2A methods
    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        sqlx::query(
            r"
            INSERT INTO a2a_clients (client_id, user_id, name, description, client_secret_hash, 
                                    api_key_hash, capabilities, redirect_uris, 
                                    is_active, rate_limit_per_minute, rate_limit_per_day)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ",
        )
        .bind(&client.id)
        .bind(Uuid::new_v4()) // Generate a user_id since A2AClient doesn't have one
        .bind(&client.name)
        .bind(&client.description)
        .bind(client_secret) // Use actual client_secret
        .bind(api_key_id) // Using api_key_id as api_key_hash
        .bind(&client.capabilities)
        .bind(&client.redirect_uris)
        .bind(client.is_active)
        .bind(100i32) // Default rate limit
        .bind(10000i32) // Default daily rate limit
        .execute(&self.pool)
        .await?;

        Ok(client.id.clone())
    }

    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r"
            SELECT client_id, user_id, name, description, client_secret_hash, capabilities, 
                   redirect_uris, contact_email, is_active, rate_limit_per_minute, 
                   rate_limit_per_day, created_at, updated_at
            FROM a2a_clients
            WHERE client_id = $1
            ",
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(A2AClient {
                    id: row.get("client_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    public_key: row.get("client_secret_hash"), // Map client_secret_hash to public_key
                    capabilities: row.get("capabilities"),
                    redirect_uris: row.get("redirect_uris"),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                    permissions: vec!["read_activities".into()], // Default permission
                    rate_limit_requests: u32::try_from(
                        row.get::<i32, _>("rate_limit_per_minute").max(0),
                    )
                    .unwrap_or(0),
                    rate_limit_window_seconds: 60, // 1 minute in seconds
                    updated_at: row.get("updated_at"),
                }))
            },
        )
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r"
            SELECT client_id, user_id, name, description, client_secret_hash, capabilities, 
                   redirect_uris, contact_email, is_active, rate_limit_per_minute, 
                   rate_limit_per_day, created_at, updated_at
            FROM a2a_clients
            WHERE name = $1
            ",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(A2AClient {
                    id: row.get("client_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    public_key: row.get("client_secret_hash"), // Map client_secret_hash to public_key
                    capabilities: row.get("capabilities"),
                    redirect_uris: row.get("redirect_uris"),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                    permissions: vec!["read_activities".into()], // Default permission
                    rate_limit_requests: u32::try_from(
                        row.get::<i32, _>("rate_limit_per_minute").max(0),
                    )
                    .unwrap_or(0),
                    rate_limit_window_seconds: 60, // 1 minute in seconds
                    updated_at: row.get("updated_at"),
                }))
            },
        )
    }

    async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        let rows = sqlx::query(
            r"
            SELECT client_id, user_id, name, description, client_secret_hash, capabilities, 
                   redirect_uris, contact_email, is_active, rate_limit_per_minute, 
                   rate_limit_per_day, created_at, updated_at
            FROM a2a_clients
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut clients = Vec::new();
        for row in rows {
            clients.push(A2AClient {
                id: row.get("client_id"),
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("client_secret_hash"), // Map client_secret_hash to public_key
                capabilities: row.get("capabilities"),
                redirect_uris: row.get("redirect_uris"),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions: vec!["read_activities".into()], // Default permission
                rate_limit_requests: u32::try_from(
                    row.get::<i32, _>("rate_limit_per_minute").max(0),
                )
                .unwrap_or(0),
                rate_limit_window_seconds: 60, // 1 minute in seconds
                updated_at: row.get("updated_at"),
            });
        }

        Ok(clients)
    }

    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        let query =
            "UPDATE a2a_clients SET is_active = false, updated_at = NOW() WHERE client_id = $1";

        let result = sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("A2A client not found: {}", client_id));
        }

        Ok(())
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        let query = "SELECT client_id, client_secret_hash FROM a2a_clients WHERE client_id = $1 AND is_active = true";

        let row = sqlx::query(query)
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                let id: String = row.get("client_id");
                let secret: String = row.get("client_secret_hash");
                Ok(Some((id, secret)))
            },
        )
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        let query =
            "UPDATE a2a_sessions SET expires_at = NOW() - INTERVAL '1 hour' WHERE client_id = $1";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        let query = "UPDATE api_keys SET is_active = false WHERE id IN (SELECT api_key_id FROM a2a_client_api_keys WHERE client_id = $1)";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(expires_in_hours);
        let scopes_json = serde_json::to_string(granted_scopes)?;

        sqlx::query(
            r"
            INSERT INTO a2a_sessions (
                session_id, client_id, user_id, granted_scopes, created_at, expires_at, last_activity
            ) VALUES ($1, $2, $3, $4, $5, $6, $5)
            ",
        )
        .bind(&session_id)
        .bind(client_id)
        .bind(user_id)
        .bind(&scopes_json)
        .bind(chrono::Utc::now())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(session_id)
    }

    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        let row = sqlx::query(
            r"
            SELECT session_token, client_id, user_id, granted_scopes, 
                   expires_at, last_activity, created_at
            FROM a2a_sessions
            WHERE session_token = $1 AND expires_at > NOW()
            ",
        )
        .bind(session_token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let scopes_str: String = row.try_get("granted_scopes")?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_str).unwrap_or_else(|_| vec![]);

            Ok(Some(A2ASession {
                id: row.try_get("session_token")?,
                client_id: row.try_get("client_id")?,
                user_id: row.try_get("user_id")?,
                granted_scopes: scopes,
                expires_at: row.try_get("expires_at")?,
                last_activity: row.try_get("last_activity")?,
                created_at: row.try_get("created_at")?,
                requests_count: 0, // Would need to be tracked separately
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        sqlx::query("UPDATE a2a_sessions SET last_activity = NOW() WHERE session_token = $1")
            .bind(session_token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        let rows = sqlx::query(
            r"
            SELECT session_token, client_id, user_id, granted_scopes, 
                   expires_at, last_activity, created_at, requests_count
            FROM a2a_sessions
            WHERE client_id = $1 AND expires_at > NOW()
            ORDER BY last_activity DESC
            ",
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let user_id_str: Option<String> = row.get("user_id");
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()?;

            let granted_scopes_str: String = row.get("granted_scopes");
            let granted_scopes = granted_scopes_str
                .split(',')
                .map(std::string::ToString::to_string)
                .collect();

            sessions.push(A2ASession {
                id: row.get("session_token"),
                client_id: row.get("client_id"),
                user_id,
                granted_scopes,
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_activity: row.get("last_activity"),
                requests_count: u64::try_from(row.get::<i32, _>("requests_count").max(0))
                    .unwrap_or(0),
            });
        }

        Ok(sessions)
    }

    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String> {
        use uuid::Uuid;

        let uuid = Uuid::new_v4().simple();
        let task_id = format!("task_{uuid}");
        let input_json = serde_json::to_string(input_data)?;

        sqlx::query(
            r"
            INSERT INTO a2a_tasks 
            (task_id, client_id, session_id, task_type, input_data, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ",
        )
        .bind(&task_id)
        .bind(client_id)
        .bind(session_id)
        .bind(task_type)
        .bind(&input_json)
        .bind("pending")
        .execute(&self.pool)
        .await?;

        Ok(task_id)
    }

    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        let row = sqlx::query(
            r"
            SELECT task_id, client_id, session_id, task_type, input_data,
                   status, result_data, method, created_at, updated_at
            FROM a2a_tasks
            WHERE task_id = $1
            ",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let input_str: String = row.try_get("input_data")?;
            let input_data: Value = serde_json::from_str(&input_str).unwrap_or(Value::Null);

            // Validate input data structure
            if !input_data.is_null() && !input_data.is_object() {
                tracing::warn!(
                    "Invalid input data structure for task, expected object but got: {:?}",
                    input_data
                );
            }

            let result_data = row
                .try_get::<Option<String>, _>("result_data")
                .map_or(None, |result_str| {
                    result_str.and_then(|s| serde_json::from_str(&s).ok())
                });

            let status_str: String = row.try_get("status")?;
            let status = match status_str.as_str() {
                "running" => TaskStatus::Running,
                "completed" => TaskStatus::Completed,
                "failed" => TaskStatus::Failed,
                "cancelled" => TaskStatus::Cancelled,
                _ => TaskStatus::Pending, // Default for unknown values (including "pending")
            };

            Ok(Some(A2ATask {
                id: row.try_get("task_id")?,
                status,
                created_at: row.try_get("created_at")?,
                completed_at: row.try_get("updated_at")?,
                result: result_data.clone(),
                error: row.try_get("method")?,
                client_id: row
                    .try_get("client_id")
                    .unwrap_or_else(|_| "unknown".into()),
                task_type: row.try_get("task_type")?,
                input_data,
                output_data: result_data,
                error_message: row.try_get("method")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        let status_str = match status {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        };

        let result_json = result.map(serde_json::to_string).transpose()?;

        sqlx::query(
            r"
            UPDATE a2a_tasks 
            SET status = $1, result_data = $2, method = $3, updated_at = NOW()
            WHERE task_id = $4
            ",
        )
        .bind(status_str)
        .bind(result_json)
        .bind(error)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()> {
        let _ = sqlx::query(
            r"
            INSERT INTO a2a_usage 
            (client_id, session_token, endpoint, status_code, 
             response_time_ms, request_size_bytes, response_size_bytes, timestamp,
             method, ip_address, user_agent, protocol_version)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ",
        )
        .bind(&usage.client_id)
        .bind(&usage.session_token)
        .bind(&usage.tool_name)
        .bind(i32::from(usage.status_code))
        .bind(
            usage
                .response_time_ms
                .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
        )
        .bind(
            usage
                .request_size_bytes
                .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
        )
        .bind(
            usage
                .response_size_bytes
                .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
        )
        .bind(usage.timestamp)
        .bind(None::<String>)
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .bind(&usage.protocol_version)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as usage_count
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp >= NOW() - INTERVAL '1 hour'
            ",
        )
        .bind(client_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get("usage_count")?;
        Ok(u32::try_from(count.max(0)).unwrap_or(0))
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        let row = sqlx::query(
            r"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code < 400 THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as failed_requests,
                AVG(response_time_ms) as avg_response_time,
                SUM(request_size_bytes) as total_request_bytes,
                SUM(response_size_bytes) as total_response_bytes
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp BETWEEN $2 AND $3
            ",
        )
        .bind(client_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        use sqlx::Row;
        let total_requests: i64 = row.try_get("total_requests")?;
        let successful_requests: i64 = row.try_get("successful_requests")?;
        let failed_requests: i64 = row.try_get("failed_requests")?;
        let avg_response_time: Option<f64> = row.try_get("avg_response_time")?;
        let total_request_bytes: Option<i64> = row.try_get("total_request_bytes")?;
        let total_response_bytes: Option<i64> = row.try_get("total_response_bytes")?;

        // Log byte usage for monitoring
        if let (Some(req_bytes), Some(resp_bytes)) = (total_request_bytes, total_response_bytes) {
            tracing::debug!(
                "A2A client {} usage: {} req bytes, {} resp bytes",
                client_id,
                req_bytes,
                resp_bytes
            );
        }

        Ok(crate::database::A2AUsageStats {
            client_id: client_id.to_string(),
            period_start: start_date,
            period_end: end_date,
            total_requests: u32::try_from(total_requests.max(0)).unwrap_or(0),
            successful_requests: u32::try_from(successful_requests.max(0)).unwrap_or(0),
            failed_requests: u32::try_from(failed_requests.max(0)).unwrap_or(0),
            avg_response_time_ms: avg_response_time.map(|t| {
                if t.is_nan() || t.is_infinite() || t < 0.0 {
                    0
                } else if t > f64::from(u32::MAX) {
                    u32::MAX
                } else {
                    // Convert to integer via string to avoid casting issues
                    let rounded = t.round();
                    let as_string = format!("{rounded:.0}");
                    as_string.parse::<u32>().unwrap_or(0)
                }
            }),
            total_request_bytes: total_request_bytes.map(|b| u64::try_from(b.max(0)).unwrap_or(0)),
            total_response_bytes: total_response_bytes
                .map(|b| u64::try_from(b.max(0)).unwrap_or(0)),
        })
    }

    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        let rows = sqlx::query(
            r"
            SELECT 
                DATE_TRUNC('day', timestamp) as day,
                COUNT(CASE WHEN status_code < 400 THEN 1 END) as success_count,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as error_count
            FROM a2a_usage
            WHERE client_id = $1 
              AND timestamp >= NOW() - INTERVAL '$2 days'
            GROUP BY DATE_TRUNC('day', timestamp)
            ORDER BY day
            ",
        )
        .bind(client_id)
        .bind(i32::try_from(days).unwrap_or(i32::MAX))
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            use sqlx::Row;
            let day: DateTime<Utc> = row.try_get("day")?;
            let success_count: i64 = row.try_get("success_count")?;
            let error_count: i64 = row.try_get("error_count")?;

            result.push((
                day,
                u32::try_from(success_count.max(0)).unwrap_or(0),
                u32::try_from(error_count.max(0)).unwrap_or(0),
            ));
        }

        Ok(result)
    }

    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        let column = match provider {
            "strava" => "strava_last_sync",
            "fitbit" => "fitbit_last_sync",
            _ => return Err(anyhow!("Unsupported provider: {provider}")),
        };

        let query = format!("SELECT {column} FROM users WHERE id = $1");
        let last_sync: Option<DateTime<Utc>> = sqlx::query_scalar(&query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(last_sync)
    }

    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        let column = match provider {
            "strava" => "strava_last_sync",
            "fitbit" => "fitbit_last_sync",
            _ => return Err(anyhow!("Unsupported provider: {provider}")),
        };

        let query = format!("UPDATE users SET {column} = $1 WHERE id = $2");
        sqlx::query(&query)
            .bind(sync_time)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        let rows = sqlx::query(
            r"
            SELECT endpoint, COUNT(*) as usage_count,
                   AVG(response_time_ms) as avg_response_time,
                   COUNT(CASE WHEN status_code < 400 THEN 1 END) as success_count,
                   COUNT(CASE WHEN status_code >= 400 THEN 1 END) as error_count
            FROM api_key_usage aku
            JOIN api_keys ak ON aku.api_key_id = ak.id
            WHERE ak.user_id = $1 AND aku.timestamp BETWEEN $2 AND $3
            GROUP BY endpoint
            ORDER BY usage_count DESC
            LIMIT 10
            ",
        )
        .bind(user_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await?;

        let mut tool_usage = Vec::new();
        for row in rows {
            use sqlx::Row;

            let endpoint: String = row.try_get("endpoint").unwrap_or_else(|_| "unknown".into());
            let usage_count: i64 = row.try_get("usage_count").unwrap_or(0);
            let avg_response_time: Option<f64> = row.try_get("avg_response_time").ok();
            let success_count: i64 = row.try_get("success_count").unwrap_or(0);
            let error_count: i64 = row.try_get("error_count").unwrap_or(0);

            // Log error rate for monitoring
            if error_count > 0 {
                let error_rate = f64::from(u32::try_from(error_count.max(0)).unwrap_or(0))
                    / f64::from(u32::try_from(usage_count.max(1)).unwrap_or(1));
                if error_rate > 0.1 {
                    tracing::warn!(
                        "High error rate for endpoint {}: {:.2}% ({} errors out of {} requests)",
                        endpoint,
                        error_rate * 100.0,
                        error_count,
                        usage_count
                    );
                }
            }

            tool_usage.push(crate::dashboard_routes::ToolUsage {
                tool_name: endpoint,
                request_count: u64::try_from(usage_count.max(0)).unwrap_or(0),
                success_rate: if usage_count > 0 {
                    f64::from(u32::try_from(success_count.max(0)).unwrap_or(0))
                        / f64::from(u32::try_from(usage_count.max(1)).unwrap_or(1))
                } else {
                    0.0
                },
                average_response_time: avg_response_time.unwrap_or(0.0),
            });
        }

        Ok(tool_usage)
    }

    // ================================
    // Admin Token Management (PostgreSQL)
    // ================================

    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        use crate::admin::{
            jwt::AdminJwtManager,
            models::{AdminPermissions, GeneratedAdminToken},
        };
        use uuid::Uuid;

        // Generate unique token ID
        let uuid = Uuid::new_v4().simple();
        let token_id = format!("admin_{uuid}");

        // Generate JWT secret and manager
        let jwt_secret = AdminJwtManager::generate_jwt_secret();
        let jwt_manager = AdminJwtManager::with_secret(&jwt_secret);

        // Get permissions
        let permissions = request.permissions.as_ref().map_or_else(
            || {
                if request.is_super_admin {
                    AdminPermissions::super_admin()
                } else {
                    AdminPermissions::default_admin()
                }
            },
            |perms| AdminPermissions::new(perms.clone()),
        );

        // Calculate expiration
        let expires_at = request.expires_in_days.map(|days| {
            chrono::Utc::now() + chrono::Duration::days(i64::try_from(days).unwrap_or(365))
        });

        // Generate JWT token
        let jwt_token = jwt_manager.generate_token(
            &token_id,
            &request.service_name,
            &permissions,
            request.is_super_admin,
            expires_at,
        )?;

        // Generate token prefix and hash for storage
        let token_prefix = AdminJwtManager::generate_token_prefix(&jwt_token);
        let token_hash = AdminJwtManager::hash_token_for_storage(&jwt_token)?;
        let jwt_secret_hash = AdminJwtManager::hash_secret(&jwt_secret);

        // Store in database
        let query = r"
            INSERT INTO admin_tokens (
                id, service_name, service_description, token_hash, token_prefix,
                jwt_secret_hash, permissions, is_super_admin, is_active,
                created_at, expires_at, usage_count
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ";

        let permissions_json = permissions.to_json()?;
        let created_at = chrono::Utc::now();

        sqlx::query(query)
            .bind(&token_id)
            .bind(&request.service_name)
            .bind(&request.service_description)
            .bind(&token_hash)
            .bind(&token_prefix)
            .bind(&jwt_secret_hash)
            .bind(&permissions_json)
            .bind(request.is_super_admin)
            .bind(true) // is_active
            .bind(created_at)
            .bind(expires_at)
            .bind(0i64) // usage_count
            .execute(&self.pool)
            .await?;

        Ok(GeneratedAdminToken {
            token_id,
            service_name: request.service_name.clone(),
            jwt_token,
            token_prefix,
            permissions,
            is_super_admin: request.is_super_admin,
            expires_at,
            created_at,
        })
    }

    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        let query = r"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE id = $1
        ";

        let row = sqlx::query(query)
            .bind(token_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_admin_token(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        let query = r"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE token_prefix = $1
        ";

        let row = sqlx::query(query)
            .bind(token_prefix)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_admin_token(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        let query = if include_inactive {
            r"
                SELECT id, service_name, service_description, token_hash, token_prefix,
                       jwt_secret_hash, permissions, is_super_admin, is_active,
                       created_at, expires_at, last_used_at, last_used_ip, usage_count
                FROM admin_tokens ORDER BY created_at DESC
            "
        } else {
            r"
                SELECT id, service_name, service_description, token_hash, token_prefix,
                       jwt_secret_hash, permissions, is_super_admin, is_active,
                       created_at, expires_at, last_used_at, last_used_ip, usage_count
                FROM admin_tokens WHERE is_active = true ORDER BY created_at DESC
            "
        };

        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(Self::row_to_admin_token(&row)?);
        }

        Ok(tokens)
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        let query = "UPDATE admin_tokens SET is_active = false WHERE id = $1";

        sqlx::query(query)
            .bind(token_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        let query = r"
            UPDATE admin_tokens 
            SET last_used_at = CURRENT_TIMESTAMP, last_used_ip = $1, usage_count = usage_count + 1
            WHERE id = $2
        ";

        sqlx::query(query)
            .bind(ip_address)
            .bind(token_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()> {
        let query = r"
            INSERT INTO admin_token_usage (
                admin_token_id, timestamp, action, target_resource,
                ip_address, user_agent, request_size_bytes, success,
                method, response_time_ms
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ";

        sqlx::query(query)
            .bind(&usage.admin_token_id)
            .bind(usage.timestamp)
            .bind(usage.action.to_string())
            .bind(&usage.target_resource)
            .bind(&usage.ip_address)
            .bind(&usage.user_agent)
            .bind(
                usage
                    .request_size_bytes
                    .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
            )
            .bind(usage.success)
            .bind(None::<String>)
            .bind(
                usage
                    .response_time_ms
                    .map(|x| i32::try_from(x).unwrap_or(i32::MAX)),
            )
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>> {
        let query = r"
            SELECT id, admin_token_id, timestamp, action, target_resource,
                   ip_address, user_agent, request_size_bytes, success,
                   method, response_time_ms
            FROM admin_token_usage 
            WHERE admin_token_id = $1 AND timestamp BETWEEN $2 AND $3
            ORDER BY timestamp DESC
        ";

        let rows = sqlx::query(query)
            .bind(token_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await?;

        let mut usage_history = Vec::new();
        for row in rows {
            usage_history.push(Self::row_to_admin_token_usage(&row)?);
        }

        Ok(usage_history)
    }

    async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<()> {
        let query = r"
            INSERT INTO admin_provisioned_keys (
                admin_token_id, api_key_id, user_email, requested_tier,
                provisioned_at, provisioned_by_service, rate_limit_requests,
                rate_limit_period, key_status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ";

        // Get service name from admin token
        let service_name = if let Some(token) = self.get_admin_token_by_id(admin_token_id).await? {
            token.service_name
        } else {
            "unknown".into()
        };

        sqlx::query(query)
            .bind(admin_token_id)
            .bind(api_key_id)
            .bind(user_email)
            .bind(tier)
            .bind(chrono::Utc::now())
            .bind(service_name)
            .bind(i32::try_from(rate_limit_requests).unwrap_or(i32::MAX))
            .bind(rate_limit_period)
            .bind("active")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>> {
        // Simplified implementation using direct queries instead of complex dynamic binding
        if let Some(token_id) = admin_token_id {
            let rows = sqlx::query(
                r"
                    SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                           provisioned_at, provisioned_by_service, rate_limit_requests,
                           rate_limit_period, key_status, revoked_at, revoked_reason
                    FROM admin_provisioned_keys 
                    WHERE admin_token_id = $1 AND provisioned_at BETWEEN $2 AND $3
                    ORDER BY provisioned_at DESC
                ",
            )
            .bind(token_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await?;

            let mut results = Vec::new();
            for row in rows {
                let result = serde_json::json!({
                    "id": row.get::<i32, _>("id"),
                    "admin_token_id": row.get::<String, _>("admin_token_id"),
                    "api_key_id": row.get::<String, _>("api_key_id"),
                    "user_email": row.get::<String, _>("user_email"),
                    "requested_tier": row.get::<String, _>("requested_tier"),
                    "provisioned_at": row.get::<DateTime<Utc>, _>("provisioned_at"),
                    "provisioned_by_service": row.get::<String, _>("provisioned_by_service"),
                    "rate_limit_requests": row.get::<i32, _>("rate_limit_requests"),
                    "rate_limit_period": row.get::<String, _>("rate_limit_period"),
                    "key_status": row.get::<String, _>("key_status"),
                    "revoked_at": row.get::<Option<DateTime<Utc>>, _>("revoked_at"),
                    "revoked_reason": row.get::<Option<String>, _>("revoked_reason"),
                });
                results.push(result);
            }
            Ok(results)
        } else {
            let rows = sqlx::query(
                r"
                    SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                           provisioned_at, provisioned_by_service, rate_limit_requests,
                           rate_limit_period, key_status, revoked_at, revoked_reason
                    FROM admin_provisioned_keys 
                    WHERE provisioned_at BETWEEN $1 AND $2
                    ORDER BY provisioned_at DESC
                ",
            )
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await?;

            let mut results = Vec::new();
            for row in rows {
                let result = serde_json::json!({
                    "id": row.get::<i32, _>("id"),
                    "admin_token_id": row.get::<String, _>("admin_token_id"),
                    "api_key_id": row.get::<String, _>("api_key_id"),
                    "user_email": row.get::<String, _>("user_email"),
                    "requested_tier": row.get::<String, _>("requested_tier"),
                    "provisioned_at": row.get::<DateTime<Utc>, _>("provisioned_at"),
                    "provisioned_by_service": row.get::<String, _>("provisioned_by_service"),
                    "rate_limit_requests": row.get::<i32, _>("rate_limit_requests"),
                    "rate_limit_period": row.get::<String, _>("rate_limit_period"),
                    "key_status": row.get::<String, _>("key_status"),
                    "revoked_at": row.get::<Option<DateTime<Utc>>, _>("revoked_at"),
                    "revoked_reason": row.get::<Option<String>, _>("revoked_reason"),
                });
                results.push(result);
            }
            Ok(results)
        }
    }

    // ================================
    // Multi-Tenant Management
    // ================================

    /// Create a new tenant
    async fn create_tenant(&self, _tenant: &crate::models::Tenant) -> Result<()> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant management
        Ok(())
    }

    /// Get tenant by ID
    async fn get_tenant_by_id(&self, _tenant_id: Uuid) -> Result<crate::models::Tenant> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant management
        Err(anyhow::anyhow!(
            "PostgreSQL tenant management not yet implemented"
        ))
    }

    /// Get tenant by slug
    async fn get_tenant_by_slug(&self, _slug: &str) -> Result<crate::models::Tenant> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant management
        Err(anyhow::anyhow!(
            "PostgreSQL tenant management not yet implemented"
        ))
    }

    /// List tenants for a user
    async fn list_tenants_for_user(&self, _user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant management
        Ok(Vec::new())
    }

    /// Store tenant OAuth credentials
    async fn store_tenant_oauth_credentials(
        &self,
        _credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant OAuth management
        Ok(())
    }

    /// Get tenant OAuth providers
    async fn get_tenant_oauth_providers(
        &self,
        _tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant OAuth management
        Ok(Vec::new())
    }

    /// Get tenant OAuth credentials for specific provider
    async fn get_tenant_oauth_credentials(
        &self,
        _tenant_id: Uuid,
        _provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        // Stub implementation - TODO: implement proper PostgreSQL tenant OAuth management
        Ok(None)
    }

    // ================================
    // OAuth App Registration
    // ================================

    /// Create OAuth application
    async fn create_oauth_app(&self, _app: &crate::models::OAuthApp) -> Result<()> {
        // Stub implementation - TODO: implement proper PostgreSQL OAuth app management
        Ok(())
    }

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(
        &self,
        _client_id: &str,
    ) -> Result<crate::models::OAuthApp> {
        // Stub implementation - TODO: implement proper PostgreSQL OAuth app management
        Err(anyhow::anyhow!(
            "PostgreSQL OAuth app management not yet implemented"
        ))
    }

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(
        &self,
        _user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        // Stub implementation - TODO: implement proper PostgreSQL OAuth app management
        Ok(Vec::new())
    }

    /// Store authorization code
    async fn store_authorization_code(
        &self,
        _code: &str,
        _client_id: &str,
        _redirect_uri: &str,
        _scope: &str,
    ) -> Result<()> {
        // Stub implementation - TODO: implement proper PostgreSQL OAuth code management
        Ok(())
    }

    /// Get authorization code data
    async fn get_authorization_code(
        &self,
        _code: &str,
    ) -> Result<crate::models::AuthorizationCode> {
        // Stub implementation - TODO: implement proper PostgreSQL OAuth code management
        Err(anyhow::anyhow!(
            "PostgreSQL OAuth code management not yet implemented"
        ))
    }

    /// Delete authorization code
    async fn delete_authorization_code(&self, _code: &str) -> Result<()> {
        // Stub implementation - TODO: implement proper PostgreSQL OAuth code management
        Ok(())
    }
}

impl PostgresDatabase {
    /// Convert database row to `AdminToken`
    fn row_to_admin_token(row: &sqlx::postgres::PgRow) -> Result<crate::admin::models::AdminToken> {
        use crate::admin::models::{AdminPermissions, AdminToken};
        use sqlx::Row;

        let permissions_json: String = row.try_get("permissions")?;
        let permissions = AdminPermissions::from_json(&permissions_json)?;

        Ok(AdminToken {
            id: row.try_get("id")?,
            service_name: row.try_get("service_name")?,
            service_description: row.try_get("service_description")?,
            token_hash: row.try_get("token_hash")?,
            token_prefix: row.try_get("token_prefix")?,
            jwt_secret_hash: row.try_get("jwt_secret_hash")?,
            permissions,
            is_super_admin: row.try_get("is_super_admin")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
            last_used_at: row.try_get("last_used_at")?,
            last_used_ip: row.try_get("last_used_ip")?,
            usage_count: u64::try_from(row.try_get::<i64, _>("usage_count")?.max(0)).unwrap_or(0),
        })
    }

    /// Convert database row to `AdminTokenUsage`
    fn row_to_admin_token_usage(
        row: &sqlx::postgres::PgRow,
    ) -> Result<crate::admin::models::AdminTokenUsage> {
        use crate::admin::models::{AdminAction, AdminTokenUsage};
        use sqlx::Row;

        let action_str: String = row.try_get("action")?;
        let action = action_str
            .parse::<AdminAction>()
            .unwrap_or(AdminAction::ProvisionKey);

        Ok(AdminTokenUsage {
            id: Some(row.try_get::<i64, _>("id")?),
            admin_token_id: row.try_get("admin_token_id")?,
            timestamp: row.try_get("timestamp")?,
            action,
            target_resource: row.try_get("target_resource")?,
            ip_address: row.try_get("ip_address")?,
            user_agent: row.try_get("user_agent")?,
            request_size_bytes: row
                .try_get::<Option<i32>, _>("request_size_bytes")?
                .map(|v| u32::try_from(v.max(0)).unwrap_or(0)),
            success: row.try_get("success")?,
            error_message: None, // Add the missing field
            response_time_ms: row
                .try_get::<Option<i32>, _>("response_time_ms")?
                .map(|v| u32::try_from(v.max(0)).unwrap_or(0)),
        })
    }

    async fn create_users_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
                strava_access_token TEXT,
                strava_refresh_token TEXT,
                strava_expires_at TIMESTAMPTZ,
                strava_scope TEXT,
                strava_nonce TEXT,
                fitbit_access_token TEXT,
                fitbit_refresh_token TEXT,
                fitbit_expires_at TIMESTAMPTZ,
                fitbit_scope TEXT,
                fitbit_nonce TEXT,
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                last_active TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_user_profiles_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_profiles (
                user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                profile_data JSONB NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_goals_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS goals (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                goal_data JSONB NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_insights_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS insights (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                insight_type TEXT NOT NULL,
                content JSONB NOT NULL,
                metadata JSONB,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_api_keys_tables(&self) -> Result<()> {
        // Create api_keys table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                description TEXT,
                tier TEXT NOT NULL CHECK (tier IN ('trial', 'starter', 'professional', 'enterprise')),
                is_active BOOLEAN NOT NULL DEFAULT true,
                rate_limit_requests INTEGER NOT NULL,
                rate_limit_window_seconds INTEGER NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ,
                last_used_at TIMESTAMPTZ,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create api_key_usage table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS api_key_usage (
                id SERIAL PRIMARY KEY,
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code SMALLINT NOT NULL,
                method TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address INET,
                user_agent TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_a2a_tables(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_clients (
                client_id TEXT PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                description TEXT,
                client_secret_hash TEXT NOT NULL,
                api_key_hash TEXT NOT NULL,
                capabilities TEXT[] NOT NULL DEFAULT '{}',
                redirect_uris TEXT[] NOT NULL DEFAULT '{}',
                contact_email TEXT,
                is_active BOOLEAN NOT NULL DEFAULT true,
                rate_limit_per_minute INTEGER NOT NULL DEFAULT 100,
                rate_limit_per_day INTEGER DEFAULT 10000,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_sessions (
                session_token TEXT PRIMARY KEY,
                client_id TEXT NOT NULL REFERENCES a2a_clients(client_id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                granted_scopes TEXT[] NOT NULL DEFAULT '{}',
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ NOT NULL,
                last_active_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_tasks (
                task_id TEXT PRIMARY KEY,
                session_token TEXT NOT NULL REFERENCES a2a_sessions(session_token) ON DELETE CASCADE,
                task_type TEXT NOT NULL,
                parameters JSONB NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                result JSONB,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_usage (
                id SERIAL PRIMARY KEY,
                client_id TEXT NOT NULL REFERENCES a2a_clients(client_id) ON DELETE CASCADE,
                session_token TEXT REFERENCES a2a_sessions(session_token) ON DELETE SET NULL,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code SMALLINT NOT NULL,
                method TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address INET,
                user_agent TEXT,
                protocol_version TEXT NOT NULL DEFAULT 'v1',
                client_capabilities TEXT[] DEFAULT '{}',
                granted_scopes TEXT[] DEFAULT '{}'
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_admin_tables(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_tokens (
                id TEXT PRIMARY KEY,
                service_name TEXT NOT NULL,
                service_description TEXT,
                token_hash TEXT NOT NULL,
                token_prefix TEXT NOT NULL,
                jwt_secret_hash TEXT NOT NULL,
                permissions TEXT NOT NULL DEFAULT '[]',
                is_super_admin BOOLEAN NOT NULL DEFAULT false,
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ,
                last_used_at TIMESTAMPTZ,
                last_used_ip INET,
                usage_count BIGINT NOT NULL DEFAULT 0
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_token_usage (
                id SERIAL PRIMARY KEY,
                admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                action TEXT NOT NULL,
                target_resource TEXT,
                ip_address INET,
                user_agent TEXT,
                request_size_bytes INTEGER,
                success BOOLEAN NOT NULL,
                method TEXT,
                response_time_ms INTEGER
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_provisioned_keys (
                id SERIAL PRIMARY KEY,
                admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
                api_key_id TEXT NOT NULL,
                user_email TEXT NOT NULL,
                requested_tier TEXT NOT NULL,
                provisioned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                provisioned_by_service TEXT NOT NULL,
                rate_limit_requests INTEGER NOT NULL,
                rate_limit_period TEXT NOT NULL,
                key_status TEXT NOT NULL DEFAULT 'active',
                revoked_at TIMESTAMPTZ,
                revoked_reason TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_jwt_usage_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS jwt_usage (
                id SERIAL PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code INTEGER NOT NULL,
                method TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address INET,
                user_agent TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_tenant_tables(&self) -> Result<()> {
        // Create tenants table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenants (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name VARCHAR(255) NOT NULL,
                slug VARCHAR(100) UNIQUE NOT NULL,
                domain VARCHAR(255) UNIQUE,
                subscription_tier VARCHAR(50) DEFAULT 'starter' CHECK (subscription_tier IN ('starter', 'professional', 'enterprise')),
                is_active BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_oauth_apps table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_oauth_apps (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                provider VARCHAR(50) NOT NULL,
                client_id VARCHAR(255) NOT NULL,
                client_secret_encrypted BYTEA NOT NULL,
                client_secret_nonce BYTEA NOT NULL,
                redirect_uri VARCHAR(500) NOT NULL,
                scopes TEXT[] DEFAULT '{}',
                rate_limit_per_day INTEGER DEFAULT 15000,
                is_active BOOLEAN DEFAULT true,
                configured_by UUID REFERENCES users(id),
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_users table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role VARCHAR(50) DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'billing', 'member')),
                joined_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, user_id)
            )
            "
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_provider_usage table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_provider_usage (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                provider VARCHAR(50) NOT NULL,
                usage_date DATE NOT NULL,
                request_count INTEGER DEFAULT 0,
                error_count INTEGER DEFAULT 0,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, provider, usage_date)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_indexes(&self) -> Result<()> {
        // User and profile indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await?;

        // API key indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_api_key_id ON api_key_usage(api_key_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_timestamp ON api_key_usage(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        // A2A indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_clients_user_id ON a2a_clients(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_client_id ON a2a_usage(client_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_timestamp ON a2a_usage(timestamp)")
            .execute(&self.pool)
            .await?;

        // Admin token indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_service ON admin_tokens(service_name)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_prefix ON admin_tokens(token_prefix)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_usage_token_id ON admin_token_usage(admin_token_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_usage_timestamp ON admin_token_usage(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_provisioned_token ON admin_provisioned_keys(admin_token_id)")
            .execute(&self.pool)
            .await?;

        // JWT usage indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_user_id ON jwt_usage(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_timestamp ON jwt_usage(timestamp)")
            .execute(&self.pool)
            .await?;

        // Tenant indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_oauth_apps_tenant_provider ON tenant_oauth_apps(tenant_id, provider)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_usage_date ON tenant_provider_usage(tenant_id, provider, usage_date)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
