// ABOUTME: PostgreSQL database implementation for cloud and production deployments
// ABOUTME: Provides enterprise-grade database support with connection pooling and scalability
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! `PostgreSQL` database implementation
//!
//! This module provides `PostgreSQL` support for cloud deployments,
//! implementing the same interface as the `SQLite` version.

use super::{shared, DatabaseProvider};
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::admin::jwks::JwksManager;
use crate::admin::jwt::AdminJwtManager;
use crate::admin::models::{
    AdminPermissions, AdminToken, AdminTokenUsage, CreateAdminTokenRequest, GeneratedAdminToken,
};
use crate::api_keys::{ApiKey, ApiKeyTier, ApiKeyUsage, ApiKeyUsageStats};
use crate::config::environment::PostgresPoolConfig;
use crate::config::fitness::FitnessConfig;
use crate::constants::http_status::{BAD_REQUEST, SUCCESS_MAX, SUCCESS_MIN};
use crate::constants::tiers;
use crate::dashboard_routes::{RequestLog, ToolUsage};
use crate::database::{
    A2AUsage, A2AUsageStats, ConversationRecord, ConversationSummary, CreateUserMcpTokenRequest,
    MessageRecord, UserMcpToken, UserMcpTokenCreated, UserMcpTokenInfo,
};
use crate::database_plugins::shared::encryption::HasEncryption;
use crate::errors::{AppError, AppResult};
use crate::models::OAuthNotification;
use crate::models::{
    AuthorizationCode, ConnectionType, OAuthApp, ProviderConnection, Tenant, TenantPlan,
    TenantToolOverride, ToolCatalogEntry, ToolCategory, User, UserOAuthApp, UserOAuthToken,
    UserStatus, UserTier,
};
use crate::oauth2_client::OAuthClientState;
use crate::oauth2_server::models::{OAuth2AuthCode, OAuth2Client, OAuth2RefreshToken, OAuth2State};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use crate::permissions::impersonation::ImpersonationSession;
use crate::permissions::UserRole;
use crate::rate_limiting::JwtUsage;
use crate::security::audit::{AuditEvent, AuditEventType, AuditSeverity};
use crate::security::key_rotation::KeyVersion;
use crate::tenant::llm_manager::{LlmCredentialRecord, LlmCredentialSummary};
use crate::tenant::TenantOAuthCredentials;
use crate::utils::uuid::parse_uuid;
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as Base64Engine;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;
use std::fmt::Write;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Type alias for tool catalog seed data tuple
/// Fields: (id, `tool_name`, `display_name`, description, category, `is_enabled_by_default`, `requires_provider`, `min_plan`)
type ToolCatalogSeedEntry<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    bool,
    Option<&'a str>,
    &'a str,
);

/// `PostgreSQL` database implementation
#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Pool<Postgres>,
    encryption_key: Vec<u8>,
}

impl PostgresDatabase {
    /// Close the database connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Update the encryption key used for token encryption/decryption
    ///
    /// This is called after the actual DEK is loaded from the database during
    /// two-tier key management initialization. The database is initially created
    /// with a temporary key, then updated with the real key once it's loaded.
    ///
    /// # Safety
    /// Only call this once during startup, before any encrypted data operations.
    pub fn update_encryption_key(&mut self, new_key: Vec<u8>) {
        self.encryption_key = new_key;
    }

    /// Helper function to parse User from database row
    fn parse_user_from_row(row: &PgRow) -> AppResult<User> {
        shared::mappers::parse_user_from_row(row)
    }

