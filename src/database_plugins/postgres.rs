// ABOUTME: PostgreSQL database implementation for cloud and production deployments
// ABOUTME: Provides enterprise-grade database support with connection pooling and scalability
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! `PostgreSQL` database implementation
//!
//! This module provides `PostgreSQL` support for cloud deployments,
//! implementing the same interface as the `SQLite` version.

use super::DatabaseProvider;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::constants::oauth_providers;
use crate::constants::tiers;
use crate::database::A2AUsage;
use crate::models::{User, UserTier};
use crate::rate_limiting::JwtUsage;
use anyhow::Context;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::fmt::Write;
use std::time::Duration;
use uuid::Uuid;

/// `PostgreSQL` database implementation
#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Pool<Postgres>,
}

impl PostgresDatabase {
    /// Close the database connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

impl PostgresDatabase {
    /// Create new `PostgreSQL` database with provided pool configuration
    /// This is called by the Database factory with centralized `ServerConfig`
    ///
    /// # Errors
    ///
    /// Returns an error if database connection or pool configuration fails
    pub async fn new(
        database_url: &str,
        _encryption_key: Vec<u8>,
        pool_config: &crate::config::environment::PostgresPoolConfig,
    ) -> Result<Self> {
        // Use pool configuration from ServerConfig (read once at startup)
        let max_connections = pool_config.max_connections;
        let min_connections = pool_config.min_connections;
        let acquire_timeout_secs = pool_config.acquire_timeout_secs;

        // Log connection pool configuration for debugging
        tracing::info!(
            "PostgreSQL pool config: max_connections={max_connections}, min_connections={min_connections}, timeout={acquire_timeout_secs}s"
        );

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(600)))
            .connect(database_url)
            .await
            .with_context(|| {
                format!("Failed to connect to PostgreSQL with {max_connections} max connections")
            })?;

        let db = Self { pool };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }
}