    /// Helper function to build A2A tasks query with dynamic filters
    fn build_a2a_tasks_query(
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<String> {
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

        if limit.is_some() {
            bind_count += 1;
            write!(query, " LIMIT ${bind_count}").map_err(|e| {
                AppError::database(format!("Failed to write LIMIT clause to query: {e}"))
            })?;
        }

        if offset.is_some() {
            bind_count += 1;
            write!(query, " OFFSET ${bind_count}").map_err(|e| {
                AppError::database(format!("Failed to write OFFSET clause to query: {e}"))
            })?;
        }

        Ok(query)
    }

    /// Helper function to parse A2A task from database row
    fn parse_a2a_task_from_row(row: &PgRow) -> AppResult<A2ATask> {
        shared::mappers::parse_a2a_task_from_row(row)
    }

    /// Map a `PostgreSQL` database row to `ToolCatalogEntry`
    fn map_pg_tool_catalog_row(row: &PgRow) -> AppResult<ToolCatalogEntry> {
        let id: String = row.get("id");
        let category_str: String = row.get("category");
        let min_plan_str: String = row.get("min_plan");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");

        Ok(ToolCatalogEntry {
            id,
            tool_name: row.get("tool_name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            category: ToolCategory::parse_str(&category_str)
                .ok_or_else(|| AppError::internal(format!("Invalid category: {category_str}")))?,
            is_enabled_by_default: row.get("is_enabled_by_default"),
            requires_provider: row.get("requires_provider"),
            min_plan: TenantPlan::parse_str(&min_plan_str)
                .ok_or_else(|| AppError::internal(format!("Invalid min_plan: {min_plan_str}")))?,
            created_at,
            updated_at,
        })
    }

    /// Map a `PostgreSQL` database row to `TenantToolOverride`
    fn map_pg_tenant_tool_override_row(row: &PgRow) -> TenantToolOverride {
        let id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let enabled_by_user_id: Option<Uuid> = row.get("enabled_by_user_id");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");

        TenantToolOverride {
            id,
            tenant_id,
            tool_name: row.get("tool_name"),
            is_enabled: row.get("is_enabled"),
            enabled_by_user_id,
            reason: row.get("reason"),
            created_at,
            updated_at,
        }
    }
}

impl PostgresDatabase {
    /// Create new `PostgreSQL` database with provided pool configuration (internal implementation)
    /// This is called by the Database factory with centralized `ServerConfig`
    ///
    /// # Errors
    ///
    /// Returns an error if database connection or pool configuration fails
    async fn new_impl(
        database_url: &str,
        encryption_key: Vec<u8>,
        pool_config: &PostgresPoolConfig,
    ) -> AppResult<Self> {
        // Use pool configuration from ServerConfig (read once at startup)
        let max_connections = pool_config.max_connections;
        let min_connections = pool_config.min_connections;
        let acquire_timeout_secs = pool_config.acquire_timeout_secs;

        // Log connection pool configuration for debugging
        info!(
            "PostgreSQL pool config: max_connections={max_connections}, min_connections={min_connections}, timeout={acquire_timeout_secs}s, retries={}",
            pool_config.connection_retries
        );

        // Attempt connection with exponential backoff retry
        let pool = Self::connect_with_retry(
            database_url,
            max_connections,
            min_connections,
            acquire_timeout_secs,
            pool_config.connection_retries,
            pool_config.initial_retry_delay_ms,
            pool_config.max_retry_delay_ms,
        )
        .await?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    /// Connect to `PostgreSQL` with exponential backoff retry on failure
    ///
    /// Handles transient connection failures (network issues, database restarts)
    /// by retrying with increasing delays between attempts.
    async fn connect_with_retry(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
        acquire_timeout_secs: u64,
        max_retries: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> AppResult<Pool<Postgres>> {
        let pool_options = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(600)))
            // Test connections before returning to caller to detect stale connections
            .test_before_acquire(true);

        let mut last_error = None;
        let mut delay_ms = initial_delay_ms;

        for attempt in 0..=max_retries {
            match pool_options.clone().connect(database_url).await {
                Ok(pool) => {
                    if attempt > 0 {
                        info!(
                            "PostgreSQL connection established after {} retries",
                            attempt
                        );
                    }
                    return Ok(pool);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < max_retries {
                        warn!(
                            "PostgreSQL connection attempt {}/{} failed, retrying in {}ms: {}",
                            attempt + 1,
                            max_retries + 1,
                            delay_ms,
                            last_error.as_ref().map_or("unknown", |e| e
                                .as_database_error()
                                .map_or("connection error", |de| de.message()))
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        // Exponential backoff with cap
                        delay_ms = (delay_ms * 2).min(max_delay_ms);
                    }
                }
            }
        }

        // All retries exhausted
        Err(AppError::database(format!(
            "Failed to connect to PostgreSQL after {} retries: {}",
            max_retries + 1,
            last_error.map_or_else(|| "unknown error".to_owned(), |e| e.to_string())
        )))
    }

    /// Create new `PostgreSQL` database with provided pool configuration (public API)
    /// This is called by the Database factory with centralized `ServerConfig`
    ///
    /// # Errors
    ///
    /// Returns an error if database connection or pool configuration fails
    pub async fn new(
        database_url: &str,
        encryption_key: Vec<u8>,
        pool_config: &PostgresPoolConfig,
    ) -> AppResult<Self> {
        Self::new_impl(database_url, encryption_key, pool_config).await
    }
}

#[async_trait]
impl DatabaseProvider for PostgresDatabase {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> AppResult<Self> {
        // Use default pool configuration when called through trait
        // In practice, the Database factory calls the inherent impl's new() directly with config
        let pool_config = PostgresPoolConfig::default();
        Self::new_impl(database_url, encryption_key, &pool_config).await
    }

    async fn migrate(&self) -> AppResult<()> {
        self.create_users_table().await?;
        self.create_user_profiles_table().await?;
        self.create_goals_table().await?;
        self.create_insights_table().await?;
        self.create_api_keys_tables().await?;
        self.create_a2a_tables().await?;
        self.create_admin_tables().await?;
        self.create_jwt_usage_table().await?;
        self.create_oauth_notifications_table().await?;
        self.create_rsa_keypairs_table().await?;
        self.create_tenant_tables().await?;
        self.create_tool_selection_tables().await?;
        self.create_chat_tables().await?;
        self.create_indexes().await?;
        Ok(())
    }

    async fn create_user(&self, user: &User) -> AppResult<Uuid> {
        sqlx::query(
            r"
            INSERT INTO users (id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin, role, user_status, approved_by, approved_at, created_at, last_active, firebase_uid, auth_provider)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(shared::enums::user_tier_to_str(&user.tier))
        .bind(None::<Option<String>>) // tenant_id is now managed via tenant_users table
        .bind(user.is_active)
        .bind(user.is_admin)
        .bind(shared::enums::user_role_to_str(&user.role))
        .bind(shared::enums::user_status_to_str(&user.user_status))
        .bind(user.approved_by)
        .bind(user.approved_at)
        .bind(user.created_at)
        .bind(user.last_active)
        .bind(&user.firebase_uid)
        .bind(&user.auth_provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create user: {e}")))?;

        Ok(user.id)
    }

    async fn get_user(&self, user_id: Uuid) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   role, user_status, approved_by, approved_at, created_at, last_active,
                   firebase_uid, auth_provider
            FROM users
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user by ID: {e}")))?;

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
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    is_active: row.get("is_active"),
                    user_status: {
                        let status_str: String = row.get("user_status");
                        shared::enums::str_to_user_status(&status_str)
                    },
                    is_admin: row.get("is_admin"),
                    role: {
                        let role_str: Option<String> = row.try_get("role").ok().flatten();
                        role_str.map_or(UserRole::User, |s| shared::enums::str_to_user_role(&s))
                    },
                    approved_by: row.get("approved_by"),
                    approved_at: row.get("approved_at"),
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                    firebase_uid: row.try_get("firebase_uid").ok().flatten(),
                    auth_provider: row
                        .try_get("auth_provider")
                        .unwrap_or_else(|_| "email".to_owned()),
                }))
            },
        )
    }

    async fn get_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   role, user_status, approved_by, approved_at, created_at, last_active,
                   firebase_uid, auth_provider
            FROM users
            WHERE email = $1
            ",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user by email: {e}")))?;

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
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    is_active: row.get("is_active"),
                    user_status: {
                        let status_str: String = row.get("user_status");
                        shared::enums::str_to_user_status(&status_str)
                    },
                    is_admin: row.get("is_admin"),
                    role: {
                        let role_str: Option<String> = row.try_get("role").ok().flatten();
                        role_str.map_or(UserRole::User, |s| shared::enums::str_to_user_role(&s))
                    },
                    approved_by: row.get("approved_by"),
                    approved_at: row.get("approved_at"),
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                    firebase_uid: row.try_get("firebase_uid").ok().flatten(),
                    auth_provider: row
                        .try_get("auth_provider")
                        .unwrap_or_else(|_| "email".to_owned()),
                }))
            },
        )
    }

    async fn get_user_by_email_required(&self, email: &str) -> AppResult<User> {
        self.get_user_by_email(email)
            .await?
            .ok_or_else(|| AppError::not_found(format!("User with email {email}")))
    }

    async fn get_first_admin_user(&self) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   role, user_status, approved_by, approved_at, created_at, last_active,
                   firebase_uid, auth_provider
            FROM users
            WHERE is_admin = true
            ORDER BY created_at ASC
            LIMIT 1
            ",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get first admin user: {e}")))?;

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
                    strava_token: None,
                    fitbit_token: None,
                    is_active: row.get("is_active"),
                    user_status: {
                        let status_str: String = row.get("user_status");
                        shared::enums::str_to_user_status(&status_str)
                    },
                    is_admin: row.get("is_admin"),
                    role: {
                        let role_str: Option<String> = row.try_get("role").ok().flatten();
                        role_str.map_or(UserRole::User, |s| shared::enums::str_to_user_role(&s))
                    },
                    approved_by: row.get("approved_by"),
                    approved_at: row.get("approved_at"),
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                    firebase_uid: row.try_get("firebase_uid").ok().flatten(),
                    auth_provider: row
                        .try_get("auth_provider")
                        .unwrap_or_else(|_| "email".to_owned()),
                }))
            },
        )
    }

    async fn get_user_by_firebase_uid(&self, firebase_uid: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   role, user_status, approved_by, approved_at, created_at, last_active,
                   firebase_uid, auth_provider
            FROM users
            WHERE firebase_uid = $1
            ",
        )
        .bind(firebase_uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user by firebase_uid: {e}")))?;

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
                    strava_token: None, // Tokens are loaded separately
                    fitbit_token: None, // Tokens are loaded separately
                    is_active: row.get("is_active"),
                    user_status: {
                        let status_str: String = row.get("user_status");
                        shared::enums::str_to_user_status(&status_str)
                    },
                    is_admin: row.get("is_admin"),
                    role: {
                        let role_str: Option<String> = row.try_get("role").ok().flatten();
                        role_str.map_or(UserRole::User, |s| shared::enums::str_to_user_role(&s))
                    },
                    approved_by: row.get("approved_by"),
                    approved_at: row.get("approved_at"),
                    created_at: row.get("created_at"),
                    last_active: row.get("last_active"),
                    firebase_uid: row.try_get("firebase_uid").ok().flatten(),
                    auth_provider: row
                        .try_get("auth_provider")
                        .unwrap_or_else(|_| "email".to_owned()),
                }))
            },
        )
    }

    async fn update_last_active(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE users
            SET last_active = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update last active timestamp: {e}")))?;

        Ok(())
    }

    async fn get_user_count(&self) -> AppResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user count: {e}")))?;

        Ok(row.get("count"))
    }

    async fn get_users_by_status(
        &self,
        status: &str,
        tenant_id: Option<Uuid>,
    ) -> AppResult<Vec<User>> {
        // Query users by status from PostgreSQL
        let status_enum = match status {
            "active" => "active",
            "pending" => "pending",
            "suspended" => "suspended",
            _ => {
                return Err(AppError::invalid_input(format!(
                    "Invalid user status: {status}"
                )))
            }
        };

        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                r"
                SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                       role, COALESCE(user_status, 'active') as user_status, approved_by, approved_at,
                       created_at, last_active, firebase_uid, auth_provider
                FROM users
                WHERE COALESCE(user_status, 'active') = $1 AND tenant_id = $2
                ORDER BY created_at DESC
                ",
            )
            .bind(status_enum)
            .bind(tid.to_string())
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r"
                SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                       role, COALESCE(user_status, 'active') as user_status, approved_by, approved_at,
                       created_at, last_active, firebase_uid, auth_provider
                FROM users
                WHERE COALESCE(user_status, 'active') = $1
                ORDER BY created_at DESC
                ",
            )
            .bind(status_enum)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to get users by status: {e}")))?;

        let mut users = Vec::new();
        for row in rows {
            let user_status_str: String = row.get("user_status");
            let user_status = match user_status_str.as_str() {
                "pending" => UserStatus::Pending,
                "suspended" => UserStatus::Suspended,
                _ => UserStatus::Active,
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
                strava_token: None,
                fitbit_token: None,
                is_active: row.get("is_active"),
                user_status,
                is_admin: row.try_get("is_admin").unwrap_or(false), // Default to false for existing users
                role: {
                    let role_str: Option<String> = row.try_get("role").ok().flatten();
                    role_str.map_or(UserRole::User, |s| shared::enums::str_to_user_role(&s))
                },
                approved_by: row.get("approved_by"),
                approved_at: row.get("approved_at"),
                created_at: row.get("created_at"),
                last_active: row.get("last_active"),
                firebase_uid: row.try_get("firebase_uid").ok().flatten(),
                auth_provider: row
                    .try_get("auth_provider")
                    .unwrap_or_else(|_| "email".to_owned()),
            });
        }

        Ok(users)
    }

    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<User>> {
        const QUERY_WITH_CURSOR: &str = r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   COALESCE(user_status, 'active') as user_status, approved_by, approved_at,
                   created_at, last_active, firebase_uid, auth_provider
            FROM users
            WHERE COALESCE(user_status, 'active') = $1
              AND (created_at < $2 OR (created_at = $2 AND id::text < $3))
            ORDER BY created_at DESC, id DESC
            LIMIT $4
        ";

        const QUERY_WITHOUT_CURSOR: &str = r"
            SELECT id, email, display_name, password_hash, tier, tenant_id, is_active, is_admin,
                   COALESCE(user_status, 'active') as user_status, approved_by, approved_at, created_at, last_active
            FROM users
            WHERE COALESCE(user_status, 'active') = $1
            ORDER BY created_at DESC, id DESC
            LIMIT $2
        ";

        // Validate status
        let status_enum = match status {
            "active" => "active",
            "pending" => "pending",
            "suspended" => "suspended",
            _ => {
                return Err(AppError::invalid_input(format!(
                    "Invalid user status: {status}"
                )))
            }
        };

        // Fetch one more than requested to determine if there are more items
        let fetch_limit = params.limit + 1;

        // Convert to i64 for SQL LIMIT clause (pagination limits are always reasonable)
        let fetch_limit_i64 = i64::try_from(fetch_limit).map_err(|e| {
            warn!(
                fetch_limit = fetch_limit,
                max_allowed = i64::MAX,
                error = %e,
                "Pagination limit conversion failed"
            );
            AppError::invalid_input(format!("Pagination limit too large: {fetch_limit}"))
        })?;

        // Execute query with appropriate parameters
        let rows = if let Some(ref cursor) = params.cursor {
            let (timestamp, id) = cursor
                .decode()
                .ok_or_else(|| AppError::invalid_input("Invalid cursor format"))?;

            sqlx::query(QUERY_WITH_CURSOR)
                .bind(status_enum)
                .bind(timestamp)
                .bind(id)
                .bind(fetch_limit_i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to get users by status with cursor: {e}"))
                })?
        } else {
            sqlx::query(QUERY_WITHOUT_CURSOR)
                .bind(status_enum)
                .bind(fetch_limit_i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Failed to get users by status: {e}")))?
        };

        // Parse rows into User structs
        let mut users: Vec<User> = rows
            .iter()
            .map(Self::parse_user_from_row)
            .collect::<AppResult<Vec<_>>>()?;

        // Determine if there are more items
        let has_more = users.len() > params.limit;
        if has_more {
            users.pop(); // Remove the extra item we fetched
        }

        // Generate next cursor from the last item
        let next_cursor = if has_more {
            users
                .last()
                .map(|last_user| Cursor::new(last_user.created_at, &last_user.id.to_string()))
        } else {
            None
        };

        Ok(CursorPage::new(users, next_cursor, None, has_more))
    }

    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: UserStatus,
        approved_by: Option<Uuid>,
    ) -> AppResult<User> {
        let status_str = shared::enums::user_status_to_str(&new_status);

        // Only set approved_by when activating a user and an approver UUID is provided
        let approved_by_str = if new_status == UserStatus::Active {
            approved_by.map(|uuid| uuid.to_string())
        } else {
            None
        };

        let approved_at = if new_status == UserStatus::Active {
            Some(chrono::Utc::now())
        } else {
            None
        };

        // Update user status
        sqlx::query(
            r"
            UPDATE users
            SET user_status = $1, approved_by = $2, approved_at = $3
            WHERE id = $4
            ",
        )
        .bind(status_str)
        .bind(approved_by_str)
        .bind(approved_at)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user status: {e}")))?;

        // Return updated user
        self.get_user(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User after status update"))
    }

    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to update user tenant ID: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        Ok(())
    }

    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> AppResult<()> {
        let result = sqlx::query(
            r"
            UPDATE users
            SET password_hash = $1, last_active = CURRENT_TIMESTAMP
            WHERE id = $2
            ",
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user password: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        Ok(())
    }

    async fn update_user_display_name(&self, user_id: Uuid, display_name: &str) -> AppResult<User> {
        let result = sqlx::query(
            r"
            UPDATE users
            SET display_name = $1, last_active = CURRENT_TIMESTAMP
            WHERE id = $2
            ",
        )
        .bind(display_name)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user display name: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User with ID: {user_id}")));
        }

        self.get_user(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User after display name update"))
    }

    async fn delete_user(&self, user_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            r"
            DELETE FROM users WHERE id = $1
            ",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete user: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User {user_id} not found")));
        }

        Ok(())
    }

    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> AppResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r"
            INSERT INTO user_profiles (user_id, profile_data, created_at, updated_at)
            VALUES ($1, $2, $3, $3)
            ON CONFLICT (user_id)
            DO UPDATE SET profile_data = $2, updated_at = $3
            ",
        )
        .bind(user_id)
        .bind(&profile_data)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to upsert user profile: {e}")))?;

        Ok(())
    }

    async fn get_user_profile(&self, user_id: Uuid) -> AppResult<Option<Value>> {
        let row = sqlx::query(
            r"
            SELECT profile_data
            FROM user_profiles
            WHERE user_id = $1
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user profile: {e}")))?;

        row.map_or_else(|| Ok(None), |row| Ok(Some(row.get("profile_data"))))
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> AppResult<String> {
        let goal_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO goals (id, user_id, goal_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            ",
        )
        .bind(&goal_id)
        .bind(user_id)
        .bind(&goal_data)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create goal: {e}")))?;

        Ok(goal_id)
    }

    async fn get_user_goals(&self, user_id: Uuid) -> AppResult<Vec<Value>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get user goals: {e}")))?;

        Ok(rows.into_iter().map(|row| row.get("goal_data")).collect())
    }

    async fn update_goal_progress(
        &self,
        goal_id: &str,
        user_id: Uuid,
        current_value: f64,
    ) -> AppResult<()> {
        // Use const to avoid clippy warning about format-like strings
        const JSON_PATH: &str = "{current_value}";
        sqlx::query(
            r"
            UPDATE goals
            SET goal_data = jsonb_set(goal_data, $3::text, $1::text::jsonb),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2 AND user_id = $4
            ",
        )
        .bind(current_value)
        .bind(goal_id)
        .bind(JSON_PATH)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update goal progress: {e}")))?;

        Ok(())
    }

    async fn get_user_configuration(&self, user_id: &str) -> AppResult<Option<String>> {
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
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create user_configurations table: {e}"))
        })?;

        let query = "SELECT config_data FROM user_configurations WHERE user_id = $1";

        let row = sqlx::query(query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user configuration: {e}")))?;

        if let Some(row) = row {
            Ok(Some(row.try_get("config_data").map_err(|e| {
                AppError::database(format!("Failed to parse config_data column: {e}"))
            })?))
        } else {
            Ok(None)
        }
    }

    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> AppResult<()> {
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
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create user_configurations table: {e}"))
        })?;

        // Insert or update configuration
        let now = chrono::Utc::now().to_rfc3339();
        let query = r"
            INSERT INTO user_configurations (user_id, config_data, created_at, updated_at)
            VALUES ($1, $2, $3, $3)
            ON CONFLICT(user_id) DO UPDATE SET
                config_data = EXCLUDED.config_data,
                updated_at = $3
        ";

        sqlx::query(query)
            .bind(user_id)
            .bind(config_json)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to save user configuration: {e}")))?;

        Ok(())
    }

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> AppResult<String> {
        let insight_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let insight_json = serde_json::to_string(&insight_data)
            .map_err(|e| AppError::database(format!("Failed to serialize insight: {e}")))?;

        sqlx::query(
            r"
            INSERT INTO insights (id, user_id, insight_type, insight_data, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ",
        )
        .bind(&insight_id)
        .bind(user_id)
        .bind("general") // Default insight type since it's not provided separately
        .bind(&insight_json)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store insight: {e}")))?;

        Ok(insight_id)
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<Value>> {
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
            .await
            .map_err(|e| AppError::database(format!("Failed to get user insights by type: {e}")))?
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
            .await
            .map_err(|e| AppError::database(format!("Failed to get user insights: {e}")))?
        };

        Ok(rows.into_iter().map(|row| row.get("content")).collect())
    }

    async fn create_api_key(&self, api_key: &ApiKey) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create API key: {e}")))?;

        Ok(())
    }

    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> AppResult<Option<ApiKey>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key by prefix: {e}")))?;

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
                        tiers::TRIAL | tiers::STARTER => ApiKeyTier::Starter,
                        tiers::PROFESSIONAL => ApiKeyTier::Professional,
                        tiers::ENTERPRISE => ApiKeyTier::Enterprise,
                        _ => ApiKeyTier::Trial,
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

    async fn get_user_api_keys(&self, user_id: Uuid) -> AppResult<Vec<ApiKey>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get user API keys: {e}")))?;

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
                    tiers::TRIAL | tiers::STARTER => ApiKeyTier::Starter,
                    tiers::PROFESSIONAL => ApiKeyTier::Professional,
                    tiers::ENTERPRISE => ApiKeyTier::Enterprise,
                    _ => ApiKeyTier::Trial,
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

    async fn update_api_key_last_used(&self, api_key_id: &str) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE api_keys
            SET last_used_at = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(api_key_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update API key last used: {e}")))?;

        Ok(())
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to deactivate API key: {e}")))?;

        Ok(())
    }

    async fn get_api_key_by_id(
        &self,
        api_key_id: &str,
        user_id: Option<Uuid>,
    ) -> AppResult<Option<ApiKey>> {
        let row = if let Some(uid) = user_id {
            sqlx::query(
                r"
                SELECT id, user_id, name, description, key_prefix, key_hash, tier,
                       rate_limit_requests, rate_limit_window_seconds, is_active,
                       created_at, last_used_at, expires_at, updated_at
                FROM api_keys
                WHERE id = $1 AND user_id = $2
                ",
            )
            .bind(api_key_id)
            .bind(uid)
            .fetch_optional(&self.pool)
            .await
        } else {
            // Admin callers that legitimately need cross-user access pass None
            sqlx::query(
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
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to get API key by ID: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| {
                use sqlx::Row;
                let tier_str: String = row.get("tier");
                let tier = match tier_str.as_str() {
                    tiers::STARTER => ApiKeyTier::Starter,
                    tiers::PROFESSIONAL => ApiKeyTier::Professional,
                    tiers::ENTERPRISE => ApiKeyTier::Enterprise,
                    _ => ApiKeyTier::Trial, // Default to trial for unknown values (including "trial")
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
    ) -> AppResult<Vec<ApiKey>> {
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
                .map_err(|e| AppError::database(format!("Failed to write LIMIT clause: {e}")))?;
            if let Some(_offset) = offset {
                param_count += 1;
                write!(&mut query, " OFFSET ${param_count}").map_err(|e| {
                    AppError::database(format!("Failed to write OFFSET clause: {e}"))
                })?;
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

        let rows = sqlx_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to list API keys: {e}")))?;

        let mut api_keys = Vec::with_capacity(rows.len());
        for row in rows {
            let tier_str: String = row.get("tier");
            let tier = match tier_str.as_str() {
                tiers::STARTER => ApiKeyTier::Starter,
                tiers::PROFESSIONAL => ApiKeyTier::Professional,
                tiers::ENTERPRISE => ApiKeyTier::Enterprise,
                _ => ApiKeyTier::Trial, // Default to trial for unknown values (including "trial")
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

    async fn cleanup_expired_api_keys(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r"
            UPDATE api_keys
            SET is_active = false
            WHERE expires_at IS NOT NULL AND expires_at < CURRENT_TIMESTAMP AND is_active = true
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to cleanup expired API keys: {e}")))?;

        Ok(result.rows_affected())
    }

    async fn get_expired_api_keys(&self) -> AppResult<Vec<ApiKey>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get expired API keys: {e}")))?;

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
                    tiers::TRIAL | tiers::STARTER => ApiKeyTier::Starter,
                    tiers::PROFESSIONAL => ApiKeyTier::Professional,
                    tiers::ENTERPRISE => ApiKeyTier::Enterprise,
                    _ => ApiKeyTier::Trial,
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

    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to record API key usage: {e}")))?;

        Ok(())
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> AppResult<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count
            FROM api_key_usage
            WHERE api_key_id = $1 AND timestamp >= CURRENT_DATE
            ",
        )
        .bind(api_key_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key current usage: {e}")))?;

        Ok(u32::try_from(row.get::<i64, _>("count").max(0)).unwrap_or(0))
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<ApiKeyUsageStats> {
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
        .bind(i32::from(SUCCESS_MIN))
        .bind(i32::from(SUCCESS_MAX))
        .bind(i32::from(BAD_REQUEST))
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key usage stats: {e}")))?;

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
        .bind(i32::from(SUCCESS_MIN))
        .bind(i32::from(SUCCESS_MAX))
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get tool usage stats: {e}")))?;

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
            api_key_id: api_key_id.to_owned(),
            period_start: start_date,
            period_end: end_date,
            total_requests: u32::try_from(row.0.max(0)).unwrap_or(0),
            successful_requests: u32::try_from(row.1.max(0)).unwrap_or(0),
            failed_requests: u32::try_from(row.2.max(0)).unwrap_or(0),
            total_response_time_ms: row.3.map_or(0u64, |v| u64::try_from(v.max(0)).unwrap_or(0)),
            tool_usage: serde_json::Value::Object(tool_usage),
        })
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to record JWT usage: {e}")))?;

        Ok(())
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> AppResult<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count
            FROM jwt_usage
            WHERE user_id = $1 AND timestamp >= DATE_TRUNC('month', CURRENT_DATE)
            ",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get JWT current usage: {e}")))?;

        Ok(u32::try_from(row.get::<i64, _>("count").max(0)).unwrap_or(0))
    }

    async fn get_request_logs(
        &self,
        user_id: Option<Uuid>,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> AppResult<Vec<RequestLog>> {
        // Build query with proper column mapping for RequestLog struct.
        // When user_id is provided, join with api_keys to scope by ownership.
        let base_query = if user_id.is_some() {
            r"SELECT
                uuid_generate_v4()::text as id,
                u.timestamp,
                u.api_key_id,
                'Unknown' as api_key_name,
                COALESCE(u.endpoint, 'unknown') as tool_name,
                u.status_code::integer as status_code,
                u.response_time_ms,
                NULL::text as error_message,
                u.request_size_bytes,
                u.response_size_bytes
              FROM api_key_usage u
              JOIN api_keys k ON u.api_key_id = k.id
              WHERE 1=1"
        } else {
            r"SELECT
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
              WHERE 1=1"
        };

        let mut condition_strings = Vec::new();
        let col_prefix = if user_id.is_some() { "u." } else { "" };

        let mut param_count = 0;
        if user_id.is_some() {
            param_count += 1;
            condition_strings.push(format!(" AND k.user_id = ${param_count}"));
        }
        if api_key_id.is_some() {
            param_count += 1;
            condition_strings.push(format!(" AND {col_prefix}api_key_id = ${param_count}"));
        }
        if start_time.is_some() {
            param_count += 1;
            condition_strings.push(format!(" AND {col_prefix}timestamp >= ${param_count}"));
        }
        if end_time.is_some() {
            param_count += 1;
            condition_strings.push(format!(" AND {col_prefix}timestamp <= ${param_count}"));
        }
        if status_filter.is_some() {
            param_count += 1;
            condition_strings.push(format!(
                " AND {col_prefix}status_code::text LIKE ${param_count}"
            ));
        }
        if tool_filter.is_some() {
            param_count += 1;
            condition_strings.push(format!(" AND {col_prefix}endpoint ILIKE ${param_count}"));
        }

        let full_query = format!(
            "{}{} ORDER BY {col_prefix}timestamp DESC LIMIT 1000",
            base_query,
            condition_strings.join("")
        );

        // Build query with proper parameter binding
        let mut query_builder = sqlx::query_as::<_, RequestLog>(&full_query);

        if let Some(uid) = user_id {
            query_builder = query_builder.bind(uid);
        }
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

        let results = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get request logs: {e}")))?;
        Ok(results)
    }

    async fn get_system_stats(&self, tenant_id: Option<Uuid>) -> AppResult<(u64, u64)> {
        let user_count_row = if let Some(tid) = tenant_id {
            sqlx::query("SELECT COUNT(*) as count FROM users WHERE tenant_id = $1")
                .bind(tid.to_string())
                .fetch_one(&self.pool)
                .await
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM users")
                .fetch_one(&self.pool)
                .await
        }
        .map_err(|e| {
            AppError::database(format!("Failed to get user count for system stats: {e}"))
        })?;

        let api_key_count_row = if let Some(tid) = tenant_id {
            sqlx::query(
                "SELECT COUNT(*) as count FROM api_keys ak JOIN users u ON ak.user_id = u.id WHERE ak.is_active = true AND u.tenant_id = $1",
            )
            .bind(tid.to_string())
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM api_keys WHERE is_active = true")
                .fetch_one(&self.pool)
                .await
        }
        .map_err(|e| {
            AppError::database(format!("Failed to get API key count for system stats: {e}"))
        })?;

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
    ) -> AppResult<String> {
        // Hash secrets before storage (never store plaintext credentials)
        let secret_hash = format!("{:x}", Sha256::digest(client_secret.as_bytes()));
        let key_hash = format!("{:x}", Sha256::digest(api_key_id.as_bytes()));

        sqlx::query(
            r"
            INSERT INTO a2a_clients (client_id, user_id, name, description, client_secret_hash,
                                    api_key_hash, capabilities, redirect_uris,
                                    is_active, rate_limit_per_minute, rate_limit_per_day)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ",
        )
        .bind(&client.id)
        .bind(client.user_id)
        .bind(&client.name)
        .bind(&client.description)
        .bind(&secret_hash)
        .bind(&key_hash)
        .bind(&client.capabilities)
        .bind(&client.redirect_uris)
        .bind(client.is_active)
        .bind(100i32) // Default rate limit
        .bind(10000i32) // Default daily rate limit
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create A2A client: {e}")))?;

        Ok(client.id.clone()) // Safe: String ownership for return value
    }

    async fn get_a2a_client(&self, client_id: &str) -> AppResult<Option<A2AClient>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A client: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(A2AClient {
                    id: row.get("client_id"),
                    user_id: row.get("user_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    public_key: String::new(), // Postgres schema does not store public_key separately
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

    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> AppResult<Option<A2AClient>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A client by API key ID: {e}")))?;

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

    async fn get_a2a_client_by_name(&self, name: &str) -> AppResult<Option<A2AClient>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A client by name: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(A2AClient {
                    id: row.get("client_id"),
                    user_id: row.get("user_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    public_key: String::new(), // Postgres schema does not store public_key separately
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

    async fn list_a2a_clients(&self, user_id: &Uuid) -> AppResult<Vec<A2AClient>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to list A2A clients: {e}")))?;

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

    async fn deactivate_a2a_client(&self, client_id: &str) -> AppResult<()> {
        let query =
            "UPDATE a2a_clients SET is_active = false, updated_at = NOW() WHERE client_id = $1";

        let result = sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to deactivate A2A client: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("A2A client {client_id}")));
        }

        Ok(())
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> AppResult<Option<(String, String)>> {
        let query = "SELECT client_id, client_secret_hash FROM a2a_clients WHERE client_id = $1 AND is_active = true";

        let row = sqlx::query(query)
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get A2A client credentials: {e}"))
            })?;

        row.map_or_else(
            || Ok(None),
            |row| {
                let id: String = row.get("client_id");
                let secret: String = row.get("client_secret_hash");
                Ok(Some((id, secret)))
            },
        )
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> AppResult<()> {
        let query =
            "UPDATE a2a_sessions SET expires_at = NOW() - INTERVAL '1 hour' WHERE client_id = $1";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to invalidate A2A client sessions: {e}"))
            })?;

        Ok(())
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> AppResult<()> {
        let query = "UPDATE api_keys SET is_active = false WHERE id IN (SELECT api_key_id FROM a2a_client_api_keys WHERE client_id = $1)";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to deactivate client API keys: {e}"))
            })?;

        Ok(())
    }

    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> AppResult<String> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create A2A session: {e}")))?;

        Ok(session_id)
    }

    async fn get_a2a_session(&self, session_token: &str) -> AppResult<Option<A2ASession>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A session: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            let scopes_str: String = row.try_get("granted_scopes").map_err(|e| {
                AppError::database(format!("Failed to parse granted_scopes column: {e}"))
            })?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_str).unwrap_or_else(|_| vec![]);

            Ok(Some(A2ASession {
                id: row.try_get("session_token").map_err(|e| {
                    AppError::database(format!("Failed to parse session_token column: {e}"))
                })?,
                client_id: row.try_get("client_id").map_err(|e| {
                    AppError::database(format!("Failed to parse client_id column: {e}"))
                })?,
                user_id: row.try_get("user_id").map_err(|e| {
                    AppError::database(format!("Failed to parse user_id column: {e}"))
                })?,
                granted_scopes: scopes,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AppError::database(format!("Failed to parse expires_at column: {e}"))
                })?,
                last_activity: row.try_get("last_activity").map_err(|e| {
                    AppError::database(format!("Failed to parse last_activity column: {e}"))
                })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                requests_count: 0, // Would need to be tracked separately
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> AppResult<()> {
        sqlx::query("UPDATE a2a_sessions SET last_activity = NOW() WHERE session_token = $1")
            .bind(session_token)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to update A2A session activity: {e}"))
            })?;

        Ok(())
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> AppResult<Vec<A2ASession>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get active A2A sessions: {e}")))?;

        let mut sessions = Vec::new();
        for row in rows {
            let user_id_str: Option<String> = row.get("user_id");
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()?;

            let granted_scopes_str: String = row.get("granted_scopes");
            let granted_scopes = granted_scopes_str.split(',').map(str::to_owned).collect();

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
    ) -> AppResult<String> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create A2A task: {e}")))?;

        Ok(task_id)
    }

    async fn get_a2a_task(&self, task_id: &str) -> AppResult<Option<A2ATask>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A task: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            let input_str: String = row.try_get("input_data").map_err(|e| {
                AppError::database(format!("Failed to parse input_data column: {e}"))
            })?;
            let input_data: Value = serde_json::from_str(&input_str).unwrap_or_else(|e| {
                warn!(
                    task_id = %task_id,
                    error = %e,
                    "Failed to deserialize A2A task input_data, using null"
                );
                Value::Null
            });

            // Validate input data structure (log type only, never log raw content)
            if !input_data.is_null() && !input_data.is_object() {
                warn!(
                    task_id = %task_id,
                    value_type = %input_data.as_str().map_or_else(|| "non-object", |_| "string"),
                    "Invalid input data structure for task, expected object"
                );
            }

            let result_data =
                row.try_get::<Option<String>, _>("result_data")
                    .map_or(None, |result_str| {
                        result_str.and_then(|s| {
                            serde_json::from_str(&s)
                                .inspect_err(|e| {
                                    warn!(
                                        task_id = %task_id,
                                        error = %e,
                                        "Failed to deserialize A2A task result_data"
                                    );
                                })
                                .ok()
                        })
                    });

            let status_str: String = row
                .try_get("status")
                .map_err(|e| AppError::database(format!("Failed to parse status column: {e}")))?;
            let status = shared::enums::str_to_task_status(&status_str);

            Ok(Some(A2ATask {
                id: row.try_get("task_id").map_err(|e| {
                    AppError::database(format!("Failed to parse task_id column: {e}"))
                })?,
                status,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                completed_at: row.try_get("updated_at").map_err(|e| {
                    AppError::database(format!("Failed to parse updated_at column: {e}"))
                })?,
                result: result_data.clone(), // Safe: JSON value ownership for A2ATask struct
                error: row.try_get("method").map_err(|e| {
                    AppError::database(format!("Failed to parse method column: {e}"))
                })?,
                client_id: row
                    .try_get("client_id")
                    .unwrap_or_else(|_| "unknown".into()),
                task_type: row.try_get("task_type").map_err(|e| {
                    AppError::database(format!("Failed to parse task_type column: {e}"))
                })?,
                input_data,
                output_data: result_data,
                error_message: row.try_get("method").map_err(|e| {
                    AppError::database(format!("Failed to parse method column: {e}"))
                })?,
                updated_at: row.try_get("updated_at").map_err(|e| {
                    AppError::database(format!("Failed to parse updated_at column: {e}"))
                })?,
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
    ) -> AppResult<Vec<A2ATask>> {
        let query = Self::build_a2a_tasks_query(client_id, status_filter, limit, offset)?;

        let mut sql_query = sqlx::query(&query);

        if let Some(client_id_val) = client_id {
            sql_query = sql_query.bind(client_id_val);
        }

        if let Some(status_val) = status_filter {
            let status_str = shared::enums::task_status_to_str(status_val);
            sql_query = sql_query.bind(status_str);
        }

        if let Some(limit_val) = limit {
            sql_query = sql_query.bind(i64::from(limit_val));
        }

        if let Some(offset_val) = offset {
            sql_query = sql_query.bind(i64::from(offset_val));
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to list A2A tasks: {e}")))?;
        rows.iter()
            .map(Self::parse_a2a_task_from_row)
            .collect::<AppResult<Vec<_>>>()
    }

    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> AppResult<()> {
        let status_str = shared::enums::task_status_to_str(status);

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
        .await
        .map_err(|e| AppError::database(format!("Failed to update A2A task status: {e}")))?;

        Ok(())
    }

    async fn record_a2a_usage(&self, usage: &A2AUsage) -> AppResult<()> {
        sqlx::query(
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
        .await
        .inspect_err(|e| {
            warn!(
                client_id = %usage.client_id,
                endpoint = %usage.tool_name,
                status_code = usage.status_code,
                error = %e,
                "Failed to record A2A usage tracking (affects billing/analytics)"
            );
        })
        .map_err(|e| AppError::database(format!("Failed to record A2A usage: {e}")))?;

        Ok(())
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> AppResult<u32> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as usage_count
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp >= NOW() - INTERVAL '1 hour'
            ",
        )
        .bind(client_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A client current usage: {e}")))?;

        let count: i64 = row
            .try_get("usage_count")
            .map_err(|e| AppError::database(format!("Failed to parse usage_count column: {e}")))?;
        Ok(u32::try_from(count.max(0)).unwrap_or(0))
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<A2AUsageStats> {
        use sqlx::Row;

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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A usage stats: {e}")))?;

        let total_requests: i64 = row.try_get("total_requests").map_err(|e| {
            AppError::database(format!("Failed to parse total_requests column: {e}"))
        })?;
        let successful_requests: i64 = row.try_get("successful_requests").map_err(|e| {
            AppError::database(format!("Failed to parse successful_requests column: {e}"))
        })?;
        let failed_requests: i64 = row.try_get("failed_requests").map_err(|e| {
            AppError::database(format!("Failed to parse failed_requests column: {e}"))
        })?;
        let avg_response_time: Option<f64> = row.try_get("avg_response_time").map_err(|e| {
            AppError::database(format!("Failed to parse avg_response_time column: {e}"))
        })?;
        let total_request_bytes: Option<i64> = row.try_get("total_request_bytes").map_err(|e| {
            AppError::database(format!("Failed to parse total_request_bytes column: {e}"))
        })?;
        let total_response_bytes: Option<i64> =
            row.try_get("total_response_bytes").map_err(|e| {
                AppError::database(format!("Failed to parse total_response_bytes column: {e}"))
            })?;

        // Log byte usage for monitoring
        if let (Some(req_bytes), Some(resp_bytes)) = (total_request_bytes, total_response_bytes) {
            debug!(
                "A2A client {} usage: {} req bytes, {} resp bytes",
                client_id, req_bytes, resp_bytes
            );
        }

        Ok(A2AUsageStats {
            client_id: client_id.to_owned(),
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
    ) -> AppResult<Vec<(DateTime<Utc>, u32, u32)>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get A2A client usage history: {e}")))?;

        let mut result = Vec::new();
        for row in rows {
            use sqlx::Row;
            let day: DateTime<Utc> = row
                .try_get("day")
                .map_err(|e| AppError::database(format!("Failed to parse day column: {e}")))?;
            let success_count: i64 = row.try_get("success_count").map_err(|e| {
                AppError::database(format!("Failed to parse success_count column: {e}"))
            })?;
            let error_count: i64 = row.try_get("error_count").map_err(|e| {
                AppError::database(format!("Failed to parse error_count column: {e}"))
            })?;

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
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Option<DateTime<Utc>>> {
        let last_sync: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT last_sync FROM user_oauth_tokens WHERE user_id = $1 AND tenant_id = $2 AND provider = $3",
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get provider last sync: {e}")))?;

        Ok(last_sync)
    }

    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE user_oauth_tokens SET last_sync = $1 WHERE user_id = $2 AND tenant_id = $3 AND provider = $4",
        )
        .bind(sync_time)
        .bind(user_id)
        .bind(tenant_id)
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update provider last sync: {e}")))?;

        Ok(())
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AppResult<Vec<ToolUsage>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to get top tools analysis: {e}")))?;

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
                    warn!(
                        "High error rate for endpoint {}: {:.2}% ({} errors out of {} requests)",
                        endpoint,
                        error_rate * 100.0,
                        error_count,
                        usage_count
                    );
                }
            }

            tool_usage.push(ToolUsage {
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
        request: &CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &JwksManager,
    ) -> AppResult<GeneratedAdminToken> {
        use uuid::Uuid;

        // Generate unique token ID
        let uuid = Uuid::new_v4().simple();
        let token_id = format!("admin_{uuid}");

        // Debug: Log token creation without exposing secrets
        debug!("Creating admin token with RS256 asymmetric signing");

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
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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

    async fn get_admin_token_by_id(&self, token_id: &str) -> AppResult<Option<AdminToken>> {
        let query = r"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE id = $1
        ";

        let row = sqlx::query(query)
            .bind(token_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            Ok(Some(shared::mappers::parse_admin_token_from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_admin_token_by_prefix(&self, token_prefix: &str) -> AppResult<Option<AdminToken>> {
        let query = r"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE token_prefix = $1
        ";

        let row = sqlx::query(query)
            .bind(token_prefix)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            Ok(Some(shared::mappers::parse_admin_token_from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn list_admin_tokens(&self, include_inactive: bool) -> AppResult<Vec<AdminToken>> {
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

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(shared::mappers::parse_admin_token_from_row(&row)?);
        }

        Ok(tokens)
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> AppResult<()> {
        let query = "UPDATE admin_tokens SET is_active = false WHERE id = $1";

        sqlx::query(query)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        let query = r"
            UPDATE admin_tokens 
            SET last_used_at = CURRENT_TIMESTAMP, last_used_ip = $1, usage_count = usage_count + 1
            WHERE id = $2
        ";

        sqlx::query(query)
            .bind(ip_address)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn record_admin_token_usage(&self, usage: &AdminTokenUsage) -> AppResult<()> {
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
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<AdminTokenUsage>> {
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
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut usage_history = Vec::new();
        for row in rows {
            usage_history.push(shared::mappers::parse_admin_token_usage_from_row(&row)?);
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
    ) -> AppResult<()> {
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
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<serde_json::Value>> {
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
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

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
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

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
    // RSA Key Persistence for JWT Signing
    // ================================

    /// Save RSA keypair to database for persistence across restarts
    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: i32,
    ) -> AppResult<()> {
        sqlx::query(
            r"
            INSERT INTO rsa_keypairs (kid, private_key_pem, public_key_pem, created_at, is_active, key_size_bits)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(kid) DO UPDATE SET
                private_key_pem = EXCLUDED.private_key_pem,
                public_key_pem = EXCLUDED.public_key_pem,
                is_active = EXCLUDED.is_active
            ",
        )
        .bind(kid)
        .bind(private_key_pem)
        .bind(public_key_pem)
        .bind(created_at)
        .bind(is_active)
        .bind(key_size_bits)
        .execute(&self.pool).await.map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Load all RSA keypairs from database
    async fn load_rsa_keypairs(
        &self,
    ) -> AppResult<Vec<(String, String, String, DateTime<Utc>, bool)>> {
        let rows = sqlx::query(
            "SELECT kid, private_key_pem, public_key_pem, created_at, is_active FROM rsa_keypairs ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut keypairs = Vec::new();
        for row in rows {
            let kid: String = row.get("kid");
            let private_key_pem: String = row.get("private_key_pem");
            let public_key_pem: String = row.get("public_key_pem");
            let created_at: DateTime<Utc> = row.get("created_at");
            let is_active: bool = row.get("is_active");

            keypairs.push((kid, private_key_pem, public_key_pem, created_at, is_active));
        }

        Ok(keypairs)
    }

    /// Update active status of RSA keypair
    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> AppResult<()> {
        sqlx::query("UPDATE rsa_keypairs SET is_active = $1 WHERE kid = $2")
            .bind(is_active)
            .bind(kid)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    // ================================
    // Multi-Tenant Management
    // ================================

    /// Create a new tenant
    async fn create_tenant(&self, tenant: &Tenant) -> AppResult<()> {
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
        .map_err(|e| AppError::database(format!("Failed to create tenant: {e}")))?;

        // Add the owner as an admin of the tenant
        let tenant_user_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        sqlx::query(
            r"
            INSERT INTO tenant_users (id, tenant_id, user_id, role, invited_at, joined_at)
            VALUES ($1, $2, $3, 'owner', $4, $4)
            ",
        )
        .bind(tenant_user_id)
        .bind(tenant.id)
        .bind(tenant.owner_user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to add owner to tenant: {e}")))?;

        info!(
            "Created tenant: {} ({}) and added owner to tenant_users",
            tenant.name, tenant.id
        );
        Ok(())
    }

    /// Get tenant by ID
    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> AppResult<Tenant> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, Uuid, DateTime<Utc>, DateTime<Utc>)>(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.subscription_tier, tu.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id AND tu.role = 'owner'
            WHERE t.id = $1 AND t.is_active = true
            ",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        match row {
            Some((id, name, slug, domain, plan, owner_user_id, created_at, updated_at)) => {
                Ok(Tenant {
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
            None => Err(AppError::not_found(format!("Tenant {tenant_id}"))),
        }
    }

    /// Get tenant by slug
    async fn get_tenant_by_slug(&self, slug: &str) -> AppResult<Tenant> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, Uuid, DateTime<Utc>, DateTime<Utc>)>(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.subscription_tier, tu.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id AND tu.role = 'owner'
            WHERE t.slug = $1 AND t.is_active = true
            ",
        )
        .bind(slug)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        match row {
            Some((id, name, slug, domain, plan, owner_user_id, created_at, updated_at)) => {
                Ok(Tenant {
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
            None => Err(AppError::not_found(format!("Tenant {slug}"))),
        }
    }

    /// List tenants for a user
    async fn list_tenants_for_user(&self, user_id: Uuid) -> AppResult<Vec<Tenant>> {
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
            ORDER BY tu.joined_at ASC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let tenants = rows
            .into_iter()
            .map(
                |(id, name, slug, domain, plan, owner_user_id, created_at, updated_at)| Tenant {
                    id,
                    name,
                    slug,
                    domain,
                    plan,
                    owner_user_id,
                    created_at,
                    updated_at,
                },
            )
            .collect();

        Ok(tenants)
    }

    /// Store tenant OAuth credentials
    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &TenantOAuthCredentials,
    ) -> AppResult<()> {
        // Encrypt the client secret using AES-256-GCM with AAD binding
        // AAD context format: "{tenant_id}|{provider}|tenant_oauth_credentials"
        let aad_context = format!(
            "{}|{}|tenant_oauth_credentials",
            credentials.tenant_id, credentials.provider
        );
        let encrypted_secret =
            HasEncryption::encrypt_data_with_aad(self, &credentials.client_secret, &aad_context)?;

        // Convert scopes Vec<String> to PostgreSQL array format
        let scopes_array: Vec<&str> = credentials.scopes.iter().map(String::as_str).collect();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO tenant_oauth_credentials
                (tenant_id, provider, client_id, client_secret_encrypted,
                 redirect_uri, scopes, rate_limit_per_day, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $8)
            ON CONFLICT (tenant_id, provider)
            DO UPDATE SET
                client_id = EXCLUDED.client_id,
                client_secret_encrypted = EXCLUDED.client_secret_encrypted,
                redirect_uri = EXCLUDED.redirect_uri,
                scopes = EXCLUDED.scopes,
                rate_limit_per_day = EXCLUDED.rate_limit_per_day,
                updated_at = EXCLUDED.updated_at
            ",
        )
        .bind(credentials.tenant_id)
        .bind(&credentials.provider)
        .bind(&credentials.client_id)
        .bind(&encrypted_secret)
        .bind(&credentials.redirect_uri)
        .bind(&scopes_array)
        .bind(i32::try_from(credentials.rate_limit_per_day).unwrap_or(i32::MAX))
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store OAuth credentials: {e}")))?;

        Ok(())
    }

    /// Get tenant OAuth providers
    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> AppResult<Vec<TenantOAuthCredentials>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Vec<String>, i32)>(
            r"
            SELECT provider, client_id, client_secret_encrypted,
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_credentials
            WHERE tenant_id = $1 AND is_active = true
            ORDER BY provider
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let credentials = rows
            .into_iter()
            .map(
                |(provider, client_id, encrypted_secret, redirect_uri, scopes, rate_limit)| {
                    // Decrypt the client secret using AAD binding
                    // AAD context format: "{tenant_id}|{provider}|tenant_oauth_credentials"
                    let aad_context = format!("{tenant_id}|{provider}|tenant_oauth_credentials");
                    let client_secret = HasEncryption::decrypt_data_with_aad(
                        self,
                        &encrypted_secret,
                        &aad_context,
                    )?;

                    Ok(TenantOAuthCredentials {
                        tenant_id,
                        provider,
                        client_id,
                        client_secret,
                        redirect_uri,
                        scopes,
                        rate_limit_per_day: u32::try_from(rate_limit).unwrap_or(0),
                    })
                },
            )
            .collect::<AppResult<Vec<_>>>()?;

        Ok(credentials)
    }

    /// Get tenant OAuth credentials for specific provider
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<TenantOAuthCredentials>> {
        let row = sqlx::query_as::<_, (String, String, String, Vec<String>, i32)>(
            r"
            SELECT client_id, client_secret_encrypted,
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_credentials
            WHERE tenant_id = $1 AND provider = $2 AND is_active = true
            ",
        )
        .bind(tenant_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        match row {
            Some((client_id, encrypted_secret, redirect_uri, scopes, rate_limit)) => {
                // Decrypt the client secret using AAD binding
                // AAD context format: "{tenant_id}|{provider}|tenant_oauth_credentials"
                let aad_context = format!("{tenant_id}|{provider}|tenant_oauth_credentials");
                let client_secret =
                    HasEncryption::decrypt_data_with_aad(self, &encrypted_secret, &aad_context)?;

                Ok(Some(TenantOAuthCredentials {
                    tenant_id,
                    provider: provider.to_owned(),
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
    async fn create_oauth_app(&self, app: &OAuthApp) -> AppResult<()> {
        let redirect_uris: Vec<&str> = app.redirect_uris.iter().map(String::as_str).collect();
        let scopes: Vec<&str> = app.scopes.iter().map(String::as_str).collect();

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
        .map_err(|e| AppError::database(format!("Failed to create OAuth app: {e}")))?;

        Ok(())
    }

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> AppResult<OAuthApp> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

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
            )) => Ok(OAuthApp {
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
            None => Err(AppError::not_found(format!("OAuth app {client_id}"))),
        }
    }

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(&self, user_id: Uuid) -> AppResult<Vec<OAuthApp>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

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
                    OAuthApp {
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
    ) -> AppResult<()> {
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
        .map_err(|e| AppError::database(format!("Failed to store authorization code: {e}")))?;

        Ok(())
    }

    /// Get authorization code data
    async fn get_authorization_code(&self, code: &str) -> AppResult<AuthorizationCode> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        match row {
            Some((code, client_id, user_id, redirect_uri, scope, created_at, expires_at)) => {
                Ok(AuthorizationCode {
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
            None => Err(AppError::not_found(
                "Authorization code not found or expired".to_owned(),
            )),
        }
    }

    /// Delete authorization code
    async fn delete_authorization_code(&self, code: &str) -> AppResult<()> {
        let result = sqlx::query(
            r"
            DELETE FROM authorization_codes
            WHERE code = $1
            ",
        )
        .bind(code)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete authorization code: {e}")))?;

        if result.rows_affected() == 0 {
            warn!("Authorization code not found for deletion (code redacted)");
        }

        Ok(())
    }

    // ================================
    // Key Rotation & Security - PostgreSQL implementations
    // ================================

    async fn store_key_version(&self, version: &KeyVersion) -> AppResult<()> {
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
            .map_err(|e| AppError::database(format!("Failed to store key version: {e}")))?;

        debug!(
            "Stored key version {} for tenant {:?}",
            version.version, version.tenant_id
        );
        Ok(())
    }

    async fn get_key_versions(&self, tenant_id: Option<Uuid>) -> AppResult<Vec<KeyVersion>> {
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
        .map_err(|e| AppError::database(format!("Failed to fetch key versions: {e}")))?;

        let mut versions = Vec::new();
        for row in rows {
            let tenant_id_str: Option<String> = row.get("tenant_id");
            let tenant_id = if let Some(tid) = tenant_id_str {
                Some(parse_uuid(&tid)?)
            } else {
                None
            };

            let version = KeyVersion {
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
    ) -> AppResult<Option<KeyVersion>> {
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
        .map_err(|e| AppError::database(format!("Failed to fetch current key version: {e}")))?;

        if let Some(row) = row {
            let tenant_id_str: Option<String> = row.get("tenant_id");
            let tenant_id = if let Some(tid) = tenant_id_str {
                Some(parse_uuid(&tid)?)
            } else {
                None
            };

            let version = KeyVersion {
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
    ) -> AppResult<()> {
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
        .map_err(|e| AppError::database(format!("Failed to update key version status: {e}")))?;

        if result.rows_affected() == 0 {
            warn!(
                "No key version found to update: tenant={:?}, version={}",
                tenant_id, version
            );
        } else {
            debug!(
                "Updated key version {} status to {} for tenant {:?}",
                version, is_active, tenant_id
            );
        }

        Ok(())
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> AppResult<u64> {
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
        .map_err(|e| AppError::database(format!("Failed to delete old key versions: {e}")))?;

        let deleted_count = result.rows_affected();
        debug!(
            "Deleted {} old key versions for tenant {:?}, kept {} most recent",
            deleted_count, tenant_id, keep_count
        );

        Ok(deleted_count)
    }

    async fn get_all_tenants(&self) -> AppResult<Vec<Tenant>> {
        let query = r"
            SELECT id, slug, name, domain, plan, owner_user_id, created_at, updated_at
            FROM tenants
            WHERE is_active = true
            ORDER BY created_at
        ";

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get all tenants: {e}")))?;

        let tenants = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                Ok(Tenant {
                    id: uuid::Uuid::parse_str(&row.try_get::<String, _>("id").map_err(|e| {
                        AppError::database(format!("Failed to parse id column: {e}"))
                    })?)
                    .map_err(|e| AppError::database(format!("Invalid tenant UUID: {e}")))?,
                    name: row.try_get("name").map_err(|e| {
                        AppError::database(format!("Failed to parse name column: {e}"))
                    })?,
                    slug: row.try_get("slug").map_err(|e| {
                        AppError::database(format!("Failed to parse slug column: {e}"))
                    })?,
                    domain: row.try_get("domain").map_err(|e| {
                        AppError::database(format!("Failed to parse domain column: {e}"))
                    })?,
                    plan: row.try_get("plan").map_err(|e| {
                        AppError::database(format!("Failed to parse plan column: {e}"))
                    })?,
                    owner_user_id: uuid::Uuid::parse_str(
                        &row.try_get::<String, _>("owner_user_id").map_err(|e| {
                            AppError::database(format!("Failed to parse owner_user_id column: {e}"))
                        })?,
                    )
                    .map_err(|e| AppError::database(format!("Invalid user UUID: {e}")))?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AppError::database(format!("Failed to parse created_at column: {e}"))
                    })?,
                    updated_at: row.try_get("updated_at").map_err(|e| {
                        AppError::database(format!("Failed to parse updated_at column: {e}"))
                    })?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok(tenants)
    }

    async fn store_audit_event(&self, event: &AuditEvent) -> AppResult<()> {
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
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Complex audit query with dynamic filtering, pagination, and exhaustive enum mapping
    ///
    /// JUSTIFICATION for `#[allow(clippy::too_many_lines)]`:
    /// - Dynamic SQL query building with optional filters (`tenant_id`, `event_type`, `limit`)
    /// - Exhaustive match for 25+ `AuditEventType` variants (cannot be extracted without loss of context)
    /// - Exhaustive match for `AuditSeverity` variants
    /// - Row-to-struct mapping with UUID parsing and JSON deserialization
    /// - Refactoring would fragment audit event construction logic across multiple functions
    #[allow(clippy::too_many_lines)]
    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<AuditEvent>> {
        use std::fmt::Write;

        let mut query = r"
            SELECT id, event_type, severity, message, source, result,
                   tenant_id, user_id, ip_address, user_agent, metadata, timestamp
            FROM audit_events
            WHERE true
        "
        .to_owned();

        let mut bind_count = 0;
        if tenant_id.is_some() {
            bind_count += 1;
            if write!(query, " AND tenant_id = ${bind_count}").is_err() {
                return Err(AppError::database(
                    "Failed to write tenant_id clause to query".to_owned(),
                ));
            }
        }
        if event_type.is_some() {
            bind_count += 1;
            if write!(query, " AND event_type = ${bind_count}").is_err() {
                return Err(AppError::database(
                    "Failed to write event_type clause to query".to_owned(),
                ));
            }
        }

        query.push_str(" ORDER BY timestamp DESC");

        if limit.is_some() {
            bind_count += 1;
            if write!(query, " LIMIT ${bind_count}").is_err() {
                return Err(AppError::database(
                    "Failed to write LIMIT clause to query".to_owned(),
                ));
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
            .map_err(|e| AppError::database(format!("Failed to get audit events: {e}")))?;

        let mut events = Vec::new();
        for row in rows {
            let event_id_str: String = row.get("id");
            let event_id = uuid::Uuid::parse_str(&event_id_str)
                .map_err(|e| AppError::database(format!("Invalid audit event UUID: {e}")))?;

            let event_type_str: String = row.get("event_type");
            let event_type = match event_type_str.as_str() {
                "UserLogin" => AuditEventType::UserLogin,
                "UserLogout" => AuditEventType::UserLogout,
                "AuthenticationFailed" => AuditEventType::AuthenticationFailed,
                "ApiKeyUsed" => AuditEventType::ApiKeyUsed,
                "OAuthCredentialsAccessed" => AuditEventType::OAuthCredentialsAccessed,
                "OAuthCredentialsModified" => AuditEventType::OAuthCredentialsModified,
                "OAuthCredentialsCreated" => AuditEventType::OAuthCredentialsCreated,
                "OAuthCredentialsDeleted" => AuditEventType::OAuthCredentialsDeleted,
                "TokenRefreshed" => AuditEventType::TokenRefreshed,
                "TenantCreated" => AuditEventType::TenantCreated,
                "TenantModified" => AuditEventType::TenantModified,
                "TenantDeleted" => AuditEventType::TenantDeleted,
                "TenantUserAdded" => AuditEventType::TenantUserAdded,
                "TenantUserRemoved" => AuditEventType::TenantUserRemoved,
                "TenantUserRoleChanged" => AuditEventType::TenantUserRoleChanged,
                "DataEncrypted" => AuditEventType::DataEncrypted,
                "DataDecrypted" => AuditEventType::DataDecrypted,
                "KeyRotated" => AuditEventType::KeyRotated,
                "EncryptionFailed" => AuditEventType::EncryptionFailed,
                "ToolExecutionFailed" => AuditEventType::ToolExecutionFailed,
                "ProviderApiCalled" => AuditEventType::ProviderApiCalled,
                "ConfigurationChanged" => AuditEventType::ConfigurationChanged,
                "SystemMaintenance" => AuditEventType::SystemMaintenance,
                "SecurityPolicyViolation" => AuditEventType::SecurityPolicyViolation,
                _ => AuditEventType::ToolExecuted, // Default fallback
            };

            let severity_str: String = row.get("severity");
            let severity = match severity_str.as_str() {
                "Warning" => AuditSeverity::Warning,
                "Error" => AuditSeverity::Error,
                "Critical" => AuditSeverity::Critical,
                _ => AuditSeverity::Info, // Default fallback
            };

            let tenant_id_str: Option<String> = row.get("tenant_id");
            let tenant_id = if let Some(tid) = tenant_id_str {
                Some(parse_uuid(&tid)?)
            } else {
                None
            };

            let user_id_str: Option<String> = row.get("user_id");
            let user_id = if let Some(uid) = user_id_str {
                Some(parse_uuid(&uid)?)
            } else {
                None
            };

            let metadata_json: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

            let event = AuditEvent {
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
                resource: None,             // Not stored in current schema
                action: "audit".to_owned(), // Default action
                result: row.get("result"),
            };
            events.push(event);
        }

        Ok(events)
    }

    // UserOAuthToken Methods - PostgreSQL implementations
    // ================================

    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> AppResult<()> {
        // SECURITY: Encrypt OAuth tokens at rest with AAD binding (AES-256-GCM)
        let encrypted_access_token = shared::encryption::encrypt_oauth_token(
            self,
            &token.access_token,
            &token.tenant_id,
            token.user_id,
            &token.provider,
        )?;

        let encrypted_refresh_token = token
            .refresh_token
            .as_ref()
            .map(|rt| {
                shared::encryption::encrypt_oauth_token(
                    self,
                    rt,
                    &token.tenant_id,
                    token.user_id,
                    &token.provider,
                )
            })
            .transpose()?;

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
        .bind(&encrypted_access_token)
        .bind(encrypted_refresh_token.as_deref())
        .bind(&token.token_type)
        .bind(token.expires_at)
        .bind(token.scope.as_deref().unwrap_or(""))
        .bind(token.created_at)
        .bind(token.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn get_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Option<UserOAuthToken>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| Ok(Some(self.row_to_user_oauth_token(&row)?)),
        )
    }

    async fn get_user_oauth_tokens(
        &self,
        user_id: uuid::Uuid,
        tenant_id: Option<&str>,
    ) -> AppResult<Vec<UserOAuthToken>> {
        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, access_token, refresh_token,
                       token_type, expires_at, scope, created_at, updated_at
                FROM user_oauth_tokens
                WHERE user_id = $1 AND tenant_id = $2
                ORDER BY created_at DESC
                ",
            )
            .bind(user_id)
            .bind(tid)
            .fetch_all(&self.pool)
            .await
        } else {
            // Intentional cross-tenant view for OAuth status checks (e.g. admin views)
            sqlx::query(
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
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(self.row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Vec<UserOAuthToken>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(self.row_to_user_oauth_token(&row)?);
        }
        Ok(tokens)
    }

    async fn delete_user_oauth_token(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn delete_user_oauth_tokens(
        &self,
        user_id: uuid::Uuid,
        tenant_id: &str,
    ) -> AppResult<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_tokens
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(user_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<()> {
        // SECURITY: Encrypt OAuth tokens at rest with AAD binding (AES-256-GCM)
        let encrypted_access_token = shared::encryption::encrypt_oauth_token(
            self,
            access_token,
            tenant_id,
            user_id,
            provider,
        )?;

        let encrypted_refresh_token = refresh_token
            .map(|rt| {
                shared::encryption::encrypt_oauth_token(self, rt, tenant_id, user_id, provider)
            })
            .transpose()?;

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
        .bind(&encrypted_access_token)
        .bind(encrypted_refresh_token.as_deref())
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Get user role for a specific tenant
    async fn get_user_tenant_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> AppResult<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM tenant_users WHERE user_id = $1 AND tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        Ok(row.map(|r| r.0))
    }

    // ================================
    // User OAuth App Credentials Implementation
    // ================================

    /// Store user OAuth app credentials (`client_id`, `client_secret`)
    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Get user OAuth app credentials for a provider
    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<UserOAuthApp>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_apps
            WHERE user_id = $1 AND provider = $2
            "
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(UserOAuthApp {
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
    async fn list_user_oauth_apps(&self, user_id: Uuid) -> AppResult<Vec<UserOAuthApp>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_apps
            WHERE user_id = $1
            ORDER BY provider
            "
        )
        .bind(user_id)
        .fetch_all(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut apps = Vec::new();
        for row in rows {
            apps.push(UserOAuthApp {
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
    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> AppResult<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_apps
            WHERE user_id = $1 AND provider = $2
            ",
        )
        .bind(user_id)
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    // ================================
    // System Secret Management Implementation
    // ================================

    /// Get or create system secret (generates if not exists)
    async fn get_or_create_system_secret(&self, secret_type: &str) -> AppResult<String> {
        // Try to get existing secret
        if let Ok(secret) = self.get_system_secret(secret_type).await {
            return Ok(secret);
        }

        // Generate new secret
        let secret_value = match secret_type {
            "admin_jwt_secret" => AdminJwtManager::generate_jwt_secret(),
            _ => {
                return Err(AppError::invalid_input(format!(
                    "Unknown secret type: {secret_type}"
                )))
            }
        };

        // Store in database
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO system_secrets (secret_type, secret_value, created_at, updated_at) VALUES ($1, $2, $3, $4)")
            .bind(secret_type)
            .bind(&secret_value)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(secret_value)
    }

    /// Get existing system secret
    async fn get_system_secret(&self, secret_type: &str) -> AppResult<String> {
        let row = sqlx::query("SELECT secret_value FROM system_secrets WHERE secret_type = $1")
            .bind(secret_type)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch record: {e}")))?;

        Ok(row
            .try_get("secret_value")
            .map_err(|e| AppError::database(format!("Failed to parse secret_value column: {e}")))?)
    }

    /// Update or insert system secret (supports both initial storage and rotation)
    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> AppResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO system_secrets (secret_type, secret_value, created_at, updated_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT(secret_type) DO UPDATE SET secret_value = EXCLUDED.secret_value, updated_at = EXCLUDED.updated_at",
        )
        .bind(secret_type)
        .bind(new_value)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool).await.map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<String> {
        let notification_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO oauth_notifications (id, user_id, provider, success, message, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ",
        )
        .bind(&notification_id)
        .bind(user_id.to_string())
        .bind(provider)
        .bind(success)
        .bind(message)
        .bind(expires_at)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(notification_id)
    }

    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> AppResult<Vec<OAuthNotification>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(OAuthNotification {
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
    ) -> AppResult<bool> {
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
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query(
            r"
            UPDATE oauth_notifications 
            SET read_at = CURRENT_TIMESTAMP
            WHERE user_id = $1 AND read_at IS NULL
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(result.rows_affected())
    }

    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> AppResult<Vec<OAuthNotification>> {
        let mut query_str = String::from(
            r"
            SELECT id, user_id, provider, success, message, expires_at, created_at, read_at
            FROM oauth_notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        );

        if let Some(l) = limit {
            write!(query_str, " LIMIT {l}")
                .map_err(|e| AppError::internal(format!("Format error: {e}")))?;
        }

        let rows = sqlx::query(&query_str)
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(OAuthNotification {
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
        config: &FitnessConfig,
    ) -> AppResult<String> {
        let config_json = serde_json::to_string(config)?;
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data, created_at, updated_at)
            VALUES ($1, NULL, $2, $3, $4, $4)
            ON CONFLICT (tenant_id, user_id, configuration_name)
            DO UPDATE SET
                config_data = EXCLUDED.config_data,
                updated_at = EXCLUDED.updated_at
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .bind(&config_json)
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch record: {e}")))?;

        Ok(result.get("id"))
    }

    /// Save user-specific fitness configuration
    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &FitnessConfig,
    ) -> AppResult<String> {
        let config_json = serde_json::to_string(config)?;
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $5)
            ON CONFLICT (tenant_id, user_id, configuration_name)
            DO UPDATE SET
                config_data = EXCLUDED.config_data,
                updated_at = EXCLUDED.updated_at
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(configuration_name)
        .bind(&config_json)
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch record: {e}")))?;

        Ok(result.get("id"))
    }

    /// Get tenant-level fitness configuration
    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> AppResult<Option<FitnessConfig>> {
        let result = sqlx::query(
            r"
            SELECT config_data FROM fitness_configurations
            WHERE tenant_id = $1 AND user_id IS NULL AND configuration_name = $2
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: FitnessConfig = serde_json::from_str(&config_json)?;
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
    ) -> AppResult<Option<FitnessConfig>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: FitnessConfig = serde_json::from_str(&config_json)?;
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = result {
            let config_json: String = row.get("config_data");
            let config: FitnessConfig = serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// List all tenant-level fitness configuration names
    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> AppResult<Vec<String>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT configuration_name FROM fitness_configurations
            WHERE tenant_id = $1
            ORDER BY configuration_name
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

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
    ) -> AppResult<Vec<String>> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch records: {e}")))?;

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
    ) -> AppResult<bool> {
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
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?
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
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?
        };

        Ok(rows_affected.rows_affected() > 0)
    }

    async fn store_oauth2_client(&self, client: &OAuth2Client) -> AppResult<()> {
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
        .execute(&self.pool).await.map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn get_oauth2_client(&self, client_id: &str) -> AppResult<Option<OAuth2Client>> {
        let row = sqlx::query(
            "SELECT id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at
             FROM oauth2_clients WHERE client_id = $1"
        )
        .bind(client_id)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            let redirect_uris: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("redirect_uris"))?;
            let grant_types: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("grant_types"))?;
            let response_types: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("response_types"))?;

            Ok(Some(OAuth2Client {
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

    async fn store_oauth2_auth_code(&self, auth_code: &OAuth2AuthCode) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO oauth2_auth_codes (code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(&auth_code.code)
        .bind(&auth_code.client_id)
        .bind(auth_code.user_id)
        .bind(&auth_code.tenant_id)
        .bind(&auth_code.redirect_uri)
        .bind(&auth_code.scope)
        .bind(auth_code.expires_at)
        .bind(auth_code.used)
        .bind(&auth_code.state)
        .bind(&auth_code.code_challenge)
        .bind(&auth_code.code_challenge_method)
        .execute(&self.pool).await.map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    async fn get_oauth2_auth_code(&self, code: &str) -> AppResult<Option<OAuth2AuthCode>> {
        let row = sqlx::query(
            "SELECT code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method
             FROM oauth2_auth_codes WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| {
                Ok(Some(OAuth2AuthCode {
                    code: row.get("code"),
                    client_id: row.get("client_id"),
                    user_id: row.get("user_id"),
                    tenant_id: row.get("tenant_id"),
                    redirect_uri: row.get("redirect_uri"),
                    scope: row.get("scope"),
                    expires_at: row.get("expires_at"),
                    used: row.get("used"),
                    state: row.get("state"),
                    code_challenge: row.get("code_challenge"),
                    code_challenge_method: row.get("code_challenge_method"),
                }))
            },
        )
    }

    async fn update_oauth2_auth_code(&self, auth_code: &OAuth2AuthCode) -> AppResult<()> {
        sqlx::query("UPDATE oauth2_auth_codes SET used = $1 WHERE code = $2")
            .bind(auth_code.used)
            .bind(&auth_code.code)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Store OAuth 2.0 refresh token
    ///
    /// The refresh token value is HMAC-SHA256 hashed before storage so that
    /// plaintext tokens are never persisted to disk.
    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &OAuth2RefreshToken,
    ) -> AppResult<()> {
        let token_hash = HasEncryption::hash_token_for_storage(self, &refresh_token.token)?;

        sqlx::query(
            "INSERT INTO oauth2_refresh_tokens (token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(&token_hash)
        .bind(&refresh_token.client_id)
        .bind(refresh_token.user_id)
        .bind(&refresh_token.tenant_id)
        .bind(&refresh_token.scope)
        .bind(refresh_token.expires_at)
        .bind(refresh_token.created_at)
        .bind(refresh_token.revoked)
        .execute(&self.pool).await.map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Get OAuth 2.0 refresh token
    ///
    /// The input token is HMAC-SHA256 hashed before querying.
    async fn get_oauth2_refresh_token(&self, token: &str) -> AppResult<Option<OAuth2RefreshToken>> {
        let token_hash = HasEncryption::hash_token_for_storage(self, token)?;

        let row = sqlx::query(
            "SELECT token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
             FROM oauth2_refresh_tokens
             WHERE token = $1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(OAuth2RefreshToken {
                token: row.try_get("token").map_err(|e| {
                    AppError::database(format!("Failed to parse token column: {e}"))
                })?,
                client_id: row.try_get("client_id").map_err(|e| {
                    AppError::database(format!("Failed to parse client_id column: {e}"))
                })?,
                user_id: row.try_get("user_id").map_err(|e| {
                    AppError::database(format!("Failed to parse user_id column: {e}"))
                })?,
                tenant_id: row.try_get("tenant_id").map_err(|e| {
                    AppError::database(format!("Failed to parse tenant_id column: {e}"))
                })?,
                scope: row.try_get("scope").map_err(|e| {
                    AppError::database(format!("Failed to parse scope column: {e}"))
                })?,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AppError::database(format!("Failed to parse expires_at column: {e}"))
                })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                revoked: row.try_get("revoked").map_err(|e| {
                    AppError::database(format!("Failed to parse revoked column: {e}"))
                })?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Revoke OAuth 2.0 refresh token
    ///
    /// The input token is HMAC-SHA256 hashed before querying.
    async fn revoke_oauth2_refresh_token(&self, token: &str) -> AppResult<()> {
        let token_hash = HasEncryption::hash_token_for_storage(self, token)?;

        sqlx::query("UPDATE oauth2_refresh_tokens SET revoked = true WHERE token = $1")
            .bind(&token_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Atomically consume OAuth 2.0 authorization code
    ///
    /// Implements atomic check-and-set using UPDATE...RETURNING
    /// to prevent TOCTOU race conditions in concurrent token exchange requests.
    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuth2AuthCode>> {
        let row = sqlx::query(
            "UPDATE oauth2_auth_codes
             SET used = true
             WHERE code = $1
               AND client_id = $2
               AND redirect_uri = $3
               AND used = false
               AND expires_at > $4
             RETURNING code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method"
        )
        .bind(code)
        .bind(client_id)
        .bind(redirect_uri)
        .bind(now)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        row.map_or_else(
            || Ok(None),
            |row| {
                use sqlx::Row;
                Ok(Some(OAuth2AuthCode {
                    code: row.get("code"),
                    client_id: row.get("client_id"),
                    user_id: row.get("user_id"),
                    tenant_id: row.get("tenant_id"),
                    redirect_uri: row.get("redirect_uri"),
                    scope: row.get("scope"),
                    expires_at: row.get("expires_at"),
                    used: row.get("used"),
                    state: row.get("state"),
                    code_challenge: row.get("code_challenge"),
                    code_challenge_method: row.get("code_challenge_method"),
                }))
            },
        )
    }

    /// Atomically consume OAuth 2.0 refresh token
    ///
    /// Implements atomic check-and-revoke using UPDATE...RETURNING
    /// to prevent TOCTOU race conditions in concurrent refresh requests.
    /// The input token is HMAC-SHA256 hashed before querying.
    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuth2RefreshToken>> {
        let token_hash = HasEncryption::hash_token_for_storage(self, token)?;

        let row = sqlx::query(
            "UPDATE oauth2_refresh_tokens
             SET revoked = true
             WHERE token = $1
               AND client_id = $2
               AND revoked = false
               AND expires_at > $3
             RETURNING token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked",
        )
        .bind(&token_hash)
        .bind(client_id)
        .bind(now)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(OAuth2RefreshToken {
                token: row.try_get("token").map_err(|e| {
                    AppError::database(format!("Failed to parse token column: {e}"))
                })?,
                client_id: row.try_get("client_id").map_err(|e| {
                    AppError::database(format!("Failed to parse client_id column: {e}"))
                })?,
                user_id: row.try_get("user_id").map_err(|e| {
                    AppError::database(format!("Failed to parse user_id column: {e}"))
                })?,
                tenant_id: row.try_get("tenant_id").map_err(|e| {
                    AppError::database(format!("Failed to parse tenant_id column: {e}"))
                })?,
                scope: row.try_get("scope").map_err(|e| {
                    AppError::database(format!("Failed to parse scope column: {e}"))
                })?,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AppError::database(format!("Failed to parse expires_at column: {e}"))
                })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                revoked: row.try_get("revoked").map_err(|e| {
                    AppError::database(format!("Failed to parse revoked column: {e}"))
                })?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Look up a refresh token by value (without `client_id` constraint)
    ///
    /// The input token is HMAC-SHA256 hashed before querying.
    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> AppResult<Option<OAuth2RefreshToken>> {
        let token_hash = HasEncryption::hash_token_for_storage(self, token)?;

        let row = sqlx::query(
            "SELECT token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
             FROM oauth2_refresh_tokens
             WHERE token = $1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(OAuth2RefreshToken {
                token: row.try_get("token").map_err(|e| {
                    AppError::database(format!("Failed to parse token column: {e}"))
                })?,
                client_id: row.try_get("client_id").map_err(|e| {
                    AppError::database(format!("Failed to parse client_id column: {e}"))
                })?,
                user_id: row.try_get("user_id").map_err(|e| {
                    AppError::database(format!("Failed to parse user_id column: {e}"))
                })?,
                tenant_id: row.try_get("tenant_id").map_err(|e| {
                    AppError::database(format!("Failed to parse tenant_id column: {e}"))
                })?,
                scope: row.try_get("scope").map_err(|e| {
                    AppError::database(format!("Failed to parse scope column: {e}"))
                })?,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AppError::database(format!("Failed to parse expires_at column: {e}"))
                })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                revoked: row.try_get("revoked").map_err(|e| {
                    AppError::database(format!("Failed to parse revoked column: {e}"))
                })?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Store `OAuth2` state for CSRF protection
    async fn store_oauth2_state(&self, state: &OAuth2State) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO oauth2_states (state, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, created_at, expires_at, used)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(&state.state)
        .bind(&state.client_id)
        .bind(state.user_id)
        .bind(&state.tenant_id)
        .bind(&state.redirect_uri)
        .bind(&state.scope)
        .bind(&state.code_challenge)
        .bind(&state.code_challenge_method)
        .bind(state.created_at)
        .bind(state.expires_at)
        .bind(state.used)
        .execute(&self.pool).await.map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Consume `OAuth2` state (atomically check and mark as used)
    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuth2State>> {
        let row = sqlx::query(
            "UPDATE oauth2_states
             SET used = true
             WHERE state = $1
               AND client_id = $2
               AND used = false
               AND expires_at > $3
             RETURNING state, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, created_at, expires_at, used",
        )
        .bind(state_value)
        .bind(client_id)
        .bind(now)
        .fetch_optional(&self.pool).await.map_err(|e| AppError::database(format!("Failed to fetch optional record: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(OAuth2State {
                state: row.try_get("state").map_err(|e| {
                    AppError::database(format!("Failed to parse state column: {e}"))
                })?,
                client_id: row.try_get("client_id").map_err(|e| {
                    AppError::database(format!("Failed to parse client_id column: {e}"))
                })?,
                user_id: row.try_get("user_id").map_err(|e| {
                    AppError::database(format!("Failed to parse user_id column: {e}"))
                })?,
                tenant_id: row.try_get("tenant_id").map_err(|e| {
                    AppError::database(format!("Failed to parse tenant_id column: {e}"))
                })?,
                redirect_uri: row.try_get("redirect_uri").map_err(|e| {
                    AppError::database(format!("Failed to parse redirect_uri column: {e}"))
                })?,
                scope: row.try_get("scope").map_err(|e| {
                    AppError::database(format!("Failed to parse scope column: {e}"))
                })?,
                code_challenge: row.try_get("code_challenge").map_err(|e| {
                    AppError::database(format!("Failed to parse code_challenge column: {e}"))
                })?,
                code_challenge_method: row.try_get("code_challenge_method").map_err(|e| {
                    AppError::database(format!("Failed to parse code_challenge_method column: {e}"))
                })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AppError::database(format!("Failed to parse expires_at column: {e}"))
                })?,
                used: row
                    .try_get("used")
                    .map_err(|e| AppError::database(format!("Failed to parse used column: {e}")))?,
            }))
        } else {
            Ok(None)
        }
    }

    // ================================
    // OAuth Client State (CSRF + PKCE)
    // ================================

    async fn store_oauth_client_state(&self, state: &OAuthClientState) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO oauth_client_states (state, provider, user_id, tenant_id, redirect_uri, scope, pkce_code_verifier, created_at, expires_at, used)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&state.state)
        .bind(&state.provider)
        .bind(state.user_id)
        .bind(&state.tenant_id)
        .bind(&state.redirect_uri)
        .bind(&state.scope)
        .bind(&state.pkce_code_verifier)
        .bind(state.created_at)
        .bind(state.expires_at)
        .bind(state.used)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store OAuth client state: {e}")))?;

        Ok(())
    }

    async fn consume_oauth_client_state(
        &self,
        state_value: &str,
        provider: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<OAuthClientState>> {
        let row = sqlx::query(
            "UPDATE oauth_client_states
             SET used = true
             WHERE state = $1
               AND provider = $2
               AND used = false
               AND expires_at > $3
             RETURNING state, provider, user_id, tenant_id, redirect_uri, scope, pkce_code_verifier, created_at, expires_at, used",
        )
        .bind(state_value)
        .bind(provider)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to consume OAuth client state: {e}")))?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(OAuthClientState {
                state: row.try_get("state").map_err(|e| {
                    AppError::database(format!("Failed to parse state column: {e}"))
                })?,
                provider: row.try_get("provider").map_err(|e| {
                    AppError::database(format!("Failed to parse provider column: {e}"))
                })?,
                user_id: row.try_get("user_id").map_err(|e| {
                    AppError::database(format!("Failed to parse user_id column: {e}"))
                })?,
                tenant_id: row.try_get("tenant_id").map_err(|e| {
                    AppError::database(format!("Failed to parse tenant_id column: {e}"))
                })?,
                redirect_uri: row.try_get("redirect_uri").map_err(|e| {
                    AppError::database(format!("Failed to parse redirect_uri column: {e}"))
                })?,
                scope: row.try_get("scope").map_err(|e| {
                    AppError::database(format!("Failed to parse scope column: {e}"))
                })?,
                pkce_code_verifier: row.try_get("pkce_code_verifier").map_err(|e| {
                    AppError::database(format!("Failed to parse pkce_code_verifier column: {e}"))
                })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    AppError::database(format!("Failed to parse created_at column: {e}"))
                })?,
                expires_at: row.try_get("expires_at").map_err(|e| {
                    AppError::database(format!("Failed to parse expires_at column: {e}"))
                })?,
                used: row
                    .try_get("used")
                    .map_err(|e| AppError::database(format!("Failed to parse used column: {e}")))?,
            }))
        } else {
            Ok(None)
        }
    }

    // ================================
    // Impersonation Session Management
    // ================================

    async fn create_impersonation_session(&self, session: &ImpersonationSession) -> AppResult<()> {
        let query = r"
            INSERT INTO impersonation_sessions (
                id, impersonator_id, target_user_id, reason,
                started_at, ended_at, is_active, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ";

        sqlx::query(query)
            .bind(&session.id)
            .bind(session.impersonator_id.to_string())
            .bind(session.target_user_id.to_string())
            .bind(&session.reason)
            .bind(session.started_at)
            .bind(session.ended_at)
            .bind(session.is_active)
            .bind(session.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to create impersonation session: {e}"))
            })?;

        Ok(())
    }

    async fn get_impersonation_session(
        &self,
        session_id: &str,
    ) -> AppResult<Option<ImpersonationSession>> {
        let query = r"
            SELECT id, impersonator_id, target_user_id, reason,
                   started_at, ended_at, is_active, created_at
            FROM impersonation_sessions WHERE id = $1
        ";

        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get impersonation session: {e}")))?;

        row.map(|r| shared::mappers::parse_impersonation_session_from_row(&r))
            .transpose()
    }

    async fn get_active_impersonation_session(
        &self,
        user_id: Uuid,
    ) -> AppResult<Option<ImpersonationSession>> {
        let query = r"
            SELECT id, impersonator_id, target_user_id, reason,
                   started_at, ended_at, is_active, created_at
            FROM impersonation_sessions
            WHERE (impersonator_id = $1 OR target_user_id = $2) AND is_active = true
            ORDER BY started_at DESC LIMIT 1
        ";

        let user_id_str = user_id.to_string();
        let row = sqlx::query(query)
            .bind(&user_id_str)
            .bind(&user_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to get active impersonation session: {e}"))
            })?;

        row.map(|r| shared::mappers::parse_impersonation_session_from_row(&r))
            .transpose()
    }

    async fn end_impersonation_session(&self, session_id: &str) -> AppResult<()> {
        let query = r"
            UPDATE impersonation_sessions
            SET is_active = false, ended_at = $1
            WHERE id = $2
        ";

        let ended_at = chrono::Utc::now();
        sqlx::query(query)
            .bind(ended_at)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to end impersonation session: {e}")))?;

        Ok(())
    }

    async fn end_all_impersonation_sessions(&self, impersonator_id: Uuid) -> AppResult<u64> {
        let query = r"
            UPDATE impersonation_sessions
            SET is_active = false, ended_at = $1
            WHERE impersonator_id = $2 AND is_active = true
        ";

        let ended_at = chrono::Utc::now();
        let result = sqlx::query(query)
            .bind(ended_at)
            .bind(impersonator_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to end impersonation sessions: {e}"))
            })?;

        Ok(result.rows_affected())
    }

    async fn list_impersonation_sessions(
        &self,
        impersonator_id: Option<Uuid>,
        target_user_id: Option<Uuid>,
        active_only: bool,
        limit: u32,
    ) -> AppResult<Vec<ImpersonationSession>> {
        use std::fmt::Write;

        // Build dynamic query based on filters
        let mut query = String::from(
            r"
            SELECT id, impersonator_id, target_user_id, reason,
                   started_at, ended_at, is_active, created_at
            FROM impersonation_sessions WHERE 1=1
            ",
        );

        let mut param_idx = 1u32;

        if impersonator_id.is_some() {
            let _ = write!(query, " AND impersonator_id = ${param_idx}");
            param_idx += 1;
        }
        if target_user_id.is_some() {
            let _ = write!(query, " AND target_user_id = ${param_idx}");
            param_idx += 1;
        }
        if active_only {
            query.push_str(" AND is_active = true");
        }
        let _ = write!(query, " ORDER BY started_at DESC LIMIT ${param_idx}");

        let mut sql_query = sqlx::query(&query);

        if let Some(id) = impersonator_id {
            sql_query = sql_query.bind(id.to_string());
        }
        if let Some(id) = target_user_id {
            sql_query = sql_query.bind(id.to_string());
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let limit_i32 = limit as i32;
        sql_query = sql_query.bind(limit_i32);

        let rows = sql_query.fetch_all(&self.pool).await.map_err(|e| {
            AppError::database(format!("Failed to list impersonation sessions: {e}"))
        })?;

        rows.iter()
            .map(shared::mappers::parse_impersonation_session_from_row)
            .collect()
    }

    // ================================
    // User MCP Token Management
    // ================================

    async fn create_user_mcp_token(
        &self,
        user_id: Uuid,
        request: &CreateUserMcpTokenRequest,
    ) -> AppResult<UserMcpTokenCreated> {
        let token_value = Self::generate_mcp_token();
        let token_hash = Self::hash_mcp_token(&token_value);
        let token_prefix = token_value.chars().take(12).collect::<String>();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let expires_at = request
            .expires_in_days
            .map(|days| now + chrono::Duration::days(i64::from(days)));

        sqlx::query(
            r"
            INSERT INTO user_mcp_tokens (
                id, user_id, name, token_hash, token_prefix,
                expires_at, last_used_at, usage_count, is_revoked, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, NULL, 0, false, $7)
            ",
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(&request.name)
        .bind(&token_hash)
        .bind(&token_prefix)
        .bind(expires_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create user MCP token: {e}")))?;

        let token = UserMcpToken {
            id,
            user_id,
            name: request.name.clone(),
            token_hash,
            token_prefix,
            expires_at,
            last_used_at: None,
            usage_count: 0,
            is_revoked: false,
            created_at: now,
        };

        Ok(UserMcpTokenCreated { token, token_value })
    }

    async fn validate_user_mcp_token(&self, token_value: &str) -> AppResult<Uuid> {
        use sqlx::Row;

        let token_hash = Self::hash_mcp_token(token_value);
        let token_prefix = token_value.chars().take(12).collect::<String>();

        let row = sqlx::query(
            r"
            SELECT id, user_id, expires_at, is_revoked
            FROM user_mcp_tokens
            WHERE token_prefix = $1 AND token_hash = $2
            ",
        )
        .bind(&token_prefix)
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to validate user MCP token: {e}")))?;

        let row = row.ok_or_else(|| AppError::auth_invalid("Invalid MCP token"))?;
        let is_revoked: bool = row.get("is_revoked");
        if is_revoked {
            return Err(AppError::auth_invalid("MCP token has been revoked"));
        }

        let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");
        if let Some(exp) = expires_at {
            if exp < chrono::Utc::now() {
                return Err(AppError::auth_invalid("MCP token has expired"));
            }
        }

        let token_id: String = row.get("id");
        self.update_user_mcp_token_usage(&token_id).await?;

        let user_id_str: String = row.get("user_id");
        Uuid::parse_str(&user_id_str)
            .map_err(|e| AppError::internal(format!("Failed to parse user_id UUID: {e}")))
    }

    async fn list_user_mcp_tokens(&self, user_id: Uuid) -> AppResult<Vec<UserMcpTokenInfo>> {
        use sqlx::Row;

        let rows = sqlx::query(
            r"
            SELECT id, name, token_prefix, expires_at, last_used_at,
                   usage_count, is_revoked, created_at
            FROM user_mcp_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list user MCP tokens: {e}")))?;
        rows.iter()
            .map(|row| {
                Ok(UserMcpTokenInfo {
                    id: row.get("id"),
                    name: row.get("name"),
                    token_prefix: row.get("token_prefix"),
                    expires_at: row.get("expires_at"),
                    last_used_at: row.get("last_used_at"),
                    usage_count: u32::try_from(row.get::<i32, _>("usage_count")).map_err(|e| {
                        AppError::internal(format!(
                            "Integer conversion failed for usage_count: {e}"
                        ))
                    })?,
                    is_revoked: row.get("is_revoked"),
                    created_at: row.get("created_at"),
                })
            })
            .collect()
    }

    async fn revoke_user_mcp_token(&self, token_id: &str, user_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            r"
            UPDATE user_mcp_tokens
            SET is_revoked = true
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(token_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to revoke user MCP token: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("MCP token not found or unauthorized"));
        }

        Ok(())
    }

    async fn get_user_mcp_token(
        &self,
        token_id: &str,
        user_id: Uuid,
    ) -> AppResult<Option<UserMcpToken>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, token_hash, token_prefix,
                   expires_at, last_used_at, usage_count, is_revoked, created_at
            FROM user_mcp_tokens
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(token_id)
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user MCP token: {e}")))?;

        row.map(|r| shared::mappers::parse_user_mcp_token_from_row(&r))
            .transpose()
    }

    async fn cleanup_expired_user_mcp_tokens(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r"
            UPDATE user_mcp_tokens
            SET is_revoked = true
            WHERE expires_at IS NOT NULL
            AND expires_at < $1
            AND is_revoked = false
            ",
        )
        .bind(chrono::Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to cleanup expired user MCP tokens: {e}"))
        })?;

        Ok(result.rows_affected())
    }

    // ================================
    // LLM Credentials Management
    // ================================

    async fn store_llm_credentials(&self, record: &LlmCredentialRecord) -> AppResult<()> {
        sqlx::query(
            r"
            INSERT INTO user_llm_credentials (
                id, tenant_id, user_id, provider, api_key_encrypted,
                base_url, default_model, is_active, created_at, updated_at, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT(tenant_id, user_id, provider) DO UPDATE SET
                api_key_encrypted = EXCLUDED.api_key_encrypted,
                base_url = EXCLUDED.base_url,
                default_model = EXCLUDED.default_model,
                is_active = EXCLUDED.is_active,
                updated_at = EXCLUDED.updated_at
            ",
        )
        .bind(record.id)
        .bind(record.tenant_id)
        .bind(record.user_id)
        .bind(&record.provider)
        .bind(&record.api_key_encrypted)
        .bind(&record.base_url)
        .bind(&record.default_model)
        .bind(record.is_active)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(record.created_by)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store LLM credentials: {e}")))?;

        Ok(())
    }

    async fn get_llm_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        provider: &str,
    ) -> AppResult<Option<LlmCredentialRecord>> {
        let row = if let Some(uid) = user_id {
            sqlx::query(
                r"
                SELECT id, tenant_id, user_id, provider, api_key_encrypted,
                       base_url, default_model, is_active, created_at, updated_at, created_by
                FROM user_llm_credentials
                WHERE tenant_id = $1 AND user_id = $2 AND provider = $3 AND is_active = TRUE
                ",
            )
            .bind(tenant_id)
            .bind(uid)
            .bind(provider)
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query(
                r"
                SELECT id, tenant_id, user_id, provider, api_key_encrypted,
                       base_url, default_model, is_active, created_at, updated_at, created_by
                FROM user_llm_credentials
                WHERE tenant_id = $1 AND user_id IS NULL AND provider = $2 AND is_active = TRUE
                ",
            )
            .bind(tenant_id)
            .bind(provider)
            .fetch_optional(&self.pool)
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to get LLM credentials: {e}")))?;

        Ok(row.map(|r| LlmCredentialRecord {
            id: r.get::<Uuid, _>("id"),
            tenant_id: r.get::<Uuid, _>("tenant_id"),
            user_id: r.get::<Option<Uuid>, _>("user_id"),
            provider: r.get::<String, _>("provider"),
            api_key_encrypted: r.get::<String, _>("api_key_encrypted"),
            base_url: r.get::<Option<String>, _>("base_url"),
            default_model: r.get::<Option<String>, _>("default_model"),
            is_active: r.get::<bool, _>("is_active"),
            created_at: r.get::<String, _>("created_at"),
            updated_at: r.get::<String, _>("updated_at"),
            created_by: r.get::<Uuid, _>("created_by"),
        }))
    }

    async fn list_llm_credentials(&self, tenant_id: Uuid) -> AppResult<Vec<LlmCredentialSummary>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, base_url, default_model, is_active, created_at, updated_at
            FROM user_llm_credentials
            WHERE tenant_id = $1
            ORDER BY provider, user_id
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list LLM credentials: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let user_id: Option<Uuid> = r.get("user_id");
                LlmCredentialSummary {
                    id: r.get::<Uuid, _>("id"),
                    user_id,
                    provider: r.get::<String, _>("provider"),
                    scope: if user_id.is_some() {
                        "user".to_owned()
                    } else {
                        "tenant".to_owned()
                    },
                    base_url: r.get::<Option<String>, _>("base_url"),
                    default_model: r.get::<Option<String>, _>("default_model"),
                    is_active: r.get::<bool, _>("is_active"),
                    created_at: r.get::<String, _>("created_at"),
                    updated_at: r.get::<String, _>("updated_at"),
                }
            })
            .collect())
    }

    async fn delete_llm_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        provider: &str,
    ) -> AppResult<bool> {
        let result = if let Some(uid) = user_id {
            sqlx::query(
                r"
                DELETE FROM user_llm_credentials
                WHERE tenant_id = $1 AND user_id = $2 AND provider = $3
                ",
            )
            .bind(tenant_id)
            .bind(uid)
            .bind(provider)
            .execute(&self.pool)
            .await
        } else {
            sqlx::query(
                r"
                DELETE FROM user_llm_credentials
                WHERE tenant_id = $1 AND user_id IS NULL AND provider = $2
                ",
            )
            .bind(tenant_id)
            .bind(provider)
            .execute(&self.pool)
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to delete LLM credentials: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_admin_config_override(
        &self,
        config_key: &str,
        tenant_id: Option<&str>,
    ) -> AppResult<Option<String>> {
        // Extract category from key (e.g., "llm.gemini_api_key" -> "llm_provider")
        let category = if config_key.starts_with("llm.") {
            "llm_provider"
        } else {
            config_key.split('.').next().unwrap_or("unknown")
        };

        let row = if let Some(tid) = tenant_id {
            sqlx::query(
                r"
                SELECT config_value
                FROM admin_config_overrides
                WHERE category = $1 AND config_key = $2 AND tenant_id = $3
                ",
            )
            .bind(category)
            .bind(config_key)
            .bind(tid)
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query(
                r"
                SELECT config_value
                FROM admin_config_overrides
                WHERE category = $1 AND config_key = $2 AND tenant_id IS NULL
                ",
            )
            .bind(category)
            .bind(config_key)
            .fetch_optional(&self.pool)
            .await
        }
        .map_err(|e| AppError::database(format!("Failed to get admin config override: {e}")))?;

        Ok(row.map(|r| r.get::<String, _>("config_value")))
    }

    // ================================
    // Encryption Interface (delegates to HasEncryption trait)
    // ================================

    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> AppResult<String> {
        shared::encryption::HasEncryption::encrypt_data_with_aad(self, data, aad)
    }

    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> AppResult<String> {
        shared::encryption::HasEncryption::decrypt_data_with_aad(self, encrypted, aad)
    }

    // ================================
    // Tool Selection (PostgreSQL implementation)
    // ================================

    async fn get_tool_catalog(&self) -> AppResult<Vec<ToolCatalogEntry>> {
        let rows = sqlx::query(
            r"
            SELECT id, tool_name, display_name, description, category,
                   is_enabled_by_default, requires_provider, min_plan,
                   created_at, updated_at
            FROM tool_catalog
            ORDER BY category, tool_name
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch tool catalog: {e}")))?;

        rows.iter().map(Self::map_pg_tool_catalog_row).collect()
    }

    async fn get_tool_catalog_entry(&self, tool_name: &str) -> AppResult<Option<ToolCatalogEntry>> {
        let row = sqlx::query(
            r"
            SELECT id, tool_name, display_name, description, category,
                   is_enabled_by_default, requires_provider, min_plan,
                   created_at, updated_at
            FROM tool_catalog
            WHERE tool_name = $1
            ",
        )
        .bind(tool_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch tool catalog entry: {e}")))?;

        row.as_ref().map(Self::map_pg_tool_catalog_row).transpose()
    }

    async fn get_tools_by_category(
        &self,
        category: ToolCategory,
    ) -> AppResult<Vec<ToolCatalogEntry>> {
        let rows = sqlx::query(
            r"
            SELECT id, tool_name, display_name, description, category,
                   is_enabled_by_default, requires_provider, min_plan,
                   created_at, updated_at
            FROM tool_catalog
            WHERE category = $1
            ORDER BY tool_name
            ",
        )
        .bind(category.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch tools by category: {e}")))?;

        rows.iter().map(Self::map_pg_tool_catalog_row).collect()
    }

    async fn get_tools_by_min_plan(&self, plan: TenantPlan) -> AppResult<Vec<ToolCatalogEntry>> {
        // Build list of acceptable plans based on hierarchy
        let acceptable_plans: Vec<&str> = match plan {
            TenantPlan::Starter => vec!["starter"],
            TenantPlan::Professional => vec!["starter", "professional"],
            TenantPlan::Enterprise => vec!["starter", "professional", "enterprise"],
        };

        let rows = sqlx::query(
            r"
            SELECT id, tool_name, display_name, description, category,
                   is_enabled_by_default, requires_provider, min_plan,
                   created_at, updated_at
            FROM tool_catalog
            WHERE min_plan = ANY($1)
            ORDER BY category, tool_name
            ",
        )
        .bind(&acceptable_plans)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch tools by plan: {e}")))?;

        rows.iter().map(Self::map_pg_tool_catalog_row).collect()
    }

    async fn get_tenant_tool_overrides(
        &self,
        tenant_id: Uuid,
    ) -> AppResult<Vec<TenantToolOverride>> {
        let rows = sqlx::query(
            r"
            SELECT id, tenant_id, tool_name, is_enabled, enabled_by_user_id,
                   reason, created_at, updated_at
            FROM tenant_tool_overrides
            WHERE tenant_id = $1
            ORDER BY tool_name
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch tenant tool overrides: {e}")))?;

        let overrides = rows
            .iter()
            .map(Self::map_pg_tenant_tool_override_row)
            .collect();
        Ok(overrides)
    }

    async fn get_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
    ) -> AppResult<Option<TenantToolOverride>> {
        let row = sqlx::query(
            r"
            SELECT id, tenant_id, tool_name, is_enabled, enabled_by_user_id,
                   reason, created_at, updated_at
            FROM tenant_tool_overrides
            WHERE tenant_id = $1 AND tool_name = $2
            ",
        )
        .bind(tenant_id)
        .bind(tool_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to fetch tenant tool override: {e}")))?;

        Ok(row.as_ref().map(Self::map_pg_tenant_tool_override_row))
    }

    async fn upsert_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
        is_enabled: bool,
        enabled_by_user_id: Option<Uuid>,
        reason: Option<String>,
    ) -> AppResult<TenantToolOverride> {
        let now = Utc::now();
        let id = Uuid::new_v4();

        // Use PostgreSQL upsert syntax
        sqlx::query(
            r"
            INSERT INTO tenant_tool_overrides (id, tenant_id, tool_name, is_enabled, enabled_by_user_id, reason, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
            ON CONFLICT(tenant_id, tool_name) DO UPDATE SET
                is_enabled = EXCLUDED.is_enabled,
                enabled_by_user_id = EXCLUDED.enabled_by_user_id,
                reason = EXCLUDED.reason,
                updated_at = EXCLUDED.updated_at
            ",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(tool_name)
        .bind(is_enabled)
        .bind(enabled_by_user_id)
        .bind(&reason)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to upsert tenant tool override: {e}")))?;

        // Fetch the resulting row (either inserted or updated)
        self.get_tenant_tool_override(tenant_id, tool_name)
            .await?
            .ok_or_else(|| AppError::internal("Failed to retrieve upserted tenant tool override"))
    }

    async fn delete_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM tenant_tool_overrides
            WHERE tenant_id = $1 AND tool_name = $2
            ",
        )
        .bind(tenant_id)
        .bind(tool_name)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete tenant tool override: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    async fn count_enabled_tools(&self, tenant_id: Uuid) -> AppResult<usize> {
        // Get tenant's plan to filter by plan restrictions
        let tenant = self.get_tenant_by_id(tenant_id).await?;
        let plan = TenantPlan::parse_str(&tenant.plan)
            .ok_or_else(|| AppError::internal(format!("Invalid tenant plan: {}", tenant.plan)))?;

        // Get tools available for this plan
        let catalog = self.get_tools_by_min_plan(plan).await?;
        let overrides = self.get_tenant_tool_overrides(tenant_id).await?;

        // Build override map
        let override_map: HashMap<String, bool> = overrides
            .into_iter()
            .map(|o| (o.tool_name, o.is_enabled))
            .collect();

        // Count enabled tools
        let count = catalog
            .iter()
            .filter(|tool| {
                override_map
                    .get(&tool.tool_name)
                    .copied()
                    .unwrap_or(tool.is_enabled_by_default)
            })
            .count();

        Ok(count)
    }

    async fn user_has_synthetic_activities(&self, user_id: Uuid) -> AppResult<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM synthetic_activities WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    // ================================
    // Provider Connections (PostgreSQL implementation)
    // ================================

    async fn register_provider_connection(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        connection_type: &ConnectionType,
        metadata: Option<&str>,
    ) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let conn_type_str = connection_type.as_str();

        sqlx::query(
            r"
            INSERT INTO provider_connections (id, user_id, tenant_id, provider, connection_type, connected_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT(user_id, tenant_id, provider) DO UPDATE SET
                connection_type = EXCLUDED.connection_type,
                connected_at = EXCLUDED.connected_at,
                metadata = EXCLUDED.metadata
            ",
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(tenant_id)
        .bind(provider)
        .bind(conn_type_str)
        .bind(now.to_rfc3339())
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_provider_connection(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<()> {
        sqlx::query(
            "DELETE FROM provider_connections WHERE user_id = $1 AND tenant_id = $2 AND provider = $3",
        )
        .bind(user_id.to_string())
        .bind(tenant_id)
        .bind(provider)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user_provider_connections(
        &self,
        user_id: Uuid,
        tenant_id: Option<&str>,
    ) -> AppResult<Vec<ProviderConnection>> {
        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, connection_type, connected_at, metadata
                FROM provider_connections
                WHERE user_id = $1 AND tenant_id = $2
                ORDER BY connected_at DESC
                ",
            )
            .bind(user_id.to_string())
            .bind(tid)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r"
                SELECT id, user_id, tenant_id, provider, connection_type, connected_at, metadata
                FROM provider_connections
                WHERE user_id = $1
                ORDER BY connected_at DESC
                ",
            )
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await?
        };

        let mut connections = Vec::with_capacity(rows.len());
        for row in rows {
            let conn_type_str: String = row.get("connection_type");
            let connected_at_str: String = row.get("connected_at");
            let connected_at = DateTime::parse_from_rfc3339(&connected_at_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

            let user_id_from_db: String = row.get("user_id");
            let parsed_user_id = Uuid::parse_str(&user_id_from_db).unwrap_or_else(|_| Uuid::nil());

            connections.push(ProviderConnection {
                id: row.get("id"),
                user_id: parsed_user_id,
                tenant_id: row.get("tenant_id"),
                provider: row.get("provider"),
                connection_type: ConnectionType::from_str_value(&conn_type_str)
                    .unwrap_or(ConnectionType::Manual),
                connected_at,
                metadata: row.get("metadata"),
            });
        }

        Ok(connections)
    }

    async fn is_provider_connected(&self, user_id: Uuid, provider: &str) -> AppResult<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM provider_connections WHERE user_id = $1 AND provider = $2",
        )
        .bind(user_id.to_string())
        .bind(provider)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    // ================================
    // Chat Conversations & Messages (PostgreSQL implementation)
    // ================================

    async fn chat_create_conversation(
        &self,
        user_id: &str,
        tenant_id: &str,
        title: &str,
        model: &str,
        system_prompt: Option<&str>,
    ) -> AppResult<ConversationRecord> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r"
            INSERT INTO chat_conversations (id, user_id, tenant_id, title, model, system_prompt, total_tokens, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $7)
            ",
        )
        .bind(&id)
        .bind(parse_uuid(user_id)?)
        .bind(tenant_id)
        .bind(title)
        .bind(model)
        .bind(system_prompt)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create conversation: {e}")))?;

        Ok(ConversationRecord {
            id,
            user_id: user_id.to_owned(),
            tenant_id: tenant_id.to_owned(),
            title: title.to_owned(),
            model: model.to_owned(),
            system_prompt: system_prompt.map(ToOwned::to_owned),
            total_tokens: 0,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        })
    }

    async fn chat_get_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<Option<ConversationRecord>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, tenant_id, title, model, system_prompt, total_tokens, created_at, updated_at
            FROM chat_conversations
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(conversation_id)
        .bind(parse_uuid(user_id)?)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get conversation: {e}")))?;

        Ok(row.map(|r| {
            let created_at: DateTime<Utc> = r.get("created_at");
            let updated_at: DateTime<Utc> = r.get("updated_at");
            let user_id_uuid: Uuid = r.get("user_id");

            ConversationRecord {
                id: r.get("id"),
                user_id: user_id_uuid.to_string(),
                tenant_id: r.get("tenant_id"),
                title: r.get("title"),
                model: r.get("model"),
                system_prompt: r.get("system_prompt"),
                total_tokens: r.get("total_tokens"),
                created_at: created_at.to_rfc3339(),
                updated_at: updated_at.to_rfc3339(),
            }
        }))
    }

    async fn chat_list_conversations(
        &self,
        user_id: &str,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ConversationSummary>> {
        let rows = sqlx::query(
            r"
            SELECT c.id, c.title, c.model, c.total_tokens, c.created_at, c.updated_at,
                   COUNT(m.id) as message_count
            FROM chat_conversations c
            LEFT JOIN chat_messages m ON m.conversation_id = c.id
            WHERE c.user_id = $1 AND c.tenant_id = $2
            GROUP BY c.id
            ORDER BY c.updated_at DESC
            LIMIT $3 OFFSET $4
            ",
        )
        .bind(parse_uuid(user_id)?)
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to list conversations: {e}")))?;

        let summaries = rows
            .into_iter()
            .map(|r| {
                let created_at: DateTime<Utc> = r.get("created_at");
                let updated_at: DateTime<Utc> = r.get("updated_at");

                ConversationSummary {
                    id: r.get("id"),
                    title: r.get("title"),
                    model: r.get("model"),
                    message_count: r.get("message_count"),
                    total_tokens: r.get("total_tokens"),
                    created_at: created_at.to_rfc3339(),
                    updated_at: updated_at.to_rfc3339(),
                }
            })
            .collect();

        Ok(summaries)
    }

    async fn chat_update_conversation_title(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: &str,
        title: &str,
    ) -> AppResult<bool> {
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE chat_conversations
            SET title = $1, updated_at = $2
            WHERE id = $3 AND user_id = $4 AND tenant_id = $5
            ",
        )
        .bind(title)
        .bind(now)
        .bind(conversation_id)
        .bind(parse_uuid(user_id)?)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update conversation title: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    async fn chat_delete_conversation(
        &self,
        conversation_id: &str,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r"
            DELETE FROM chat_conversations
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            ",
        )
        .bind(conversation_id)
        .bind(parse_uuid(user_id)?)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete conversation: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    async fn chat_add_message(
        &self,
        conversation_id: &str,
        user_id: &str,
        role: &str,
        content: &str,
        token_count: Option<u32>,
        finish_reason: Option<&str>,
    ) -> AppResult<MessageRecord> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let user_uuid = parse_uuid(user_id)?;

        // Insert message only if the conversation belongs to the user
        let result = sqlx::query(
            r"
            INSERT INTO chat_messages (id, conversation_id, role, content, token_count, finish_reason, created_at)
            SELECT $1, $2, $3, $4, $5, $6, $7
            WHERE EXISTS (
                SELECT 1 FROM chat_conversations WHERE id = $2 AND user_id = $8
            )
            ",
        )
        .bind(&id)
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(token_count.map(i64::from))
        .bind(finish_reason)
        .bind(now)
        .bind(user_uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to add message: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(
                "Conversation not found or access denied",
            ));
        }

        // Update conversation's updated_at and total_tokens (ownership already verified above)
        if let Some(tokens) = token_count {
            sqlx::query(
                r"
                UPDATE chat_conversations
                SET updated_at = $1, total_tokens = total_tokens + $2
                WHERE id = $3 AND user_id = $4
                ",
            )
            .bind(now)
            .bind(i64::from(tokens))
            .bind(conversation_id)
            .bind(user_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to update conversation tokens: {e}"))
            })?;
        } else {
            sqlx::query(
                r"
                UPDATE chat_conversations
                SET updated_at = $1
                WHERE id = $2 AND user_id = $3
                ",
            )
            .bind(now)
            .bind(conversation_id)
            .bind(user_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to update conversation timestamp: {e}"))
            })?;
        }

        Ok(MessageRecord {
            id,
            conversation_id: conversation_id.to_owned(),
            role: role.to_owned(),
            content: content.to_owned(),
            token_count: token_count.map(i64::from),
            finish_reason: finish_reason.map(ToOwned::to_owned),
            created_at: now.to_rfc3339(),
        })
    }

    async fn chat_get_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<MessageRecord>> {
        let rows = sqlx::query(
            r"
            SELECT m.id, m.conversation_id, m.role, m.content, m.token_count, m.finish_reason, m.created_at
            FROM chat_messages m
            JOIN chat_conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1 AND c.user_id = $2
            ORDER BY m.created_at ASC
            ",
        )
        .bind(conversation_id)
        .bind(parse_uuid(user_id)?)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get messages: {e}")))?;

        let messages = rows
            .into_iter()
            .map(|r| {
                let created_at: DateTime<Utc> = r.get("created_at");

                MessageRecord {
                    id: r.get("id"),
                    conversation_id: r.get("conversation_id"),
                    role: r.get("role"),
                    content: r.get("content"),
                    token_count: r.get("token_count"),
                    finish_reason: r.get("finish_reason"),
                    created_at: created_at.to_rfc3339(),
                }
            })
            .collect();

        Ok(messages)
    }

    async fn chat_get_recent_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
        limit: i64,
    ) -> AppResult<Vec<MessageRecord>> {
        let rows = sqlx::query(
            r"
            SELECT m.id, m.conversation_id, m.role, m.content, m.token_count, m.finish_reason, m.created_at
            FROM chat_messages m
            JOIN chat_conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1 AND c.user_id = $2
            ORDER BY m.created_at DESC
            LIMIT $3
            ",
        )
        .bind(conversation_id)
        .bind(parse_uuid(user_id)?)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get recent messages: {e}")))?;

        // Reverse to get chronological order
        let mut messages: Vec<MessageRecord> = rows
            .into_iter()
            .map(|r| {
                let created_at: DateTime<Utc> = r.get("created_at");

                MessageRecord {
                    id: r.get("id"),
                    conversation_id: r.get("conversation_id"),
                    role: r.get("role"),
                    content: r.get("content"),
                    token_count: r.get("token_count"),
                    finish_reason: r.get("finish_reason"),
                    created_at: created_at.to_rfc3339(),
                }
            })
            .collect();
        messages.reverse();

        Ok(messages)
    }

    async fn chat_get_message_count(&self, conversation_id: &str, user_id: &str) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar(
            r"
            SELECT COUNT(*)
            FROM chat_messages m
            JOIN chat_conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1 AND c.user_id = $2
            ",
        )
        .bind(conversation_id)
        .bind(parse_uuid(user_id)?)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get message count: {e}")))?;

        Ok(count)
    }

    async fn chat_delete_all_user_conversations(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> AppResult<i64> {
        let result = sqlx::query(
            r"
            DELETE FROM chat_conversations
            WHERE user_id = $1 AND tenant_id = $2
            ",
        )
        .bind(parse_uuid(user_id)?)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete user conversations: {e}")))?;

        #[allow(clippy::cast_possible_wrap)]
        Ok(result.rows_affected() as i64)
    }

    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        created_by: &str,
    ) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(1);

        sqlx::query(
            r"
            INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(token_hash)
        .bind(expires_at.to_rfc3339())
        .bind(created_by)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store password reset token: {e}")))?;

        Ok(id)
    }

    async fn consume_password_reset_token(&self, token_hash: &str) -> AppResult<Uuid> {
        let now = Utc::now().to_rfc3339();

        let row = sqlx::query(
            r"
            UPDATE password_reset_tokens
            SET used_at = $1
            WHERE token_hash = $2
              AND used_at IS NULL
              AND expires_at > $1
            RETURNING user_id
            ",
        )
        .bind(&now)
        .bind(token_hash)
        .fetch_optional(&self.pool)
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

    async fn invalidate_user_reset_tokens(&self, user_id: Uuid) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r"
            UPDATE password_reset_tokens
            SET used_at = $1
            WHERE user_id = $2
              AND used_at IS NULL
            ",
        )
        .bind(&now)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to invalidate reset tokens: {e}")))?;

        Ok(())
    }
}

impl PostgresDatabase {
    /// Generate a new MCP token with secure random bytes
    fn generate_mcp_token() -> String {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        format!("pmcp_{}", URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Hash a token for storage
    fn hash_mcp_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Update token usage statistics
    async fn update_user_mcp_token_usage(&self, token_id: &str) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE user_mcp_tokens
            SET last_used_at = $1, usage_count = usage_count + 1
            WHERE id = $2
            ",
        )
        .bind(chrono::Utc::now())
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update user MCP token usage: {e}")))?;

        Ok(())
    }

    /// Convert database row to `UserOAuthToken` with decryption
    ///
    /// SECURITY: Decrypts OAuth tokens from database storage (AES-256-GCM with AAD)
    fn row_to_user_oauth_token(&self, row: &PgRow) -> AppResult<UserOAuthToken> {
        use sqlx::Row;

        let user_id: uuid::Uuid = row
            .try_get("user_id")
            .map_err(|e| AppError::database(format!("Failed to parse user_id column: {e}")))?;
        let tenant_id: String = row
            .try_get("tenant_id")
            .map_err(|e| AppError::database(format!("Failed to parse tenant_id column: {e}")))?;
        let provider: String = row
            .try_get("provider")
            .map_err(|e| AppError::database(format!("Failed to parse provider column: {e}")))?;

        // Decrypt access token
        let encrypted_access_token: String = row
            .try_get("access_token")
            .map_err(|e| AppError::database(format!("Failed to parse access_token column: {e}")))?;
        let access_token = shared::encryption::decrypt_oauth_token(
            self,
            &encrypted_access_token,
            &tenant_id,
            user_id,
            &provider,
        )?;

        // Decrypt refresh token (optional)
        let refresh_token = row
            .try_get::<Option<String>, _>("refresh_token")
            .map_err(|e| AppError::database(format!("Failed to parse refresh_token column: {e}")))?
            .map(|encrypted_rt| {
                shared::encryption::decrypt_oauth_token(
                    self,
                    &encrypted_rt,
                    &tenant_id,
                    user_id,
                    &provider,
                )
            })
            .transpose()?;

        Ok(UserOAuthToken {
            id: row
                .try_get("id")
                .map_err(|e| AppError::database(format!("Failed to parse id column: {e}")))?,
            user_id,
            tenant_id,
            provider,
            access_token,
            refresh_token,
            token_type: row.try_get("token_type").map_err(|e| {
                AppError::database(format!("Failed to parse token_type column: {e}"))
            })?,
            expires_at: row.try_get("expires_at").map_err(|e| {
                AppError::database(format!("Failed to parse expires_at column: {e}"))
            })?,
            scope: row.try_get("scope").ok(),
            created_at: row.try_get("created_at").map_err(|e| {
                AppError::database(format!("Failed to parse created_at column: {e}"))
            })?,
            updated_at: row.try_get("updated_at").map_err(|e| {
                AppError::database(format!("Failed to parse updated_at column: {e}"))
            })?,
        })
    }

    async fn create_users_table(&self) -> AppResult<()> {
        // OAuth tokens are stored in user_oauth_tokens table, not here
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT NOT NULL,
                tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
                tenant_id TEXT,
                is_active BOOLEAN NOT NULL DEFAULT true,
                user_status TEXT NOT NULL DEFAULT 'pending' CHECK (user_status IN ('pending', 'active', 'suspended')),
                is_admin BOOLEAN NOT NULL DEFAULT false,
                role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('super_admin', 'admin', 'user')),
                approved_by UUID REFERENCES users(id),
                approved_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                last_active TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                firebase_uid TEXT,
                auth_provider TEXT NOT NULL DEFAULT 'email'
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create users table: {e}")))?;

        // Create unique index for Firebase UID lookups (enforces uniqueness for non-null values)
        sqlx::query(
            r"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_users_firebase_uid
            ON users(firebase_uid)
            WHERE firebase_uid IS NOT NULL
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create firebase_uid index: {e}")))?;

        // Create index for auth provider queries
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_users_auth_provider ON users(auth_provider)
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create auth_provider index: {e}")))?;

        Ok(())
    }

    async fn create_user_profiles_table(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create user_profiles table: {e}")))?;
        Ok(())
    }

    async fn create_goals_table(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create goals table: {e}")))?;
        Ok(())
    }

    async fn create_insights_table(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create insights table: {e}")))?;
        Ok(())
    }

    async fn create_api_keys_tables(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create api_keys table: {e}")))?;

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
        .await
        .map_err(|e| AppError::database(format!("Failed to create api_key_usage table: {e}")))?;
        Ok(())
    }

    async fn create_a2a_tables(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create a2a_clients table: {e}")))?;

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
        .await
        .map_err(|e| AppError::database(format!("Failed to create a2a_sessions table: {e}")))?;

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
        .await
        .map_err(|e| AppError::database(format!("Failed to create a2a_tasks table: {e}")))?;

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
        .await
        .map_err(|e| AppError::database(format!("Failed to create a2a_usage table: {e}")))?;
        Ok(())
    }

    async fn create_admin_tables(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create admin_tokens table: {e}")))?;

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
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create admin_token_usage table: {e}"))
        })?;

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
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create admin_provisioned_keys table: {e}"
            ))
        })?;
        Ok(())
    }

    async fn create_jwt_usage_table(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create jwt_usage table: {e}")))?;
        Ok(())
    }

    /// Create OAuth notifications table for MCP resource delivery
    async fn create_oauth_notifications_table(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create oauth_notifications table: {e}"))
        })?;

        // Create indices for efficient queries
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_id
            ON oauth_notifications (user_id)
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_oauth_notifications_user_id: {e}"
            ))
        })?;

        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_oauth_notifications_user_unread
            ON oauth_notifications (user_id, read_at)
            WHERE read_at IS NULL
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_oauth_notifications_user_unread: {e}"
            ))
        })?;

        Ok(())
    }

    /// Create RSA keypairs table for JWT signing key persistence
    async fn create_rsa_keypairs_table(&self) -> AppResult<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS rsa_keypairs (
                kid TEXT PRIMARY KEY,
                private_key_pem TEXT NOT NULL,
                public_key_pem TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT false,
                key_size_bits INTEGER NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create rsa_keypairs table: {e}")))?;

        // Create index for active key lookup
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_rsa_keypairs_active
            ON rsa_keypairs (is_active)
            WHERE is_active = true
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_rsa_keypairs_active: {e}"
            ))
        })?;

        Ok(())
    }

    /// Creates complete multi-tenant database schema with all required tables
    ///
    /// JUSTIFICATION for `#[allow(clippy::too_many_lines)]`:
    /// - Creates 6 interdependent tables in a single atomic migration
    /// - Each table has comprehensive schema: constraints, foreign keys, indexes, defaults
    /// - Splitting into separate functions obscures the complete schema structure
    /// - Database migrations benefit from having the full DDL in one location for review
    /// - Tables: `tenants`, `tenant_oauth_credentials`, `tenant_users`, `tenant_provider_usage`,
    ///   `oauth_apps`, `authorization_codes`, `user_oauth_tokens`
    #[allow(clippy::too_many_lines)]
    async fn create_tenant_tables(&self) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Failed to create tenants table: {e}")))?;

        // Create tenant_oauth_credentials table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_oauth_credentials (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                provider VARCHAR(50) NOT NULL,
                client_id VARCHAR(255) NOT NULL,
                client_secret_encrypted TEXT NOT NULL,
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
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create tenant_oauth_credentials table: {e}"
            ))
        })?;

        // Create tenant_users table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role VARCHAR(50) DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'billing', 'member')),
                invited_at TIMESTAMPTZ NOT NULL,
                joined_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, user_id)
            )
            "
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create tenant_users table: {e}")))?;

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
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create tenant_provider_usage table: {e}"))
        })?;

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
        .await
        .map_err(|e| AppError::database(format!("Failed to create oauth_apps table: {e}")))?;

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
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create authorization_codes table: {e}"))
        })?;

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
                last_sync TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, tenant_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create user_oauth_tokens table: {e}"))
        })?;

        Ok(())
    }

    /// Create tool selection tables for per-tenant MCP tool configuration
    async fn create_tool_selection_tables(&self) -> AppResult<()> {
        // Create tool_catalog table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tool_catalog (
                id VARCHAR(255) PRIMARY KEY,
                tool_name VARCHAR(255) NOT NULL UNIQUE,
                display_name VARCHAR(255) NOT NULL,
                description TEXT NOT NULL,
                category VARCHAR(50) NOT NULL CHECK (category IN (
                    'fitness', 'analysis', 'goals', 'nutrition',
                    'recipes', 'sleep', 'configuration', 'connections'
                )),
                is_enabled_by_default BOOLEAN NOT NULL DEFAULT true,
                requires_provider VARCHAR(50),
                min_plan VARCHAR(50) NOT NULL DEFAULT 'starter' CHECK (min_plan IN ('starter', 'professional', 'enterprise')),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create tool_catalog table: {e}")))?;

        // Create tenant_tool_overrides table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_tool_overrides (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                tool_name VARCHAR(255) NOT NULL REFERENCES tool_catalog(tool_name) ON DELETE CASCADE,
                is_enabled BOOLEAN NOT NULL,
                enabled_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
                reason TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, tool_name)
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create tenant_tool_overrides table: {e}"))
        })?;

        // Create indexes for tool selection tables
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tool_catalog_category ON tool_catalog(category)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_tool_catalog_category: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tool_catalog_min_plan ON tool_catalog(min_plan)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_tool_catalog_min_plan: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tenant_tool_overrides_tenant ON tenant_tool_overrides(tenant_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_tenant_tool_overrides_tenant: {e}"
            ))
        })?;

        // Seed tool_catalog with default tools
        self.seed_tool_catalog().await?;

        Ok(())
    }

    /// Create chat tables for AI conversation storage
    async fn create_chat_tables(&self) -> AppResult<()> {
        // Create chat_conversations table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS chat_conversations (
                id VARCHAR(255) PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                tenant_id VARCHAR(255) NOT NULL,
                title TEXT NOT NULL,
                model VARCHAR(255) NOT NULL DEFAULT 'gemini-2.0-flash-exp',
                system_prompt TEXT,
                total_tokens BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!("Failed to create chat_conversations table: {e}"))
        })?;

        // Create chat_messages table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS chat_messages (
                id VARCHAR(255) PRIMARY KEY,
                conversation_id VARCHAR(255) NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
                role VARCHAR(50) NOT NULL CHECK (role IN ('system', 'user', 'assistant')),
                content TEXT NOT NULL,
                token_count BIGINT,
                finish_reason VARCHAR(50),
                created_at TIMESTAMPTZ NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create chat_messages table: {e}")))?;

        // Create indexes for chat tables
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chat_conversations_user ON chat_conversations(user_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_chat_conversations_user: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chat_conversations_tenant ON chat_conversations(tenant_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_chat_conversations_tenant: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chat_conversations_updated ON chat_conversations(updated_at DESC)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_chat_conversations_updated: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation ON chat_messages(conversation_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_chat_messages_conversation: {e}"
            ))
        })?;

        Ok(())
    }

    /// Seed the `tool_catalog` table with default tools
    async fn seed_tool_catalog(&self) -> AppResult<()> {
        // Check if tools already exist
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_catalog")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to count tool catalog: {e}")))?;

        if count > 0 {
            return Ok(());
        }

        // Seed tool catalog with all ToolId variants (same as SQLite migration)
        let tools: Vec<ToolCatalogSeedEntry<'_>> = vec![
            (
                "tc-001",
                "get_activities",
                "Get Activities",
                "Get user fitness activities with optional filtering and limits",
                "fitness",
                true,
                None,
                "starter",
            ),
            (
                "tc-002",
                "get_athlete",
                "Get Athlete Profile",
                "Get user athlete profile and basic information",
                "fitness",
                true,
                None,
                "starter",
            ),
            (
                "tc-003",
                "get_stats",
                "Get Statistics",
                "Get user performance statistics and metrics",
                "fitness",
                true,
                None,
                "starter",
            ),
            (
                "tc-004",
                "analyze_activity",
                "Analyze Activity",
                "Analyze a specific activity with detailed performance insights",
                "analysis",
                true,
                None,
                "starter",
            ),
            (
                "tc-005",
                "get_activity_intelligence",
                "Activity Intelligence",
                "Get AI-powered intelligence analysis for an activity",
                "analysis",
                true,
                None,
                "starter",
            ),
            (
                "tc-006",
                "get_connection_status",
                "Connection Status",
                "Check OAuth connection status for fitness providers",
                "connections",
                true,
                None,
                "starter",
            ),
            (
                "tc-007",
                "connect_provider",
                "Connect Provider",
                "Connect to a fitness data provider via OAuth",
                "connections",
                true,
                None,
                "starter",
            ),
            (
                "tc-008",
                "disconnect_provider",
                "Disconnect Provider",
                "Disconnect user from a fitness data provider",
                "connections",
                true,
                None,
                "starter",
            ),
            (
                "tc-009",
                "set_goal",
                "Set Goal",
                "Set a new fitness goal for the user",
                "goals",
                true,
                None,
                "starter",
            ),
            (
                "tc-010",
                "suggest_goals",
                "Suggest Goals",
                "Get AI-suggested fitness goals based on activity history",
                "goals",
                true,
                None,
                "starter",
            ),
            (
                "tc-011",
                "analyze_goal_feasibility",
                "Goal Feasibility",
                "Analyze whether a goal is achievable given current fitness level",
                "goals",
                true,
                None,
                "professional",
            ),
            (
                "tc-012",
                "track_progress",
                "Track Progress",
                "Track progress towards fitness goals",
                "goals",
                true,
                None,
                "starter",
            ),
            (
                "tc-013",
                "calculate_metrics",
                "Calculate Metrics",
                "Calculate custom fitness metrics and performance indicators",
                "analysis",
                true,
                None,
                "starter",
            ),
            (
                "tc-014",
                "analyze_performance_trends",
                "Performance Trends",
                "Analyze performance trends over time",
                "analysis",
                true,
                None,
                "professional",
            ),
            (
                "tc-015",
                "compare_activities",
                "Compare Activities",
                "Compare two activities for performance analysis",
                "analysis",
                true,
                None,
                "starter",
            ),
            (
                "tc-016",
                "detect_patterns",
                "Detect Patterns",
                "Detect patterns and insights in activity data",
                "analysis",
                true,
                None,
                "professional",
            ),
            (
                "tc-017",
                "generate_recommendations",
                "Generate Recommendations",
                "Generate personalized training recommendations",
                "analysis",
                true,
                None,
                "professional",
            ),
            (
                "tc-018",
                "calculate_fitness_score",
                "Fitness Score",
                "Calculate overall fitness score based on recent activities",
                "analysis",
                true,
                None,
                "starter",
            ),
            (
                "tc-019",
                "predict_performance",
                "Predict Performance",
                "Predict future performance based on training patterns",
                "analysis",
                true,
                None,
                "enterprise",
            ),
            (
                "tc-020",
                "analyze_training_load",
                "Training Load",
                "Analyze training load and recovery metrics",
                "analysis",
                true,
                None,
                "professional",
            ),
            (
                "tc-021",
                "get_configuration_catalog",
                "Configuration Catalog",
                "Get the complete configuration catalog with all available parameters",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-022",
                "get_configuration_profiles",
                "Configuration Profiles",
                "Get available configuration profiles (Research, Elite, Recreational, etc.)",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-023",
                "get_user_configuration",
                "Get User Config",
                "Get current user configuration settings and overrides",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-024",
                "update_user_configuration",
                "Update User Config",
                "Update user configuration parameters and session overrides",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-025",
                "calculate_personalized_zones",
                "Personalized Zones",
                "Calculate personalized training zones based on user VO2 max",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-026",
                "validate_configuration",
                "Validate Config",
                "Validate configuration parameters against safety rules",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-027",
                "analyze_sleep_quality",
                "Sleep Quality",
                "Analyze sleep quality from Fitbit/Garmin data using NSF/AASM guidelines",
                "sleep",
                true,
                None,
                "professional",
            ),
            (
                "tc-028",
                "calculate_recovery_score",
                "Recovery Score",
                "Calculate holistic recovery score combining TSB, sleep quality, and HRV",
                "sleep",
                true,
                None,
                "professional",
            ),
            (
                "tc-029",
                "suggest_rest_day",
                "Rest Day Suggestion",
                "AI-powered rest day recommendation based on recovery indicators",
                "sleep",
                true,
                None,
                "professional",
            ),
            (
                "tc-030",
                "track_sleep_trends",
                "Sleep Trends",
                "Track sleep patterns and correlate with performance over time",
                "sleep",
                true,
                None,
                "professional",
            ),
            (
                "tc-031",
                "optimize_sleep_schedule",
                "Sleep Schedule",
                "Optimize sleep duration based on training load and recovery needs",
                "sleep",
                true,
                None,
                "enterprise",
            ),
            (
                "tc-032",
                "get_fitness_config",
                "Get Fitness Config",
                "Get user fitness configuration settings including heart rate zones",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-033",
                "set_fitness_config",
                "Set Fitness Config",
                "Save user fitness configuration settings for zones and thresholds",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-034",
                "list_fitness_configs",
                "List Fitness Configs",
                "List all available fitness configuration names for the user",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-035",
                "delete_fitness_config",
                "Delete Fitness Config",
                "Delete a specific fitness configuration by name",
                "configuration",
                true,
                None,
                "starter",
            ),
            (
                "tc-036",
                "calculate_daily_nutrition",
                "Daily Nutrition",
                "Calculate daily calorie and macronutrient needs using Mifflin-St Jeor BMR formula",
                "nutrition",
                true,
                None,
                "starter",
            ),
            (
                "tc-037",
                "get_nutrient_timing",
                "Nutrient Timing",
                "Get optimal pre/post-workout nutrition recommendations following ISSN guidelines",
                "nutrition",
                true,
                None,
                "professional",
            ),
            (
                "tc-038",
                "search_food",
                "Search Food",
                "Search USDA FoodData Central database for foods by name/description",
                "nutrition",
                true,
                None,
                "starter",
            ),
            (
                "tc-039",
                "get_food_details",
                "Food Details",
                "Get detailed nutritional information for a specific food from USDA database",
                "nutrition",
                true,
                None,
                "starter",
            ),
            (
                "tc-040",
                "analyze_meal_nutrition",
                "Meal Nutrition",
                "Analyze total calories and macronutrients for a meal of multiple foods",
                "nutrition",
                true,
                None,
                "starter",
            ),
            (
                "tc-041",
                "get_recipe_constraints",
                "Recipe Constraints",
                "Get macro targets for LLM recipe generation by training phase",
                "recipes",
                true,
                None,
                "starter",
            ),
            (
                "tc-042",
                "validate_recipe",
                "Validate Recipe",
                "Validate recipe nutrition against USDA and calculate macros",
                "recipes",
                true,
                None,
                "starter",
            ),
            (
                "tc-043",
                "save_recipe",
                "Save Recipe",
                "Save validated recipe with cached nutrition data",
                "recipes",
                true,
                None,
                "starter",
            ),
            (
                "tc-044",
                "list_recipes",
                "List Recipes",
                "List saved recipes with optional meal timing filter",
                "recipes",
                true,
                None,
                "starter",
            ),
            (
                "tc-045",
                "get_recipe",
                "Get Recipe",
                "Get a specific recipe by ID",
                "recipes",
                true,
                None,
                "starter",
            ),
            (
                "tc-046",
                "delete_recipe",
                "Delete Recipe",
                "Delete a recipe from collection",
                "recipes",
                true,
                None,
                "starter",
            ),
            (
                "tc-047",
                "search_recipes",
                "Search Recipes",
                "Search recipes by name, tags, or description",
                "recipes",
                true,
                None,
                "starter",
            ),
        ];

        for (
            id,
            tool_name,
            display_name,
            description,
            category,
            is_enabled,
            requires_provider,
            min_plan,
        ) in tools
        {
            sqlx::query(
                r"
                INSERT INTO tool_catalog (id, tool_name, display_name, description, category, is_enabled_by_default, requires_provider, min_plan)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (tool_name) DO NOTHING
                ",
            )
            .bind(id)
            .bind(tool_name)
            .bind(display_name)
            .bind(description)
            .bind(category)
            .bind(is_enabled)
            .bind(requires_provider)
            .bind(min_plan)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to seed tool {tool_name}: {e}")))?;
        }

        Ok(())
    }

    async fn create_user_indexes(&self) -> AppResult<()> {
        // User and profile indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to create index idx_users_email: {e}"))
            })?;

        Ok(())
    }

    async fn create_api_key_indexes(&self) -> AppResult<()> {
        // API key indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to create index idx_api_keys_user_id: {e}"))
            })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_api_key_id ON api_key_usage(api_key_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_api_key_usage_api_key_id: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_timestamp ON api_key_usage(timestamp)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_api_key_usage_timestamp: {e}"
            ))
        })?;

        Ok(())
    }

    async fn create_a2a_indexes(&self) -> AppResult<()> {
        // A2A indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_clients_user_id ON a2a_clients(user_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!(
                    "Failed to create index idx_a2a_clients_user_id: {e}"
                ))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_client_id ON a2a_usage(client_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!(
                    "Failed to create index idx_a2a_usage_client_id: {e}"
                ))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_timestamp ON a2a_usage(timestamp)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!(
                    "Failed to create index idx_a2a_usage_timestamp: {e}"
                ))
            })?;

        Ok(())
    }

    async fn create_admin_token_indexes(&self) -> AppResult<()> {
        // Admin token indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_service ON admin_tokens(service_name)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_admin_tokens_service: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_prefix ON admin_tokens(token_prefix)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_admin_tokens_prefix: {e}"
            ))
        })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_usage_token_id ON admin_token_usage(admin_token_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create index idx_admin_usage_token_id: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_usage_timestamp ON admin_token_usage(timestamp)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_admin_usage_timestamp: {e}"
            ))
        })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_provisioned_token ON admin_provisioned_keys(admin_token_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create index idx_admin_provisioned_token: {e}")))?;

        Ok(())
    }

    async fn create_jwt_usage_indexes(&self) -> AppResult<()> {
        // JWT usage indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_user_id ON jwt_usage(user_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to create index idx_jwt_usage_user_id: {e}"))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_timestamp ON jwt_usage(timestamp)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!(
                    "Failed to create index idx_jwt_usage_timestamp: {e}"
                ))
            })?;

        Ok(())
    }

    async fn create_tenant_indexes(&self) -> AppResult<()> {
        // Tenant indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_oauth_credentials_tenant_provider ON tenant_oauth_credentials(tenant_id, provider)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create index idx_tenant_oauth_credentials_tenant_provider: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_tenant_users_tenant: {e}"
            ))
        })?;

        // UserOAuthToken indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_user ON user_oauth_tokens(user_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::database(format!(
                "Failed to create index idx_user_oauth_tokens_user: {e}"
            ))
        })?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_tenant_provider ON user_oauth_tokens(tenant_id, provider)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create index idx_user_oauth_tokens_tenant_provider: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to create index idx_tenant_users_user: {e}"))
            })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_usage_date ON tenant_provider_usage(tenant_id, provider, usage_date)")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to create index idx_tenant_usage_date: {e}")))?;

        Ok(())
    }

    async fn create_indexes(&self) -> AppResult<()> {
        self.create_user_indexes().await?;
        self.create_api_key_indexes().await?;
        self.create_a2a_indexes().await?;
        self.create_admin_token_indexes().await?;
        self.create_jwt_usage_indexes().await?;
        self.create_tenant_indexes().await?;

        Ok(())
    }
}