#[async_trait]
impl DatabaseProvider for PostgresDatabase {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Use default pool configuration when called through trait
        // In practice, the Database factory calls the inherent impl's new() directly with config
        let pool_config = crate::config::environment::PostgresPoolConfig::default();
        Self::new(database_url, encryption_key, &pool_config).await
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
        self.create_oauth_notifications_table().await?;
        self.create_tenant_tables().await?; // Add tenant tables
        self.create_indexes().await?;
        Ok(())
    }

    async fn create_user(&self, user: &User) -> Result<Uuid> {
        sqlx::query(
            r"
            INSERT INTO users (id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin, user_status, approved_by, approved_at, created_at, last_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(match user.tier {
            UserTier::Starter => tiers::STARTER,
            UserTier::Professional => tiers::PROFESSIONAL,
            UserTier::Enterprise => tiers::ENTERPRISE,
        })
        .bind(&user.tenant_id)
        .bind(user.is_active)
        .bind(user.is_admin)
        .bind(match user.user_status {
            crate::models::UserStatus::Active => "active",
            crate::models::UserStatus::Pending => "pending",
            crate::models::UserStatus::Suspended => "suspended",
        })
        .bind(user.approved_by)
        .bind(user.approved_at)
        .bind(user.created_at)
        .bind(user.last_active)
        .execute(&self.pool)
        .await?;

        Ok(user.id)
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin, 
                   user_status, approved_by, approved_at, created_at, last_active
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
                            tiers::PROFESSIONAL => UserTier::Professional,
                            tiers::ENTERPRISE => UserTier::Enterprise,
                            _ => UserTier::Starter,
                        }
                    },
                    tenant_id: row.get("tenant_id"),
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    is_active: row.get("is_active"),
                    user_status: {
                        let status_str: String = row.get("user_status");
                        match status_str.as_str() {
                            "pending" => crate::models::UserStatus::Pending,
                            "suspended" => crate::models::UserStatus::Suspended,
                            _ => crate::models::UserStatus::Active,
                        }
                    },
                    is_admin: row.get("is_admin"),
                    approved_by: row.get("approved_by"),
                    approved_at: row.get("approved_at"),
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                }))
            },
        )
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin, 
                   user_status, approved_by, approved_at, created_at, last_active
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
                            tiers::PROFESSIONAL => UserTier::Professional,
                            tiers::ENTERPRISE => UserTier::Enterprise,
                            _ => UserTier::Starter,
                        }
                    },
                    tenant_id: row.get("tenant_id"),
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    is_active: row.get("is_active"),
                    user_status: {
                        let status_str: String = row.get("user_status");
                        match status_str.as_str() {
                            "pending" => crate::models::UserStatus::Pending,
                            "suspended" => crate::models::UserStatus::Suspended,
                            _ => crate::models::UserStatus::Active,
                        }
                    },
                    is_admin: row.get("is_admin"),
                    approved_by: row.get("approved_by"),
                    approved_at: row.get("approved_at"),
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                }))
            },
        )
    }

    async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        self.get_user_by_email(email)
            .await?
            .ok_or_else(|| anyhow!("User with email {email} not found"))
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

    async fn get_users_by_status(&self, status: &str) -> Result<Vec<User>> {
        // Query users by status from PostgreSQL
        let status_enum = match status {
            "active" => "active",
            "pending" => "pending",
            "suspended" => "suspended",
            _ => return Err(anyhow!("Invalid user status: {status}")),
        };

        let rows = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   COALESCE(user_status, 'active') as user_status, approved_by, approved_at, created_at, last_active
            FROM users
            WHERE COALESCE(user_status, 'active') = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(status_enum)
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_status_str: String = row.get("user_status");
            let user_status = match user_status_str.as_str() {
                "pending" => crate::models::UserStatus::Pending,
                "suspended" => crate::models::UserStatus::Suspended,
                _ => crate::models::UserStatus::Active,
            };

            users.push(User {
                id: row.get("id"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                password_hash: row.get("password_hash"),
                tier: {
                    let tier_str: String = row.get("tier");
                    match tier_str.as_str() {
                        tiers::PROFESSIONAL => UserTier::Professional,
                        tiers::ENTERPRISE => UserTier::Enterprise,
                        _ => UserTier::Starter,
                    }
                },
                tenant_id: row.get("tenant_id"),
                strava_token: None,
                fitbit_token: None,
                is_active: row.get("is_active"),
                user_status,
                is_admin: row.try_get("is_admin").unwrap_or(false), // Default to false for existing users
                approved_by: row.get("approved_by"),
                approved_at: row.get("approved_at"),
                created_at: row.get("created_at"),
                last_active: row.get("last_active"),
            });
        }

        Ok(users)
    }

    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<User>> {
        use crate::pagination::{Cursor, CursorPage};

        // Validate status
        let status_enum = match status {
            "active" => "active",
            "pending" => "pending",
            "suspended" => "suspended",
            _ => return Err(anyhow!("Invalid user status: {status}")),
        };

        // Fetch one more than requested to determine if there are more items
        let fetch_limit = params.limit + 1;

        // Convert to i64 for SQL LIMIT clause (pagination limits are always reasonable)
        let fetch_limit_i64 =
            i64::try_from(fetch_limit).map_err(|_| anyhow!("Pagination limit too large"))?;

        let (query, cursor_timestamp, cursor_id) = if let Some(ref cursor) = params.cursor {
            let (timestamp, id) = cursor
                .decode()
                .ok_or_else(|| anyhow!("Invalid cursor format"))?;

            let query = r"
                SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                       COALESCE(user_status, 'active') as user_status, approved_by, approved_at, created_at, last_active
                FROM users
                WHERE COALESCE(user_status, 'active') = $1
                  AND (created_at < $2 OR (created_at = $2 AND id::text < $3))
                ORDER BY created_at DESC, id DESC
                LIMIT $4
            ";
            (query, Some(timestamp), Some(id))
        } else {
            let query = r"
                SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                       COALESCE(user_status, 'active') as user_status, approved_by, approved_at, created_at, last_active
                FROM users
                WHERE COALESCE(user_status, 'active') = $1
                ORDER BY created_at DESC, id DESC
                LIMIT $2
            ";
            (query, None, None)
        };

        // Execute query with appropriate parameters
        let rows = if let (Some(ts), Some(id)) = (cursor_timestamp, cursor_id) {
            sqlx::query(query)
                .bind(status_enum)
                .bind(ts)
                .bind(id)
                .bind(fetch_limit_i64)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query)
                .bind(status_enum)
                .bind(fetch_limit_i64)
                .fetch_all(&self.pool)
                .await?
        };

        // Parse rows into User structs
        let mut users = Vec::new();
        for row in rows {
            let user_status_str: String = row.get("user_status");
            let user_status = match user_status_str.as_str() {
                "pending" => crate::models::UserStatus::Pending,
                "suspended" => crate::models::UserStatus::Suspended,
                _ => crate::models::UserStatus::Active,
            };

            users.push(User {
                id: row.get("id"),
                email: row.get("email"),
                display_name: row.get("display_name"),
                password_hash: row.get("password_hash"),
                tier: {
                    let tier_str: String = row.get("tier");
                    match tier_str.as_str() {
                        tiers::PROFESSIONAL => UserTier::Professional,
                        tiers::ENTERPRISE => UserTier::Enterprise,
                        _ => UserTier::Starter,
                    }
                },
                tenant_id: row.get("tenant_id"),
                strava_token: None,
                fitbit_token: None,
                is_active: row.get("is_active"),
                user_status,
                is_admin: row.try_get("is_admin").unwrap_or(false),
                approved_by: row.get("approved_by"),
                approved_at: row.get("approved_at"),
                created_at: row.get("created_at"),
                last_active: row.get("last_active"),
            });
        }

        // Determine if there are more items
        let has_more = users.len() > params.limit;
        if has_more {
            users.pop(); // Remove the extra item we fetched
        }

        // Generate next cursor from the last item
        let next_cursor = if has_more && !users.is_empty() {
            let last_user = users.last().expect("Users should not be empty");
            Some(Cursor::new(last_user.created_at, &last_user.id.to_string()))
        } else {
            None
        };

        Ok(CursorPage::new(users, next_cursor, None, has_more))
    }

    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<User> {
        let status_str = match new_status {
            crate::models::UserStatus::Active => "active",
            crate::models::UserStatus::Pending => "pending",
            crate::models::UserStatus::Suspended => "suspended",
        };

        let admin_uuid =
            if new_status == crate::models::UserStatus::Active && !admin_token_id.is_empty() {
                Some(admin_token_id)
            } else {
                None
            };

        let approved_at = if new_status == crate::models::UserStatus::Active {
            Some(chrono::Utc::now())
        } else {
            None
        };

        // First ensure the user_status column exists, create if needed
        let _ = sqlx::query(
            "ALTER TABLE users ADD COLUMN IF NOT EXISTS user_status TEXT DEFAULT 'active'",
        )
        .execute(&self.pool)
        .await;

        let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS approved_by TEXT")
            .execute(&self.pool)
            .await;

        let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ")
            .execute(&self.pool)
            .await;

        // Update user status
        sqlx::query(
            r"
            UPDATE users 
            SET user_status = $1, approved_by = $2, approved_at = $3
            WHERE id = $4
            ",
        )
        .bind(status_str)
        .bind(admin_uuid)
        .bind(approved_at)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // Return updated user
        self.get_user(user_id)
            .await?
            .ok_or_else(|| anyhow!("User not found after status update"))
    }

    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> Result<()> {
        let result = sqlx::query(
            r"
            UPDATE users 
            SET tenant_id = $1
            WHERE id = $2
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("No user found with ID: {user_id}"));
        }

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
                        tiers::TRIAL | tiers::STARTER => crate::api_keys::ApiKeyTier::Starter,
                        tiers::PROFESSIONAL => crate::api_keys::ApiKeyTier::Professional,
                        tiers::ENTERPRISE => crate::api_keys::ApiKeyTier::Enterprise,
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
                    tiers::TRIAL | tiers::STARTER => crate::api_keys::ApiKeyTier::Starter,
                    tiers::PROFESSIONAL => crate::api_keys::ApiKeyTier::Professional,
                    tiers::ENTERPRISE => crate::api_keys::ApiKeyTier::Enterprise,
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
                    tiers::STARTER => crate::api_keys::ApiKeyTier::Starter,
                    tiers::PROFESSIONAL => crate::api_keys::ApiKeyTier::Professional,
                    tiers::ENTERPRISE => crate::api_keys::ApiKeyTier::Enterprise,
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
                .map_err(|e| anyhow::anyhow!("Failed to write LIMIT clause: {e}"))?;
            if let Some(_offset) = offset {
                param_count += 1;
                write!(&mut query, " OFFSET ${param_count}")
                    .map_err(|e| anyhow::anyhow!("Failed to write OFFSET clause: {e}"))?;
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
                tiers::STARTER => crate::api_keys::ApiKeyTier::Starter,
                tiers::PROFESSIONAL => crate::api_keys::ApiKeyTier::Professional,
                tiers::ENTERPRISE => crate::api_keys::ApiKeyTier::Enterprise,
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
                    tiers::TRIAL | tiers::STARTER => crate::api_keys::ApiKeyTier::Starter,
                    tiers::PROFESSIONAL => crate::api_keys::ApiKeyTier::Professional,
                    tiers::ENTERPRISE => crate::api_keys::ApiKeyTier::Enterprise,
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10)
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
        let row = sqlx::query_as::<Postgres, (i64, i64, i64, Option<i64>, Option<i64>, Option<i64>)>(
            r"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code >= $1 AND status_code <= $2 THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status_code >= $3 THEN 1 END) as failed_requests,
                SUM(response_time_ms) as total_response_time,
                SUM(request_size_bytes) as total_request_size,
                SUM(response_size_bytes) as total_response_size
            FROM api_key_usage 
            WHERE api_key_id = $4 AND timestamp >= $5 AND timestamp <= $6
            "
        )
        .bind(i32::from(crate::constants::http_status::SUCCESS_MIN))
        .bind(i32::from(crate::constants::http_status::SUCCESS_MAX))
        .bind(i32::from(crate::constants::http_status::BAD_REQUEST))
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        // Get tool usage aggregation
        let tool_usage_stats = sqlx::query_as::<Postgres, (String, i64, Option<f64>, i64)>(
            r"
            SELECT tool_name, 
                   COUNT(*) as tool_count,
                   AVG(response_time_ms) as avg_response_time,
                   COUNT(CASE WHEN status_code >= $1 AND status_code <= $2 THEN 1 END) as success_count
            FROM api_key_usage
            WHERE api_key_id = $3 AND timestamp >= $4 AND timestamp <= $5
            GROUP BY tool_name
            ORDER BY tool_count DESC
            "
        )
        .bind(i32::from(crate::constants::http_status::SUCCESS_MIN))
        .bind(i32::from(crate::constants::http_status::SUCCESS_MAX))
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        let mut tool_usage = serde_json::Map::new();
        for (tool_name, tool_count, avg_response_time, success_count) in tool_usage_stats {
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
            total_requests: u32::try_from(row.0.max(0)).unwrap_or(0),
            successful_requests: u32::try_from(row.1.max(0)).unwrap_or(0),
            failed_requests: u32::try_from(row.2.max(0)).unwrap_or(0),
            total_response_time_ms: row.3.map_or(0u64, |v| u64::try_from(v.max(0)).unwrap_or(0)),
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10)
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

        Ok(client.id.clone()) // Safe: String ownership for return value
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
                    user_id: row.get("user_id"),
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

    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r"
            SELECT c.client_id, c.user_id, c.name, c.description, c.client_secret_hash, c.capabilities,
                   c.redirect_uris, c.contact_email, c.is_active, c.rate_limit_per_minute,
                   c.rate_limit_per_day, c.created_at, c.updated_at
            FROM a2a_clients c
            INNER JOIN a2a_client_api_keys k ON c.client_id = k.client_id
            WHERE k.api_key_id = $1 AND c.is_active = true
            ",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(A2AClient {
                    id: row.get("client_id"),
                    user_id: row.get("user_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    public_key: row.get("client_secret_hash"),
                    capabilities: row.get("capabilities"),
                    redirect_uris: row.get("redirect_uris"),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                    permissions: vec!["read_activities".into()],
                    rate_limit_requests: u32::try_from(
                        row.get::<i32, _>("rate_limit_per_minute").max(0),
                    )
                    .unwrap_or(0),
                    rate_limit_window_seconds: 60,
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
                    user_id: row.get("user_id"),
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
                user_id: *user_id, // Use the provided user_id parameter
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
            return Err(anyhow::anyhow!("A2A client not found: {client_id}"));
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
                result: result_data.clone(), // Safe: JSON value ownership for A2ATask struct
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

    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        use std::fmt::Write;
        let mut query = String::from(
            r"
            SELECT task_id, client_id, session_id, task_type, input_data,
                   status, result_data, method, created_at, updated_at
            FROM a2a_tasks
            ",
        );

        let mut conditions = Vec::new();
        let mut bind_count = 0;

        if client_id.is_some() {
            bind_count += 1;
            conditions.push(format!("client_id = ${bind_count}"));
        }

        if status_filter.is_some() {
            bind_count += 1;
            conditions.push(format!("status = ${bind_count}"));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(_limit_val) = limit {
            bind_count += 1;
            if write!(query, " LIMIT ${bind_count}").is_err() {
                return Err(anyhow::anyhow!("Failed to write LIMIT clause to query"));
            }
        }

        if let Some(_offset_val) = offset {
            bind_count += 1;
            if write!(query, " OFFSET ${bind_count}").is_err() {
                return Err(anyhow::anyhow!("Failed to write OFFSET clause to query"));
            }
        }

        let mut sql_query = sqlx::query(&query);

        if let Some(client_id_val) = client_id {
            sql_query = sql_query.bind(client_id_val);
        }

        if let Some(status_val) = status_filter {
            let status_str = match status_val {
                TaskStatus::Pending => "pending",
                TaskStatus::Running => "running",
                TaskStatus::Completed => "completed",
                TaskStatus::Failed => "failed",
                TaskStatus::Cancelled => "cancelled",
            };
            sql_query = sql_query.bind(status_str);
        }

        if let Some(limit_val) = limit {
            sql_query = sql_query.bind(i64::from(limit_val));
        }

        if let Some(offset_val) = offset {
            sql_query = sql_query.bind(i64::from(offset_val));
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut tasks = Vec::new();
        for row in rows {
            use sqlx::Row;
            let input_str: String = row.try_get("input_data")?;
            let input_data: Value = serde_json::from_str(&input_str).unwrap_or(Value::Null);

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
                _ => TaskStatus::Pending,
            };

            tasks.push(A2ATask {
                id: row.try_get("task_id")?,
                status,
                created_at: row.try_get("created_at")?,
                completed_at: row.try_get("updated_at")?,
                result: result_data.clone(), // Safe: JSON value ownership for A2ATask struct
                error: row.try_get("method")?,
                client_id: row
                    .try_get("client_id")
                    .unwrap_or_else(|_| "unknown".into()),
                task_type: row.try_get("task_type")?,
                input_data,
                output_data: result_data,
                error_message: row.try_get("method")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(tasks)
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::inet, $11, $12)
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
            oauth_providers::STRAVA => "strava_last_sync",
            oauth_providers::FITBIT => "fitbit_last_sync",
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
            oauth_providers::STRAVA => "strava_last_sync",
            oauth_providers::FITBIT => "fitbit_last_sync",
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
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        use crate::admin::{
            jwt::AdminJwtManager,
            models::{AdminPermissions, GeneratedAdminToken},
        };
        use uuid::Uuid;

        // Generate unique token ID
        let uuid = Uuid::new_v4().simple();
        let token_id = format!("admin_{uuid}");

        // Debug: Log token creation without exposing secrets
        tracing::debug!("Creating admin token with RS256 asymmetric signing");

        // Create JWT manager for RS256 token operations (no HS256 secret needed)
        let jwt_manager = AdminJwtManager::new();

        // Get permissions
        let permissions = request.permissions.as_ref().map_or_else(
            || {
                if request.is_super_admin {
                    AdminPermissions::super_admin()
                } else {
                    AdminPermissions::default_admin()
                }
            },
            |perms| AdminPermissions::new(perms.clone()), // Safe: Vec<String> ownership for permissions struct
        );

        // Calculate expiration
        let expires_at = request.expires_in_days.map(|days| {
            chrono::Utc::now() + chrono::Duration::days(i64::try_from(days).unwrap_or(365))
        });

        // Generate JWT token using RS256 (asymmetric signing)
        let jwt_token = jwt_manager.generate_token(
            &token_id,
            &request.service_name,
            &permissions,
            request.is_super_admin,
            expires_at,
            jwks_manager,
        )?;

        // Generate token prefix and hash for storage
        let token_prefix = AdminJwtManager::generate_token_prefix(&jwt_token);
        let token_hash = AdminJwtManager::hash_token_for_storage(&jwt_token)?;
        let jwt_secret_hash = AdminJwtManager::hash_secret(admin_jwt_secret);

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
            service_name: request.service_name.clone(), // Safe: String ownership for GeneratedAdminToken struct
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
            ) VALUES ($1, $2, $3, $4, $5::inet, $6, $7, $8, $9, $10)
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
    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO tenants (id, name, slug, domain, subscription_tier, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, true, $6, $7)
            ",
        )
        .bind(tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&tenant.domain)
        .bind(&tenant.plan)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create tenant: {e}"))?;

        // Add the owner as an admin of the tenant
        sqlx::query(
            r"
            INSERT INTO tenant_users (tenant_id, user_id, role, joined_at)
            VALUES ($1, $2, 'owner', CURRENT_TIMESTAMP)
            ",
        )
        .bind(tenant.id)
        .bind(tenant.owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to add owner to tenant: {e}"))?;

        tracing::info!(
            "Created tenant: {} ({}) and added owner to tenant_users",
            tenant.name,
            tenant.id
        );
        Ok(())
    }

    /// Get tenant by ID
    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, Uuid, DateTime<Utc>, DateTime<Utc>)>(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.subscription_tier, tu.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id AND tu.role = 'owner'
            WHERE t.id = $1 AND t.is_active = true
            ",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((id, name, slug, domain, plan, owner_user_id, created_at, updated_at)) => {
                Ok(crate::models::Tenant {
                    id,
                    name,
                    slug,
                    domain,
                    plan,
                    owner_user_id,
                    created_at,
                    updated_at,
                })
            }
            None => Err(anyhow::anyhow!("Tenant not found: {}", tenant_id)),
        }
    }

    /// Get tenant by slug
    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, Uuid, DateTime<Utc>, DateTime<Utc>)>(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.subscription_tier, tu.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id AND tu.role = 'owner'
            WHERE t.slug = $1 AND t.is_active = true
            ",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((id, name, slug, domain, plan, owner_user_id, created_at, updated_at)) => {
                Ok(crate::models::Tenant {
                    id,
                    name,
                    slug,
                    domain,
                    plan,
                    owner_user_id,
                    created_at,
                    updated_at,
                })
            }
            None => Err(anyhow::anyhow!("Tenant not found with slug: {}", slug)),
        }
    }

    /// List tenants for a user
    async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                String,
                Uuid,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r"
            SELECT DISTINCT t.id, t.name, t.slug, t.domain, t.subscription_tier, 
                   owner.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id
            JOIN tenant_users owner ON t.id = owner.tenant_id AND owner.role = 'owner'
            WHERE tu.user_id = $1 AND t.is_active = true
            ORDER BY t.created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let tenants = rows
            .into_iter()
            .map(
                |(id, name, slug, domain, plan, owner_user_id, created_at, updated_at)| {
                    crate::models::Tenant {
                        id,
                        name,
                        slug,
                        domain,
                        plan,
                        owner_user_id,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(tenants)
    }

    /// Store tenant OAuth credentials
    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        // Encrypt the client secret using proper AES-256-GCM
        let encryption_manager = crate::security::TenantEncryptionManager::new(
            // Use a deterministic key derived from tenant ID for consistency
            ring::digest::digest(
                &ring::digest::SHA256,
                format!("oauth_secret_key_{}", credentials.tenant_id).as_bytes(),
            )
            .as_ref()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to create encryption key"))?,
        );

        let encrypted_data = encryption_manager
            .encrypt_tenant_data(credentials.tenant_id, &credentials.client_secret)
            .map_err(|e| anyhow::anyhow!("Failed to encrypt OAuth secret: {}", e))?;

        let encrypted_secret = encrypted_data.data.as_bytes().to_vec();
        let nonce = encrypted_data.metadata.key_version.to_le_bytes().to_vec();

        // Convert scopes Vec<String> to PostgreSQL array format
        let scopes_array: Vec<&str> = credentials
            .scopes
            .iter()
            .map(std::string::String::as_str)
            .collect();

        sqlx::query(
            r"
            INSERT INTO tenant_oauth_apps 
                (tenant_id, provider, client_id, client_secret_encrypted, client_secret_nonce, 
                 redirect_uri, scopes, rate_limit_per_day, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true)
            ON CONFLICT (tenant_id, provider) 
            DO UPDATE SET 
                client_id = EXCLUDED.client_id,
                client_secret_encrypted = EXCLUDED.client_secret_encrypted,
                client_secret_nonce = EXCLUDED.client_secret_nonce,
                redirect_uri = EXCLUDED.redirect_uri,
                scope = EXCLUDED.scope,
                rate_limit_per_day = EXCLUDED.rate_limit_per_day,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(credentials.tenant_id)
        .bind(&credentials.provider)
        .bind(&credentials.client_id)
        .bind(&encrypted_secret)
        .bind(&nonce)
        .bind(&credentials.redirect_uri)
        .bind(&scopes_array)
        .bind(i32::try_from(credentials.rate_limit_per_day).unwrap_or(i32::MAX))
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store OAuth credentials: {}", e))?;

        Ok(())
    }

    /// Get tenant OAuth providers
    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        let rows =
            sqlx::query_as::<_, (String, String, Vec<u8>, Vec<u8>, String, Vec<String>, i32)>(
                r"
            SELECT provider, client_id, client_secret_encrypted, client_secret_nonce, 
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_apps
            WHERE tenant_id = $1 AND is_active = true
            ORDER BY provider
            ",
            )
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await?;

        let credentials = rows
            .into_iter()
            .map(
                |(
                    provider,
                    client_id,
                    encrypted_secret,
                    nonce,
                    redirect_uri,
                    scopes,
                    rate_limit,
                )| {
                    // Decrypt the client secret using proper AES-256-GCM
                    let encryption_manager = crate::security::TenantEncryptionManager::new(
                        ring::digest::digest(
                            &ring::digest::SHA256,
                            format!("oauth_secret_key_{tenant_id}").as_bytes(),
                        )
                        .as_ref()
                        .try_into()
                        .unwrap_or([0u8; 32]),
                    );

                    let encrypted_data = crate::security::EncryptedData {
                        data: String::from_utf8_lossy(&encrypted_secret).to_string(),
                        metadata: crate::security::EncryptionMetadata {
                            key_version: u32::from_le_bytes(
                                nonce.as_slice().try_into().unwrap_or([1, 0, 0, 0]),
                            ),
                            tenant_id: Some(tenant_id),
                            algorithm: "AES-256-GCM".to_string(),
                            encrypted_at: chrono::Utc::now(),
                        },
                    };

                    let client_secret = encryption_manager
                        .decrypt_tenant_data(tenant_id, &encrypted_data)
                        .unwrap_or_else(|_| "DECRYPTION_FAILED".to_string());

                    crate::tenant::TenantOAuthCredentials {
                        tenant_id,
                        provider,
                        client_id,
                        client_secret,
                        redirect_uri,
                        scopes,
                        rate_limit_per_day: u32::try_from(rate_limit).unwrap_or(0),
                    }
                },
            )
            .collect();

        Ok(credentials)
    }

    /// Get tenant OAuth credentials for specific provider
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        let row = sqlx::query_as::<_, (String, Vec<u8>, Vec<u8>, String, Vec<String>, i32)>(
            r"
            SELECT client_id, client_secret_encrypted, client_secret_nonce, 
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_apps
            WHERE tenant_id = $1 AND provider = $2 AND is_active = true
            ",
        )
        .bind(tenant_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((client_id, encrypted_secret, nonce, redirect_uri, scopes, rate_limit)) => {
                // Decrypt the client secret using proper AES-256-GCM
                let encryption_manager = crate::security::TenantEncryptionManager::new(
                    ring::digest::digest(
                        &ring::digest::SHA256,
                        format!("oauth_secret_key_{tenant_id}").as_bytes(),
                    )
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Failed to create encryption key"))?,
                );

                let encrypted_data = crate::security::EncryptedData {
                    data: String::from_utf8_lossy(&encrypted_secret).to_string(),
                    metadata: crate::security::EncryptionMetadata {
                        key_version: u32::from_le_bytes(
                            nonce.as_slice().try_into().unwrap_or([1, 0, 0, 0]),
                        ),
                        tenant_id: Some(tenant_id),
                        algorithm: "AES-256-GCM".to_string(),
                        encrypted_at: chrono::Utc::now(),
                    },
                };

                let client_secret = encryption_manager
                    .decrypt_tenant_data(tenant_id, &encrypted_data)
                    .map_err(|e| anyhow::anyhow!("Failed to decrypt OAuth secret: {}", e))?;

                Ok(Some(crate::tenant::TenantOAuthCredentials {
                    tenant_id,
                    provider: provider.to_string(),
                    client_id,
                    client_secret,
                    redirect_uri,
                    scopes,
                    rate_limit_per_day: u32::try_from(rate_limit).unwrap_or(0),
                }))
            }
            None => Ok(None),
        }
    }

    // ================================
    // OAuth App Registration
    // ================================

    /// Create OAuth application
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        let redirect_uris: Vec<&str> = app
            .redirect_uris
            .iter()
            .map(std::string::String::as_str)
            .collect();
        let scopes: Vec<&str> = app.scopes.iter().map(std::string::String::as_str).collect();

        sqlx::query(
            r"
            INSERT INTO oauth_apps 
                (id, client_id, client_secret, name, description, redirect_uris, 
                 scopes, app_type, owner_user_id, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true, $10, $11)
            ",
        )
        .bind(app.id)
        .bind(&app.client_id)
        .bind(&app.client_secret)
        .bind(&app.name)
        .bind(&app.description)
        .bind(&redirect_uris)
        .bind(&scopes)
        .bind(&app.app_type)
        .bind(app.owner_user_id)
        .bind(app.created_at)
        .bind(app.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create OAuth app: {}", e))?;

        Ok(())
    }

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                Option<String>,
                Vec<String>,
                Vec<String>,
                String,
                Uuid,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r"
            SELECT id, client_id, client_secret, name, description, redirect_uris, 
                   scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps
            WHERE client_id = $1 AND is_active = true
            ",
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((
                id,
                client_id,
                client_secret,
                name,
                description,
                redirect_uris,
                scopes,
                app_type,
                owner_user_id,
                created_at,
                updated_at,
            )) => Ok(crate::models::OAuthApp {
                id,
                client_id,
                client_secret,
                name,
                description,
                redirect_uris,
                scopes,
                app_type,
                owner_user_id,
                created_at,
                updated_at,
            }),
            None => Err(anyhow::anyhow!("OAuth app not found: {}", client_id)),
        }
    }

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                Option<String>,
                Vec<String>,
                Vec<String>,
                String,
                Uuid,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r"
            SELECT id, client_id, client_secret, name, description, redirect_uris, 
                   scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps
            WHERE owner_user_id = $1 AND is_active = true
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let apps = rows
            .into_iter()
            .map(
                |(
                    id,
                    client_id,
                    client_secret,
                    name,
                    description,
                    redirect_uris,
                    scopes,
                    app_type,
                    owner_user_id,
                    created_at,
                    updated_at,
                )| {
                    crate::models::OAuthApp {
                        id,
                        client_id,
                        client_secret,
                        name,
                        description,
                        redirect_uris,
                        scopes,
                        app_type,
                        owner_user_id,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(apps)
    }

    /// Store authorization code
    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> Result<()> {
        // Use the provided user_id from auth context
        let expires_at = Utc::now() + chrono::Duration::minutes(10); // OAuth codes expire in 10 minutes

        sqlx::query(
            r"
            INSERT INTO authorization_codes 
                (code, client_id, user_id, redirect_uri, scope, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, $6)
            ",
        )
        .bind(code)
        .bind(client_id)
        .bind(user_id)
        .bind(redirect_uri)
        .bind(scope)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store authorization code: {}", e))?;

        Ok(())
    }

    /// Get authorization code data
    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                Uuid,
                String,
                String,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r"
            SELECT code, client_id, user_id, redirect_uri, scope, created_at, expires_at
            FROM authorization_codes
            WHERE code = $1 AND expires_at > CURRENT_TIMESTAMP
            ",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((code, client_id, user_id, redirect_uri, scope, created_at, expires_at)) => {
                Ok(crate::models::AuthorizationCode {
                    code,
                    client_id,
                    redirect_uri,
                    scope,
                    user_id: Some(user_id),
                    expires_at,
                    created_at,
                    is_used: false, // Will be marked as used when deleted
                })
            }
            None => Err(anyhow::anyhow!(
                "Authorization code not found or expired: {}",
                code
            )),
        }
    }

    /// Delete authorization code
    async fn delete_authorization_code(&self, code: &str) -> Result<()> {
        let result = sqlx::query(
            r"
            DELETE FROM authorization_codes
            WHERE code = $1
            ",
        )
        .bind(code)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to delete authorization code: {}", e))?;

        if result.rows_affected() == 0 {
            tracing::warn!("Authorization code not found for deletion: {}", code);
        }

        Ok(())
    }

    // ================================
    // Key Rotation & Security - PostgreSQL implementations
    // ================================

    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        let query = r"
            INSERT INTO key_versions (tenant_id, version, created_at, expires_at, is_active, algorithm)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (tenant_id, version) DO UPDATE SET
                expires_at = EXCLUDED.expires_at,
                is_active = EXCLUDED.is_active,
                algorithm = EXCLUDED.algorithm
        ";

        sqlx::query(query)
            .bind(version.tenant_id.map(|id| id.to_string()))
            .bind(i32::try_from(version.version).unwrap_or(0)) // Safe: version ranges are controlled by application
            .bind(version.created_at)
            .bind(version.expires_at)
            .bind(version.is_active)
            .bind(&version.algorithm)
            .execute(&self.pool)
            .await
            .context("Failed to store key version")?;

        tracing::debug!(
            "Stored key version {} for tenant {:?}",
            version.version,
            version.tenant_id
        );
        Ok(())
    }

    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        let query = match tenant_id {
            Some(_) => {
                r"
                SELECT tenant_id, version, created_at, expires_at, is_active, algorithm
                FROM key_versions 
                WHERE tenant_id = $1
                ORDER BY version DESC
            "
            }
            None => {
                r"
                SELECT tenant_id, version, created_at, expires_at, is_active, algorithm
                FROM key_versions 
                WHERE tenant_id IS NULL
                ORDER BY version DESC
            "
            }
        };

        let rows = if let Some(tid) = tenant_id {
            sqlx::query(query)
                .bind(tid.to_string())
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query(query).fetch_all(&self.pool).await
        }
        .context("Failed to fetch key versions")?;

        let mut versions = Vec::new();
        for row in rows {
            let tenant_id_str: Option<String> = row.get("tenant_id");
            let tenant_id = if let Some(tid) = tenant_id_str {
                Some(crate::utils::uuid::parse_uuid(&tid)?)
            } else {
                None
            };

            let version = crate::security::key_rotation::KeyVersion {
                tenant_id,
                version: u32::try_from(row.get::<i32, _>("version")).unwrap_or(0), // Safe: stored versions are always positive
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                is_active: row.get("is_active"),
                algorithm: row.get("algorithm"),
            };
            versions.push(version);
        }

        Ok(versions)
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        let query = match tenant_id {
            Some(_) => {
                r"
                SELECT tenant_id, version, created_at, expires_at, is_active, algorithm
                FROM key_versions 
                WHERE tenant_id = $1 AND is_active = true
                ORDER BY version DESC
                LIMIT 1
            "
            }
            None => {
                r"
                SELECT tenant_id, version, created_at, expires_at, is_active, algorithm
                FROM key_versions 
                WHERE tenant_id IS NULL AND is_active = true
                ORDER BY version DESC
                LIMIT 1
            "
            }
        };

        let row = if let Some(tid) = tenant_id {
            sqlx::query(query)
                .bind(tid.to_string())
                .fetch_optional(&self.pool)
                .await
        } else {
            sqlx::query(query).fetch_optional(&self.pool).await
        }
        .context("Failed to fetch current key version")?;

        if let Some(row) = row {
            let tenant_id_str: Option<String> = row.get("tenant_id");
            let tenant_id = if let Some(tid) = tenant_id_str {
                Some(crate::utils::uuid::parse_uuid(&tid)?)
            } else {
                None
            };

            let version = crate::security::key_rotation::KeyVersion {
                tenant_id,
                version: u32::try_from(row.get::<i32, _>("version")).unwrap_or(0), // Safe: stored versions are always positive
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                is_active: row.get("is_active"),
                algorithm: row.get("algorithm"),
            };
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()> {
        let query = match tenant_id {
            Some(_) => {
                r"
                UPDATE key_versions 
                SET is_active = $3
                WHERE tenant_id = $1 AND version = $2
            "
            }
            None => {
                r"
                UPDATE key_versions 
                SET is_active = $2
                WHERE tenant_id IS NULL AND version = $1
            "
            }
        };

        let result = if let Some(tid) = tenant_id {
            sqlx::query(query)
                .bind(tid.to_string())
                .bind(i32::try_from(version).unwrap_or(0)) // Safe: version ranges are controlled by application
                .bind(is_active)
                .execute(&self.pool)
                .await
        } else {
            sqlx::query(query)
                .bind(i32::try_from(version).unwrap_or(0)) // Safe: version ranges are controlled by application
                .bind(is_active)
                .execute(&self.pool)
                .await
        }
        .context("Failed to update key version status")?;

        if result.rows_affected() == 0 {
            tracing::warn!(
                "No key version found to update: tenant={:?}, version={}",
                tenant_id,
                version
            );
        } else {
            tracing::debug!(
                "Updated key version {} status to {} for tenant {:?}",
                version,
                is_active,
                tenant_id
            );
        }

        Ok(())
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        let query = match tenant_id {
            Some(_) => {
                r"
                DELETE FROM key_versions 
                WHERE tenant_id = $1 
                AND version NOT IN (
                    SELECT version FROM key_versions 
                    WHERE tenant_id = $1
                    ORDER BY version DESC 
                    LIMIT $2
                )
            "
            }
            None => {
                r"
                DELETE FROM key_versions 
                WHERE tenant_id IS NULL 
                AND version NOT IN (
                    SELECT version FROM key_versions 
                    WHERE tenant_id IS NULL
                    ORDER BY version DESC 
                    LIMIT $1
                )
            "
            }
        };

        let result = if let Some(tid) = tenant_id {
            sqlx::query(query)
                .bind(tid.to_string())
                .bind(i32::try_from(keep_count).unwrap_or(0)) // Safe: keep_count ranges are controlled by application
                .execute(&self.pool)
                .await
        } else {
            sqlx::query(query)
                .bind(i32::try_from(keep_count).unwrap_or(0)) // Safe: keep_count ranges are controlled by application
                .execute(&self.pool)
                .await
        }
        .context("Failed to delete old key versions")?;

        let deleted_count = result.rows_affected();
        tracing::debug!(
            "Deleted {} old key versions for tenant {:?}, kept {} most recent",
            deleted_count,
            tenant_id,
            keep_count
        );

        Ok(deleted_count)
    }

    async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        let query = r"
            SELECT id, slug, name, domain, plan, owner_user_id, created_at, updated_at
            FROM tenants
            WHERE is_active = true
            ORDER BY created_at
        ";

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to get all tenants")?;

        let tenants = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                Ok(crate::models::Tenant {
                    id: uuid::Uuid::parse_str(&row.try_get::<String, _>("id")?)
                        .context("Invalid tenant UUID")?,
                    name: row.try_get("name")?,
                    slug: row.try_get("slug")?,
                    domain: row.try_get("domain")?,
                    plan: row.try_get("plan")?,
                    owner_user_id: uuid::Uuid::parse_str(
                        &row.try_get::<String, _>("owner_user_id")?,
                    )
                    .context("Invalid user UUID")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(tenants)
    }

    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> Result<()> {
        let query = r"
            INSERT INTO audit_events (
                id, event_type, severity, message, source, result, 
                tenant_id, user_id, ip_address, user_agent, metadata, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10, $11, $12)
        ";

        let event_type_str = format!("{:?}", event.event_type);
        let severity_str = format!("{:?}", event.severity);
        let metadata_json = serde_json::to_string(&event.metadata)?;

        sqlx::query(query)
            .bind(event.event_id.to_string())
            .bind(&event_type_str)
            .bind(&severity_str)
            .bind(&event.description)
            .bind("security") // source - using generic security source
            .bind(&event.result)
            .bind(event.tenant_id.map(|id| id.to_string()))
            .bind(event.user_id.map(|id| id.to_string()))
            .bind(&event.source_ip)
            .bind(&event.user_agent)
            .bind(&metadata_json)
            .bind(event.timestamp)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Long function: Complex audit query with dynamic filtering, pagination, and result mapping
    #[allow(clippy::too_many_lines)]
    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        use std::fmt::Write;

        let mut query = r"
            SELECT id, event_type, severity, message, source, result,
                   tenant_id, user_id, ip_address, user_agent, metadata, timestamp
            FROM audit_events
            WHERE true
        "
        .to_string();

        let mut bind_count = 0;
        if tenant_id.is_some() {
            bind_count += 1;
            if write!(query, " AND tenant_id = ${bind_count}").is_err() {
                return Err(anyhow::anyhow!("Failed to write tenant_id clause to query"));
            }
        }
        if event_type.is_some() {
            bind_count += 1;
            if write!(query, " AND event_type = ${bind_count}").is_err() {
                return Err(anyhow::anyhow!(
                    "Failed to write event_type clause to query"
                ));
            }
        }

        query.push_str(" ORDER BY timestamp DESC");

        if limit.is_some() {
            bind_count += 1;
            if write!(query, " LIMIT ${bind_count}").is_err() {
                return Err(anyhow::anyhow!("Failed to write LIMIT clause to query"));
            }
        }

        let mut sql_query = sqlx::query(&query);

        if let Some(tid) = tenant_id {
            sql_query = sql_query.bind(tid.to_string());
        }
        if let Some(et) = event_type {
            sql_query = sql_query.bind(et);
        }
        if let Some(l) = limit {
            sql_query = sql_query.bind(i32::try_from(l).unwrap_or(0)); // Safe: limit ranges are controlled by application
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to get audit events")?;

        let mut events = Vec::new();
        for row in rows {
            let event_id_str: String = row.get("id");
            let event_id =
                uuid::Uuid::parse_str(&event_id_str).context("Invalid audit event UUID")?;

            let event_type_str: String = row.get("event_type");
            let event_type = match event_type_str.as_str() {
                "UserLogin" => crate::security::audit::AuditEventType::UserLogin,
                "UserLogout" => crate::security::audit::AuditEventType::UserLogout,
                "AuthenticationFailed" => {
                    crate::security::audit::AuditEventType::AuthenticationFailed
                }
                "ApiKeyUsed" => crate::security::audit::AuditEventType::ApiKeyUsed,
                "OAuthCredentialsAccessed" => {
                    crate::security::audit::AuditEventType::OAuthCredentialsAccessed
                }
                "OAuthCredentialsModified" => {
                    crate::security::audit::AuditEventType::OAuthCredentialsModified
                }
                "OAuthCredentialsCreated" => {
                    crate::security::audit::AuditEventType::OAuthCredentialsCreated
                }
                "OAuthCredentialsDeleted" => {
                    crate::security::audit::AuditEventType::OAuthCredentialsDeleted
                }
                "TokenRefreshed" => crate::security::audit::AuditEventType::TokenRefreshed,
                "TenantCreated" => crate::security::audit::AuditEventType::TenantCreated,
                "TenantModified" => crate::security::audit::AuditEventType::TenantModified,
                "TenantDeleted" => crate::security::audit::AuditEventType::TenantDeleted,
                "TenantUserAdded" => crate::security::audit::AuditEventType::TenantUserAdded,
                "TenantUserRemoved" => crate::security::audit::AuditEventType::TenantUserRemoved,
                "TenantUserRoleChanged" => {
                    crate::security::audit::AuditEventType::TenantUserRoleChanged
                }
                "DataEncrypted" => crate::security::audit::AuditEventType::DataEncrypted,
                "DataDecrypted" => crate::security::audit::AuditEventType::DataDecrypted,
                "KeyRotated" => crate::security::audit::AuditEventType::KeyRotated,
                "EncryptionFailed" => crate::security::audit::AuditEventType::EncryptionFailed,
                "ToolExecutionFailed" => {
                    crate::security::audit::AuditEventType::ToolExecutionFailed
                }
                "ProviderApiCalled" => crate::security::audit::AuditEventType::ProviderApiCalled,
                "ConfigurationChanged" => {
                    crate::security::audit::AuditEventType::ConfigurationChanged
                }
                "SystemMaintenance" => crate::security::audit::AuditEventType::SystemMaintenance,
                "SecurityPolicyViolation" => {
                    crate::security::audit::AuditEventType::SecurityPolicyViolation
                }
                _ => crate::security::audit::AuditEventType::ToolExecuted, // Default fallback
            };

            let severity_str: String = row.get("severity");
            let severity = match severity_str.as_str() {
                "Warning" => crate::security::audit::AuditSeverity::Warning,
                "Error" => crate::security::audit::AuditSeverity::Error,
                "Critical" => crate::security::audit::AuditSeverity::Critical,
                _ => crate::security::audit::AuditSeverity::Info, // Default fallback
            };

            let tenant_id_str: Option<String> = row.get("tenant_id");
            let tenant_id = if let Some(tid) = tenant_id_str {
                Some(crate::utils::uuid::parse_uuid(&tid)?)
            } else {
                None
            };

            let user_id_str: Option<String> = row.get("user_id");
            let user_id = if let Some(uid) = user_id_str {
                Some(crate::utils::uuid::parse_uuid(&uid)?)
            } else {
                None
            };

            let metadata_json: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

            let event = crate::security::audit::AuditEvent {
                event_id,
                event_type,
                severity,
                timestamp: row.get("timestamp"),
                user_id,
                tenant_id,
                source_ip: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                session_id: None, // Not stored in current schema
                description: row.get("message"),
                metadata,
                resource: None,              // Not stored in current schema
                action: "audit".to_string(), // Default action
                result: row.get("result"),
            };
            events.push(event);
        }

        Ok(events)
    }

    // UserOAuthToken Methods - PostgreSQL implementations
    // ================================

    async fn upsert_user_oauth_token(&self, token: &crate::models::UserOAuthToken) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO user_oauth_tokens (
                id, user_id, tenant_id, provider, access_token, refresh_token,
                token_type, expires_at, scope, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (user_id, tenant_id, provider) 
            DO UPDATE SET
                id = EXCLUDED.id,
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                token_type = EXCLUDED.token_type,
                expires_at = EXCLUDED.expires_at,
                scope = EXCLUDED.scope,
                updated_at = EXCLUDED.updated_at
            ",
        )
        .bind(&token.id)
        .bind(token.user_id)
        .bind(&token.tenant_id)
        .bind(&token.provider)
        .bind(&token.access_token)
        .bind(token.refresh_token.as_deref())
        .bind(&token.token_type)
        .bind(token.expires_at)
        .bind(token.scope.as_deref().unwrap_or(""))
        .bind(token.created_at)
        .bind(token.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<crate::models::UserOAuthToken>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| Ok(Some(Self::row_to_user_oauth_token(&row)?)),
        )
    }

    async fn get_user_oauth_tokens(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<crate::models::UserOAuthToken>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(Self::row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<crate::models::UserOAuthToken>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                   token_type, expires_at, scope, created_at, updated_at
            FROM user_oauth_tokens
            WHERE tenant_id = $1 AND provider = $2
            ORDER BY created_at DESC
            ",
        )
        .bind(tenant_id)
        .bind(provider)
        .fetch_all(&self.pool)
        .await?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(Self::row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    async fn delete_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(provider)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_user_oauth_tokens(&self, user_id: uuid::Uuid) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1
            ",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn refresh_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE user_oauth_tokens
            SET access_token = $4,
                refresh_token = $5,
                expires_at = $6,
                updated_at = CURRENT_TIMESTAMP
            WHERE user_id = $1 AND tenant_id = $2 AND provider = $3
            ",
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(provider)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user role for a specific tenant
    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM tenant_users WHERE user_id = $1 AND tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }

    // ================================
    // User OAuth App Credentials Implementation
    // ================================

    /// Store user OAuth app credentials (client_id, client_secret)
    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        // Create user_oauth_apps table if it doesn't exist
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_oauth_apps (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                provider TEXT NOT NULL,
                client_id TEXT NOT NULL,
                client_secret TEXT NOT NULL,
                redirect_uri TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW(),
                UNIQUE(user_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Insert or update OAuth app credentials
        sqlx::query(
            r"
            INSERT INTO user_oauth_apps (user_id, provider, client_id, client_secret, redirect_uri)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, provider)
            DO UPDATE SET 
                client_id = EXCLUDED.client_id,
                client_secret = EXCLUDED.client_secret,
                redirect_uri = EXCLUDED.redirect_uri,
                updated_at = NOW()
            ",
        )
        .bind(user_id)
        .bind(provider)
        .bind(client_id)
        .bind(client_secret)
        .bind(redirect_uri)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user OAuth app credentials for a provider
    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::models::UserOAuthApp>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_apps
            WHERE user_id = $1 AND provider = $2
            "
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(crate::models::UserOAuthApp {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    provider: row.get("provider"),
                    client_id: row.get("client_id"),
                    client_secret: row.get("client_secret"),
                    redirect_uri: row.get("redirect_uri"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            },
        )
    }

    /// List all OAuth app providers configured for a user
    async fn list_user_oauth_apps(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::UserOAuthApp>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_apps
            WHERE user_id = $1
            ORDER BY provider
            "
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut apps = Vec::new();
        for row in rows {
            apps.push(crate::models::UserOAuthApp {
                id: row.get("id"),
                user_id: row.get("user_id"),
                provider: row.get("provider"),
                client_id: row.get("client_id"),
                client_secret: row.get("client_secret"),
                redirect_uri: row.get("redirect_uri"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(apps)
    }

    /// Remove user OAuth app credentials for a provider
    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_apps
            WHERE user_id = $1 AND provider = $2
            ",
        )
        .bind(user_id)
        .bind(provider)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ================================
    // System Secret Management Implementation
    // ================================

    /// Get or create system secret (generates if not exists)
    async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        // Try to get existing secret
        if let Ok(secret) = self.get_system_secret(secret_type).await {
            return Ok(secret);
        }

        // Generate new secret
        let secret_value = match secret_type {
            "admin_jwt_secret" => crate::admin::jwt::AdminJwtManager::generate_jwt_secret(),
            _ => return Err(anyhow::anyhow!("Unknown secret type: {}", secret_type)),
        };

        // Store in database
        sqlx::query("INSERT INTO system_secrets (secret_type, secret_value) VALUES ($1, $2)")
            .bind(secret_type)
            .bind(&secret_value)
            .execute(&self.pool)
            .await?;

        Ok(secret_value)
    }

    /// Get existing system secret
    async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        let row = sqlx::query("SELECT secret_value FROM system_secrets WHERE secret_type = $1")
            .bind(secret_type)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("secret_value")?)
    }

    /// Update system secret (for rotation)
    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        sqlx::query(
            "UPDATE system_secrets SET secret_value = $1, updated_at = CURRENT_TIMESTAMP WHERE secret_type = $2",
        )
        .bind(new_value)
        .bind(secret_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ================================
    // OAuth Notifications
    // ================================

    async fn store_oauth_notification(
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
            VALUES ($1, $2, $3, $4, $5, $6)
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

    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, success, message, expires_at, created_at, read_at
            FROM oauth_notifications
            WHERE user_id = $1 AND read_at IS NULL
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(crate::database::oauth_notifications::OAuthNotification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                provider: row.get("provider"),
                success: row.get("success"),
                message: row.get("message"),
                expires_at: row.get("expires_at"),
                created_at: row.get("created_at"),
                read_at: row.get("read_at"),
            });
        }

        Ok(notifications)
    }

    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        let result = sqlx::query(
            r"
            UPDATE oauth_notifications 
            SET read_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND user_id = $2 AND read_at IS NULL
            ",
        )
        .bind(notification_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query(
            r"
            UPDATE oauth_notifications 
            SET read_at = CURRENT_TIMESTAMP
            WHERE user_id = $1 AND read_at IS NULL
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        let mut query_str = String::from(
            r"
            SELECT id, user_id, provider, success, message, expires_at, created_at, read_at
            FROM oauth_notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        );

        if let Some(l) = limit {
            write!(query_str, " LIMIT {l}").map_err(|e| anyhow!("Format error: {e}"))?;
        }

        let rows = sqlx::query(&query_str)
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(crate::database::oauth_notifications::OAuthNotification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                provider: row.get("provider"),
                success: row.get("success"),
                message: row.get("message"),
                expires_at: row.get("expires_at"),
                created_at: row.get("created_at"),
                read_at: row.get("read_at"),
            });
        }

        Ok(notifications)
    }

    // ================================
    // Fitness Configuration Management
    // ================================

    /// Save tenant-level fitness configuration
    async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data)
            VALUES ($1, NULL, $2, $3)
            ON CONFLICT (tenant_id, user_id, configuration_name) 
            DO UPDATE SET 
                config_data = EXCLUDED.config_data,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .bind(&config_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get("id"))
    }

    /// Save user-specific fitness configuration
    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tenant_id, user_id, configuration_name) 
            DO UPDATE SET 
                config_data = EXCLUDED.config_data,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(configuration_name)
        .bind(&config_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get("id"))
    }

    /// Get tenant-level fitness configuration
    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let result = sqlx::query(
            r"
            SELECT config_data FROM fitness_configurations
            WHERE tenant_id = $1 AND user_id IS NULL AND configuration_name = $2
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: crate::config::fitness_config::FitnessConfig =
                serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Get user-specific fitness configuration
    async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        // First try to get user-specific configuration
        let result = sqlx::query(
            r"
            SELECT config_data FROM fitness_configurations
            WHERE tenant_id = $1 AND user_id = $2 AND configuration_name = $3
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: crate::config::fitness_config::FitnessConfig =
                serde_json::from_str(&config_json)?;
            return Ok(Some(config));
        }

        // Fall back to tenant default configuration
        let result = sqlx::query(
            r"
            SELECT config_data FROM fitness_configurations
            WHERE tenant_id = $1 AND user_id IS NULL AND configuration_name = $2
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: crate::config::fitness_config::FitnessConfig =
                serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// List all tenant-level fitness configuration names
    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT configuration_name FROM fitness_configurations
            WHERE tenant_id = $1
            ORDER BY configuration_name
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let configurations = rows
            .into_iter()
            .map(|row| row.get::<String, _>("configuration_name"))
            .collect();

        Ok(configurations)
    }

    /// List all user-specific fitness configuration names
    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT configuration_name FROM fitness_configurations
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY configuration_name
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let configurations = rows
            .into_iter()
            .map(|row| row.get::<String, _>("configuration_name"))
            .collect();

        Ok(configurations)
    }

    /// Delete fitness configuration (tenant or user-specific)
    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        let rows_affected = if let Some(uid) = user_id {
            sqlx::query(
                r"
                DELETE FROM fitness_configurations
                WHERE tenant_id = $1 AND user_id = $2 AND configuration_name = $3
                ",
            )
            .bind(tenant_id)
            .bind(uid)
            .bind(configuration_name)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                r"
                DELETE FROM fitness_configurations
                WHERE tenant_id = $1 AND user_id IS NULL AND configuration_name = $2
                ",
            )
            .bind(tenant_id)
            .bind(configuration_name)
            .execute(&self.pool)
            .await?
        };

        Ok(rows_affected.rows_affected() > 0)
    }

    async fn store_oauth2_client(
        &self,
        client: &crate::oauth2::models::OAuth2Client,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth2_clients (id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(&client.id)
        .bind(&client.client_id)
        .bind(&client.client_secret_hash)
        .bind(serde_json::to_string(&client.redirect_uris)?)
        .bind(serde_json::to_string(&client.grant_types)?)
        .bind(serde_json::to_string(&client.response_types)?)
        .bind(&client.client_name)
        .bind(&client.client_uri)
        .bind(&client.scope)
        .bind(client.created_at)
        .bind(client.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2Client>> {
        let row = sqlx::query(
            "SELECT id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at
             FROM oauth2_clients WHERE client_id = $1"
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let redirect_uris: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("redirect_uris"))?;
            let grant_types: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("grant_types"))?;
            let response_types: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("response_types"))?;

            Ok(Some(crate::oauth2::models::OAuth2Client {
                id: row.get("id"),
                client_id: row.get("client_id"),
                client_secret_hash: row.get("client_secret_hash"),
                redirect_uris,
                grant_types,
                response_types,
                client_name: row.get("client_name"),
                client_uri: row.get("client_uri"),
                scope: row.get("scope"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2::models::OAuth2AuthCode,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth2_auth_codes (code, client_id, user_id, redirect_uri, scope, expires_at, used, code_challenge, code_challenge_method)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(&auth_code.code)
        .bind(&auth_code.client_id)
        .bind(auth_code.user_id)
        .bind(&auth_code.redirect_uri)
        .bind(&auth_code.scope)
        .bind(auth_code.expires_at)
        .bind(auth_code.used)
        .bind(&auth_code.code_challenge)
        .bind(&auth_code.code_challenge_method)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2AuthCode>> {
        let row = sqlx::query(
            "SELECT code, client_id, user_id, redirect_uri, scope, expires_at, used, code_challenge, code_challenge_method
             FROM oauth2_auth_codes WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(crate::oauth2::models::OAuth2AuthCode {
                    code: row.get("code"),
                    client_id: row.get("client_id"),
                    user_id: row.get("user_id"),
                    redirect_uri: row.get("redirect_uri"),
                    scope: row.get("scope"),
                    expires_at: row.get("expires_at"),
                    used: row.get("used"),
                    code_challenge: row.get("code_challenge"),
                    code_challenge_method: row.get("code_challenge_method"),
                }))
            },
        )
    }

    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2::models::OAuth2AuthCode,
    ) -> Result<()> {
        sqlx::query("UPDATE oauth2_auth_codes SET used = $1 WHERE code = $2")
            .bind(auth_code.used)
            .bind(&auth_code.code)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store OAuth 2.0 refresh token
    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2::models::OAuth2RefreshToken,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth2_refresh_tokens (token, client_id, user_id, scope, expires_at, created_at, revoked)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(&refresh_token.token)
        .bind(&refresh_token.client_id)
        .bind(refresh_token.user_id)
        .bind(&refresh_token.scope)
        .bind(refresh_token.expires_at)
        .bind(refresh_token.created_at)
        .bind(refresh_token.revoked)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get OAuth 2.0 refresh token
    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2RefreshToken>> {
        let row = sqlx::query(
            "SELECT token, client_id, user_id, scope, expires_at, created_at, revoked
             FROM oauth2_refresh_tokens
             WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(crate::oauth2::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id: row.try_get("user_id")?,
                scope: row.try_get("scope")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                revoked: row.try_get("revoked")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Revoke OAuth 2.0 refresh token
    async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()> {
        sqlx::query("UPDATE oauth2_refresh_tokens SET revoked = true WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

impl PostgresDatabase {
    /// Convert database row to `UserOAuthToken`
    fn row_to_user_oauth_token(
        row: &sqlx::postgres::PgRow,
    ) -> Result<crate::models::UserOAuthToken> {
        use sqlx::Row;

        Ok(crate::models::UserOAuthToken {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            tenant_id: row.try_get("tenant_id")?,
            provider: row.try_get("provider")?,
            access_token: row.try_get("access_token")?,
            refresh_token: row.try_get("refresh_token")?,
            token_type: row.try_get("token_type")?,
            expires_at: row.try_get("expires_at")?,
            scope: row.try_get("scope").ok(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

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
                tenant_id TEXT,
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
                user_status TEXT NOT NULL DEFAULT 'pending' CHECK (user_status IN ('pending', 'active', 'suspended')),
                is_admin BOOLEAN NOT NULL DEFAULT false,
                approved_by UUID REFERENCES users(id),
                approved_at TIMESTAMPTZ,
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

    /// Create OAuth notifications table for MCP resource delivery
    async fn create_oauth_notifications_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth_notifications (
                id TEXT PRIMARY KEY,
                user_id UUID NOT NULL,
                provider TEXT NOT NULL,
                success BOOLEAN NOT NULL DEFAULT true,
                message TEXT NOT NULL,
                expires_at TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                read_at TIMESTAMPTZ,
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

    // Long function: Creates complete multi-tenant database schema with all required tables
    #[allow(clippy::too_many_lines)]
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

        // Create OAuth Apps table for app registration
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth_apps (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                client_id VARCHAR(255) UNIQUE NOT NULL,
                client_secret VARCHAR(255) NOT NULL,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                redirect_uris TEXT[] NOT NULL DEFAULT '{}',
                scopes TEXT[] NOT NULL DEFAULT '{}',
                app_type VARCHAR(50) DEFAULT 'web' CHECK (app_type IN ('desktop', 'web', 'mobile', 'server')),
                owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                is_active BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create Authorization Code table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS authorization_codes (
                code VARCHAR(255) PRIMARY KEY,
                client_id VARCHAR(255) NOT NULL REFERENCES oauth_apps(client_id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                redirect_uri VARCHAR(500) NOT NULL,
                scope VARCHAR(500) NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                expires_at TIMESTAMPTZ NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create user_oauth_tokens table for per-user, per-tenant OAuth tokens
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_oauth_tokens (
                id VARCHAR(255) PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                tenant_id VARCHAR(255) NOT NULL,
                provider VARCHAR(50) NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT,
                token_type VARCHAR(50) DEFAULT 'bearer',
                expires_at TIMESTAMPTZ,
                scope TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, tenant_id, provider)
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

        // UserOAuthToken indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_user ON user_oauth_tokens(user_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_tenant_provider ON user_oauth_tokens(tenant_id, provider)")
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