// Implement encryption support for PostgreSQL (harmonize with SQLite security)
impl shared::encryption::HasEncryption for PostgresDatabase {
    /// Encrypt data using AES-256-GCM with Additional Authenticated Data
    ///
    /// This brings `PostgreSQL` to security parity with `SQLite`, which already
    /// encrypts OAuth tokens at rest.
    ///
    /// # Security
    /// - Uses AES-256-GCM (AEAD cipher) via ring crate
    /// - Generates unique 96-bit nonce per encryption
    /// - Binds AAD to prevent cross-tenant token reuse
    /// - Output: base64(nonce || ciphertext || `auth_tag`)
    fn encrypt_data_with_aad(&self, data: &str, aad_context: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Generate unique nonce (96 bits for GCM)
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|e| AppError::database(format!("Failed to generate nonce: {e:?}")))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| AppError::database(format!("Failed to create encryption key: {e:?}")))?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt data with AAD binding
        let mut data_bytes = data.as_bytes().to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        key.seal_in_place_append_tag(nonce, aad, &mut data_bytes)
            .map_err(|e| AppError::database(format!("Encryption failed: {e:?}")))?;

        // Combine nonce and encrypted data, then base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(data_bytes);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    /// Decrypt data using AES-256-GCM with Additional Authenticated Data
    ///
    /// Reverses `encrypt_data_with_aad`. AAD context must match or decryption fails.
    ///
    /// # Security
    /// - Verifies AAD matches (prevents token context switching)
    /// - Authenticates ciphertext hasn't been tampered
    /// - Fails safely on any mismatch/corruption
    fn decrypt_data_with_aad(&self, encrypted_data: &str, aad_context: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decode from base64
        let combined = general_purpose::STANDARD
            .decode(encrypted_data)
            .map_err(|e| AppError::database(format!("Failed to decode base64 data: {e}")))?;

        if combined.len() < 12 {
            return Err(AppError::database(
                "Invalid encrypted data: too short".to_owned(),
            ));
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes
                .try_into()
                .map_err(|e| AppError::database(format!("Invalid nonce size: {e:?}")))?,
        );

        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| AppError::database(format!("Failed to create decryption key: {e:?}")))?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt data with AAD verification
        let mut decrypted_data = encrypted_bytes.to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        let decrypted = key
            .open_in_place(nonce, aad, &mut decrypted_data)
            .map_err(|e| {
                AppError::database(format!(
                    "Decryption failed (possible AAD mismatch or tampered data): {e:?}"
                ))
            })?;

        String::from_utf8(decrypted.to_vec()).map_err(|e| {
            AppError::database(format!("Failed to convert decrypted data to string: {e}"))
        })
    }

    /// Compute HMAC-SHA256 of a token for secure storage
    ///
    /// Used for refresh tokens where we need deterministic lookups but don't
    /// need to recover the original value.
    fn hash_token_for_storage(&self, token: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::hmac;

        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.encryption_key);
        let tag = hmac::sign(&key, token.as_bytes());
        Ok(general_purpose::STANDARD.encode(tag.as_ref()))
    }
}
