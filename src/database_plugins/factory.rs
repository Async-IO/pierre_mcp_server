// ABOUTME: Database factory and provider abstraction for multi-database support
// ABOUTME: Provides unified interface for SQLite and PostgreSQL with runtime database selection
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Database factory for creating database providers
//!
//! This module provides automatic database type detection and creation
//! based on connection strings.

#![allow(missing_docs)]

use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::errors::AppError;
use crate::models::UserOAuthApp;
use crate::rate_limiting::JwtUsage;
use anyhow::Result;
use chrono::{DateTime, Utc};
use tracing::{debug, info};
use uuid::Uuid;

#[cfg(feature = "postgresql")]
use super::postgres::PostgresDatabase;
#[cfg(feature = "postgresql")]
use crate::database::repositories::ApiKeyRepository;
// Phase 3: Use crate::database::Database directly (eliminates sqlite.rs wrapper)
use crate::database::Database as SqliteDatabase;

/// Supported database types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    /// `SQLite` embedded database
    SQLite,
    /// `PostgreSQL` database server
    PostgreSQL,
}

/// Database instance wrapper that delegates to the appropriate implementation
#[derive(Clone)]
pub enum Database {
    /// `SQLite` database instance
    SQLite(SqliteDatabase),
    /// `PostgreSQL` database instance (requires postgresql feature)
    #[cfg(feature = "postgresql")]
    PostgreSQL(PostgresDatabase),
}

impl Database {
    /// Get a descriptive string for the current database backend
    #[must_use]
    pub const fn backend_info(&self) -> &'static str {
        match self {
            Self::SQLite(_) => "SQLite (Local Development)",
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_) => "PostgreSQL (Cloud-Ready)",
        }
    }

    /// Get the database type enum
    #[must_use]
    pub const fn database_type(&self) -> DatabaseType {
        match self {
            Self::SQLite(_) => DatabaseType::SQLite,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_) => DatabaseType::PostgreSQL,
        }
    }

    /// Get detailed database information for logging/monitoring
    #[must_use]
    pub fn info_summary(&self) -> String {
        match self {
            Self::SQLite(_) => "Database Backend: SQLite\n\
                     Type: Embedded file-based database\n\
                     Use Case: Local development and testing\n\
                     Features: Zero-configuration, serverless, lightweight"
                .to_owned(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_) => "Database Backend: PostgreSQL\n\
                     Type: Client-server relational database\n\
                     Use Case: Production and cloud deployments\n\
                     Features: Concurrent access, advanced queries, scalability"
                .to_owned(),
        }
    }

    /// Create a new database instance based on the connection string (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database URL format is unsupported or invalid
    /// - `PostgreSQL` feature is not enabled when `PostgreSQL` URL is provided
    /// - Database connection fails
    /// - Database initialization or migration fails
    /// - Encryption key is invalid
    pub async fn new_impl(
        database_url: &str,
        encryption_key: Vec<u8>,
        #[cfg(feature = "postgresql")] pool_config: &crate::config::environment::PostgresPoolConfig,
    ) -> Result<Self> {
        debug!("Detecting database type from URL: {}", database_url);
        let db_type = detect_database_type(database_url)?;
        info!("Detected database type: {:?}", db_type);

        match db_type {
            DatabaseType::SQLite => {
                info!("Initializing SQLite database");
                let db = SqliteDatabase::new(database_url, encryption_key).await?;
                info!("SQLite database initialized successfully");
                Ok(Self::SQLite(db))
            }
            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => {
                info!("Initializing PostgreSQL database");
                let db = PostgresDatabase::new(database_url, encryption_key, pool_config).await?;
                info!("PostgreSQL database initialized successfully");
                Ok(Self::PostgreSQL(db))
            }
            #[cfg(not(feature = "postgresql"))]
            DatabaseType::PostgreSQL => {
                let err_msg =
                    "PostgreSQL support not enabled. Enable the 'postgresql' feature flag.";
                tracing::error!("{}", err_msg);
                Err(AppError::config(err_msg).into())
            }
        }
    }

    /// Create a new database instance based on the connection string (public API)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database URL format is unsupported or invalid
    /// - `PostgreSQL` feature is not enabled when `PostgreSQL` URL is provided
    /// - Database connection fails
    /// - Database initialization or migration fails
    /// - Encryption key is invalid
    pub async fn new(
        database_url: &str,
        encryption_key: Vec<u8>,
        #[cfg(feature = "postgresql")] pool_config: &crate::config::environment::PostgresPoolConfig,
    ) -> Result<Self> {
        #[cfg(feature = "postgresql")]
        {
            Self::new_impl(database_url, encryption_key, pool_config).await
        }
        #[cfg(not(feature = "postgresql"))]
        {
            Self::new_impl(database_url, encryption_key).await
        }
    }

    /// Run database migrations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any migration fails
    /// - Database connection is lost during migration
    /// - Insufficient database permissions
    pub async fn migrate(&self) -> Result<()> {
        match self {
            Self::SQLite(db) => db.migrate().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.migrate().await,
        }
    }

    // ================================
    // Repository Pattern Accessors
    // ================================

    /// Get `UserRepository` for user account management
    #[must_use]
    pub fn users(&self) -> crate::database::repositories::UserRepositoryImpl {
        match self {
            Self::SQLite(db) => db.users(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::UserRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `OAuthTokenRepository` for OAuth token storage
    #[must_use]
    pub fn oauth_tokens(&self) -> crate::database::repositories::OAuthTokenRepositoryImpl {
        match self {
            Self::SQLite(db) => db.oauth_tokens(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::OAuthTokenRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `ApiKeyRepository` for API key management
    #[must_use]
    pub fn api_keys(&self) -> crate::database::repositories::ApiKeyRepositoryImpl {
        match self {
            Self::SQLite(db) => db.api_keys(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::ApiKeyRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `UsageRepository` for usage tracking and analytics
    #[must_use]
    pub fn usage(&self) -> crate::database::repositories::UsageRepositoryImpl {
        match self {
            Self::SQLite(db) => db.usage(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::UsageRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `A2ARepository` for Agent-to-Agent management
    #[must_use]
    pub fn a2a(&self) -> crate::database::repositories::A2ARepositoryImpl {
        match self {
            Self::SQLite(db) => db.a2a(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::A2ARepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `ProfileRepository` for user profiles and goals
    #[must_use]
    pub fn profiles(&self) -> crate::database::repositories::ProfileRepositoryImpl {
        match self {
            Self::SQLite(db) => db.profiles(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::ProfileRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `InsightRepository` for AI-generated insights
    #[must_use]
    pub fn insights(&self) -> crate::database::repositories::InsightRepositoryImpl {
        match self {
            Self::SQLite(db) => db.insights(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::InsightRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `AdminRepository` for admin token management
    #[must_use]
    pub fn admin(&self) -> crate::database::repositories::AdminRepositoryImpl {
        match self {
            Self::SQLite(db) => db.admin(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::AdminRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `TenantRepository` for multi-tenant management
    #[must_use]
    pub fn tenants(&self) -> crate::database::repositories::TenantRepositoryImpl {
        match self {
            Self::SQLite(db) => db.tenants(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::TenantRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `OAuth2ServerRepository` for OAuth 2.0 server functionality
    #[must_use]
    pub fn oauth2_server(&self) -> crate::database::repositories::OAuth2ServerRepositoryImpl {
        match self {
            Self::SQLite(db) => db.oauth2_server(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::OAuth2ServerRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `SecurityRepository` for key rotation and audit
    #[must_use]
    pub fn security(&self) -> crate::database::repositories::SecurityRepositoryImpl {
        match self {
            Self::SQLite(db) => db.security(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::SecurityRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `NotificationRepository` for OAuth notifications
    #[must_use]
    pub fn notifications(&self) -> crate::database::repositories::NotificationRepositoryImpl {
        match self {
            Self::SQLite(db) => db.notifications(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::NotificationRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Get `FitnessConfigRepository` for fitness configuration management
    #[must_use]
    pub fn fitness_configs(&self) -> crate::database::repositories::FitnessConfigRepositoryImpl {
        match self {
            Self::SQLite(db) => db.fitness_configs(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_db) => {
                crate::database::repositories::FitnessConfigRepositoryImpl::new(self.clone())
            }
        }
    }

    /// Create a new user in the database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User data validation fails
    /// - Database constraint violations (e.g., duplicate email)
    /// - SQL execution fails
    /// - Database connection issues
    #[tracing::instrument(skip(self, user), fields(db_operation = "create_user", email = %user.email))]
    pub async fn create_user(&self, user: &crate::models::User) -> Result<uuid::Uuid> {
        match self {
            Self::SQLite(db) => db.create_user(user).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_user(user).await,
        }
    }

    /// Get a user by their UUID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    #[tracing::instrument(skip(self), fields(db_operation = "get_user"))]
    pub async fn get_user(&self, user_id: uuid::Uuid) -> Result<Option<crate::models::User>> {
        match self {
            Self::SQLite(db) => db.get_user(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user(user_id).await,
        }
    }

    /// Get a user by their email address
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    /// - Email format validation fails
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<crate::models::User>> {
        match self {
            Self::SQLite(db) => db.get_user_by_email(email).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_by_email(email).await,
        }
    }

    /// Get a user by email, returning an error if not found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User with email is not found
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_by_email_required(&self, email: &str) -> Result<crate::models::User> {
        match self {
            Self::SQLite(db) => db.get_user_by_email_required(email).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_by_email_required(email).await,
        }
    }

    /// Update user's last active timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_last_active(&self, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_last_active(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_last_active(user_id).await,
        }
    }

    /// Get total count of users in the database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Count aggregation fails
    /// - Database connection issues
    pub async fn get_user_count(&self) -> Result<i64> {
        match self {
            Self::SQLite(db) => db.get_user_count().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_count().await,
        }
    }

    /// Get all users with a specific status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_users_by_status(&self, status: &str) -> Result<Vec<crate::models::User>> {
        match self {
            Self::SQLite(db) => db.get_users_by_status(status).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_users_by_status(status).await,
        }
    }

    /// Get users with a specific status using cursor-based pagination
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Cursor parsing fails
    /// - Database connection issues
    pub async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<crate::models::User>> {
        match self {
            Self::SQLite(db) => db.get_users_by_status_cursor(status, params).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_users_by_status_cursor(status, params).await,
        }
    }

    /// Update user status and record approval information
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Database update fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn update_user_status(
        &self,
        user_id: uuid::Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<crate::models::User> {
        match self {
            Self::SQLite(db) => {
                db.update_user_status(user_id, new_status, admin_token_id)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.update_user_status(user_id, new_status, admin_token_id)
                    .await
            }
        }
    }

    /// Update user's tenant ID for multi-tenant support
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_user_tenant_id(&self, user_id: uuid::Uuid, tenant_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_user_tenant_id(user_id, tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_user_tenant_id(user_id, tenant_id).await,
        }
    }

    /// Create or update a user profile with the provided data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operation fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn upsert_user_profile(
        &self,
        user_id: uuid::Uuid,
        profile_data: serde_json::Value,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.upsert_user_profile(user_id, profile_data).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.upsert_user_profile(user_id, profile_data).await,
        }
    }

    /// Get user profile data by user ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_profile(&self, user_id: uuid::Uuid) -> Result<Option<serde_json::Value>> {
        match self {
            Self::SQLite(db) => db.get_user_profile(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_profile(user_id).await,
        }
    }

    /// Create a new goal for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Goal data validation fails
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_goal(
        &self,
        user_id: uuid::Uuid,
        goal_data: serde_json::Value,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => db.create_goal(user_id, goal_data).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_goal(user_id, goal_data).await,
        }
    }

    /// Get all goals for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_goals(&self, user_id: uuid::Uuid) -> Result<Vec<serde_json::Value>> {
        match self {
            Self::SQLite(db) => db.get_user_goals(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_goals(user_id).await,
        }
    }

    /// Update the progress value for a specific goal
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Goal does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_goal_progress(goal_id, current_value).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_goal_progress(goal_id, current_value).await,
        }
    }

    /// Get user configuration data by user ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        match self {
            Self::SQLite(db) => db.get_user_configuration(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_configuration(user_id).await,
        }
    }

    /// Save user configuration data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Database update fails
    /// - Database connection issues
    pub async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.save_user_configuration(user_id, config_json).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.save_user_configuration(user_id, config_json).await,
        }
    }

    /// Store a new insight for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Insight data validation fails
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_insight(
        &self,
        user_id: uuid::Uuid,
        insight_data: serde_json::Value,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => db.store_insight(user_id, insight_data).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_insight(user_id, insight_data).await,
        }
    }

    /// Get insights for a user with optional filtering
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_insights(
        &self,
        user_id: uuid::Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<serde_json::Value>> {
        match self {
            Self::SQLite(db) => db.get_user_insights(user_id, insight_type, limit).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_insights(user_id, insight_type, limit).await,
        }
    }

    /// Create a new API key in the database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key data validation fails
    /// - Database constraint violations (e.g., duplicate key)
    /// - SQL execution fails
    /// - Database connection issues
    pub async fn create_api_key(&self, api_key: &crate::api_keys::ApiKey) -> Result<()> {
        match self {
            Self::SQLite(db) => db.create_api_key(api_key).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_api_key(api_key).await,
        }
    }

    /// Get an API key by its prefix and hash
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_api_key_by_prefix(
        &self,
        prefix: &str,
        hash: &str,
    ) -> Result<Option<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => db.get_api_key_by_prefix(prefix, hash).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_api_key_by_prefix(prefix, hash).await,
        }
    }

    /// Get all API keys for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_api_keys(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => db.get_user_api_keys(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_api_keys(user_id).await,
        }
    }

    /// Update the last used timestamp for an API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_api_key_last_used(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_api_key_last_used(api_key_id).await,
        }
    }

    /// Deactivate an API key for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key does not exist or doesn't belong to user
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_api_key(&self, api_key_id: &str, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_api_key(api_key_id, user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_api_key(api_key_id, user_id).await,
        }
    }

    /// Get an API key by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_api_key_by_id(
        &self,
        api_key_id: &str,
    ) -> Result<Option<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => db.get_api_key_by_id(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_api_key_by_id(api_key_id).await,
        }
    }

    /// Get API keys with optional filtering by user email, active status, limit and offset
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_api_keys_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => {
                // User email filtering requires user lookup (deferred to repository layer)
                // Currently filtering by active status only
                let _ = user_email; // Suppress unused variable warning
                db.get_api_keys_filtered(
                    None,                 // user_id (requires email lookup)
                    None,                 // tier filter
                    Some(active_only),    // is_active filter
                    limit.unwrap_or(100), // limit with default
                    offset.unwrap_or(0),  // offset with default
                )
                .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                // PostgreSQL uses the new repository pattern via ApiKeyRepository
                db.api_keys()
                    .list_filtered(user_email, active_only, limit, offset)
                    .await
                    .map_err(Into::into)
            }
        }
    }

    /// Remove expired API keys from the database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        match self {
            Self::SQLite(db) => db.cleanup_expired_api_keys().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.cleanup_expired_api_keys().await,
        }
    }

    /// Get all expired API keys
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_expired_api_keys(&self) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => db.get_expired_api_keys().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_expired_api_keys().await,
        }
    }

    /// Record API key usage statistics
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data validation fails
    /// - Database connection issues
    pub async fn record_api_key_usage(&self, usage: &crate::api_keys::ApiKeyUsage) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_api_key_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_api_key_usage(usage).await,
        }
    }

    /// Get current usage count for an API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Count aggregation fails
    /// - Database connection issues
    pub async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        match self {
            Self::SQLite(db) => db.get_api_key_current_usage(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_api_key_current_usage(api_key_id).await,
        }
    }

    /// Get detailed usage statistics for an API key within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Statistics aggregation fails
    /// - Date range validation fails
    /// - Database connection issues
    pub async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::api_keys::ApiKeyUsageStats> {
        match self {
            Self::SQLite(db) => {
                db.get_api_key_usage_stats(api_key_id, start_date, end_date)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_api_key_usage_stats(api_key_id, start_date, end_date)
                    .await
            }
        }
    }

    /// Record JWT token usage for rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data validation fails
    /// - Database connection issues
    pub async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_jwt_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_jwt_usage(usage).await,
        }
    }

    /// Get current JWT usage count for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Count aggregation fails
    /// - Database connection issues
    pub async fn get_jwt_current_usage(&self, user_id: uuid::Uuid) -> Result<u32> {
        match self {
            Self::SQLite(db) => db.get_jwt_current_usage(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_jwt_current_usage(user_id).await,
        }
    }

    /// Get request logs with optional filters
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Date range validation fails
    /// - Database connection issues
    pub async fn get_request_logs(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        match self {
            Self::SQLite(db) => {
                db.get_request_logs_with_filters(
                    api_key_id,
                    start_time,
                    end_time,
                    status_filter,
                    tool_filter,
                )
                .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_request_logs_with_filters(
                    api_key_id,
                    start_time,
                    end_time,
                    status_filter,
                    tool_filter,
                )
                .await
            }
        }
    }

    /// Get system-wide statistics (user count, active API keys)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Count aggregation fails
    /// - Database connection issues
    pub async fn get_system_stats(&self) -> Result<(u64, u64)> {
        match self {
            Self::SQLite(db) => db.get_system_stats().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_system_stats().await,
        }
    }

    /// Create a new A2A (Agent-to-Agent) client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client data validation fails
    /// - Database constraint violations
    /// - Secret encryption fails
    /// - SQL execution fails
    /// - Database connection issues
    pub async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => {
                db.create_a2a_client(client, client_secret, api_key_id)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.create_a2a_client(client, client_secret, api_key_id)
                    .await
            }
        }
    }

    /// Get an A2A client by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client(client_id).await,
        }
    }

    /// Get A2A client by associated API key ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client_by_api_key_id(
        &self,
        api_key_id: &str,
    ) -> Result<Option<A2AClient>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_by_api_key_id(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_by_api_key_id(api_key_id).await,
        }
    }

    /// Get A2A client by name
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_by_name(name).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_by_name(name).await,
        }
    }

    /// List all A2A clients for a specific user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_a2a_clients(&self, user_id: &uuid::Uuid) -> Result<Vec<A2AClient>> {
        match self {
            Self::SQLite(db) => db.list_a2a_clients(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_a2a_clients(user_id).await,
        }
    }

    /// Deactivate an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_a2a_client(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_a2a_client(client_id).await,
        }
    }

    /// Get A2A client credentials (client secret and hash)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Database query execution fails
    /// - Secret decryption fails
    /// - Database connection issues
    pub async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_credentials(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_credentials(client_id).await,
        }
    }

    /// Invalidate all active sessions for an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.invalidate_a2a_client_sessions(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.invalidate_a2a_client_sessions(client_id).await,
        }
    }

    /// Deactivate all API keys associated with an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_client_api_keys(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_client_api_keys(client_id).await,
        }
    }

    /// Create a new A2A session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client does not exist
    /// - Session token generation fails
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&uuid::Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => {
                db.create_a2a_session(client_id, user_id, granted_scopes, expires_in_hours)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.create_a2a_session(client_id, user_id, granted_scopes, expires_in_hours)
                    .await
            }
        }
    }

    /// Get an A2A session by token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        match self {
            Self::SQLite(db) => db.get_a2a_session(session_token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_session(session_token).await,
        }
    }

    /// Update last activity timestamp for an A2A session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_a2a_session_activity(session_token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_a2a_session_activity(session_token).await,
        }
    }

    /// Get all active sessions for an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        match self {
            Self::SQLite(db) => db.get_active_a2a_sessions(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_active_a2a_sessions(client_id).await,
        }
    }

    /// Create a new A2A task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client or session does not exist
    /// - Task data validation fails
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &serde_json::Value,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => {
                db.create_a2a_task(client_id, session_id, task_type, input_data)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.create_a2a_task(client_id, session_id, task_type, input_data)
                    .await
            }
        }
    }

    /// Get an A2A task by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        match self {
            Self::SQLite(db) => db.get_a2a_task(task_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_task(task_id).await,
        }
    }

    /// List A2A tasks with optional filters
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Pagination parameters are invalid
    /// - Database connection issues
    pub async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        match self {
            Self::SQLite(db) => {
                db.list_a2a_tasks(client_id, status_filter, limit, offset)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.list_a2a_tasks(client_id, status_filter, limit, offset)
                    .await
            }
        }
    }

    /// Update A2A task status and result
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task does not exist
    /// - Database update fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&serde_json::Value>,
        error: Option<&str>,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.update_a2a_task_status(task_id, status, result, error)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.update_a2a_task_status(task_id, status, result, error)
                    .await
            }
        }
    }

    /// Record A2A client usage statistics
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data validation fails
    /// - Database connection issues
    pub async fn record_a2a_usage(&self, usage: &crate::database::A2AUsage) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_a2a_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_a2a_usage(usage).await,
        }
    }

    /// Get current usage count for an A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Count aggregation fails
    /// - Database connection issues
    pub async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_current_usage(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_current_usage(client_id).await,
        }
    }

    /// Get A2A usage statistics within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Statistics aggregation fails
    /// - Date range validation fails
    /// - Database connection issues
    pub async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        match self {
            Self::SQLite(db) => {
                db.get_a2a_usage_stats(client_id, start_date, end_date)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_a2a_usage_stats(client_id, start_date, end_date)
                    .await
            }
        }
    }

    /// Get A2A client usage history for specified number of days
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Statistics aggregation fails
    /// - Database connection issues
    pub async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, u32, u32)>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_usage_history(client_id, days).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_usage_history(client_id, days).await,
        }
    }

    /// Get last synchronization timestamp for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_provider_last_sync(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        match self {
            Self::SQLite(db) => db.get_provider_last_sync(user_id, provider).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_provider_last_sync(user_id, provider).await,
        }
    }

    /// Update last synchronization timestamp for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_provider_last_sync(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        sync_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.update_provider_last_sync(user_id, provider, sync_time)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.update_provider_last_sync(user_id, provider, sync_time)
                    .await
            }
        }
    }

    /// Get tool usage analytics for a user within a time range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Statistics aggregation fails
    /// - Date range validation fails
    /// - Database connection issues
    pub async fn get_top_tools_analysis(
        &self,
        user_id: uuid::Uuid,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        match self {
            Self::SQLite(db) => {
                db.get_top_tools_analysis(user_id, start_time, end_time)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_top_tools_analysis(user_id, start_time, end_time)
                    .await
            }
        }
    }

    // ================================
    // Admin Token Management
    // ================================

    /// Create a new admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token generation fails
    /// - Database insertion fails
    /// - Token data validation fails
    /// - Hash generation fails
    /// - Database connection issues
    pub async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        match self {
            Self::SQLite(db) => {
                db.create_admin_token(request, admin_jwt_secret, jwks_manager)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.create_admin_token(request, admin_jwt_secret, jwks_manager)
                    .await
            }
        }
    }

    /// Get admin token by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        match self {
            Self::SQLite(db) => db.get_admin_token_by_id(token_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_admin_token_by_id(token_id).await,
        }
    }

    /// Get admin token by its prefix
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        match self {
            Self::SQLite(db) => db.get_admin_token_by_prefix(token_prefix).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_admin_token_by_prefix(token_prefix).await,
        }
    }

    /// List all admin tokens with optional inactive filter
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        match self {
            Self::SQLite(db) => db.list_admin_tokens(include_inactive).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_admin_tokens(include_inactive).await,
        }
    }

    /// Deactivate an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_admin_token(token_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_admin_token(token_id).await,
        }
    }

    /// Update last used timestamp and IP for an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_admin_token_last_used(token_id, ip_address).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_admin_token_last_used(token_id, ip_address).await,
        }
    }

    /// Record admin token usage event
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data validation fails
    /// - Database connection issues
    pub async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_admin_token_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_admin_token_usage(usage).await,
        }
    }

    /// Get admin token usage history within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Date range validation fails
    /// - Database connection issues
    pub async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>> {
        match self {
            Self::SQLite(db) => {
                db.get_admin_token_usage_history(token_id, start_date, end_date)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_admin_token_usage_history(token_id, start_date, end_date)
                    .await
            }
        }
    }

    /// Record an API key provisioned by an admin token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data validation fails
    /// - Database connection issues
    pub async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.record_admin_provisioned_key(
                    admin_token_id,
                    api_key_id,
                    user_email,
                    tier,
                    rate_limit_requests,
                    rate_limit_period,
                )
                .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.record_admin_provisioned_key(
                    admin_token_id,
                    api_key_id,
                    user_email,
                    tier,
                    rate_limit_requests,
                    rate_limit_period,
                )
                .await
            }
        }
    }

    /// Get API keys provisioned by admin tokens within a date range
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Date range validation fails
    /// - Database connection issues
    pub async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<serde_json::Value>> {
        match self {
            Self::SQLite(db) => {
                db.get_admin_provisioned_keys(admin_token_id, start_date, end_date)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_admin_provisioned_keys(admin_token_id, start_date, end_date)
                    .await
            }
        }
    }

    // Multi-tenant management implementations
    /// Create a new tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant data validation fails
    /// - Database constraint violations (e.g., duplicate slug)
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        match self {
            Self::SQLite(db) => db.create_tenant(tenant).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_tenant(tenant).await,
        }
    }

    /// Get a tenant by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant does not exist
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_by_id(&self, tenant_id: uuid::Uuid) -> Result<crate::models::Tenant> {
        match self {
            Self::SQLite(db) => db.get_tenant_by_id(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_by_id(tenant_id).await,
        }
    }

    /// Get a tenant by its slug
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant does not exist
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        match self {
            Self::SQLite(db) => db.get_tenant_by_slug(slug).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_by_slug(slug).await,
        }
    }

    /// List all tenants that a user has access to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_tenants_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<crate::models::Tenant>> {
        match self {
            Self::SQLite(db) => db.list_tenants_for_user(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_tenants_for_user(user_id).await,
        }
    }

    /// Store OAuth credentials for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Credentials validation fails
    /// - Credentials encryption fails
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_tenant_oauth_credentials(credentials).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_tenant_oauth_credentials(credentials).await,
        }
    }

    /// Get all OAuth providers configured for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Credentials decryption fails
    /// - Database connection issues
    pub async fn get_tenant_oauth_providers(
        &self,
        tenant_id: uuid::Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        match self {
            Self::SQLite(db) => db.get_tenant_oauth_providers(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_oauth_providers(tenant_id).await,
        }
    }

    /// Get OAuth credentials for a specific provider and tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Credentials decryption fails
    /// - Database connection issues
    pub async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: uuid::Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        match self {
            Self::SQLite(db) => db.get_tenant_oauth_credentials(tenant_id, provider).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_oauth_credentials(tenant_id, provider).await,
        }
    }

    // OAuth app registration implementations
    /// Register a new OAuth application
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - OAuth app data validation fails
    /// - Database constraint violations
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        match self {
            Self::SQLite(db) => db.create_oauth_app(app).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_oauth_app(app).await,
        }
    }

    /// Get an OAuth app by its client ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - OAuth app does not exist
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth_app_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<crate::models::OAuthApp> {
        match self {
            Self::SQLite(db) => db.get_oauth_app_by_client_id(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_oauth_app_by_client_id(client_id).await,
        }
    }

    /// List all OAuth apps registered by a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_oauth_apps_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        match self {
            Self::SQLite(db) => db.list_oauth_apps_for_user(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_oauth_apps_for_user(user_id).await,
        }
    }

    // ================================
    // OAuth 2.0 Server (RFC 7591)
    // ================================

    /// Store an OAuth 2.0 client registration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client data validation fails
    /// - Database constraint violations (e.g., duplicate `client_id`)
    /// - SQL execution fails
    /// - Database connection issues
    pub async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_oauth2_client(client).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_oauth2_client(client).await,
        }
    }

    /// Retrieve an OAuth 2.0 client by client ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>> {
        match self {
            Self::SQLite(db) => db.get_oauth2_client(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_oauth2_client(client_id).await,
        }
    }

    /// Store an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authorization code already exists
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_oauth2_auth_code(auth_code).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_oauth2_auth_code(auth_code).await,
        }
    }

    /// Retrieve an OAuth 2.0 authorization code by code value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        match self {
            Self::SQLite(db) => db.get_oauth2_auth_code(code).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_oauth2_auth_code(code).await,
        }
    }

    /// Update an existing OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authorization code does not exist
    /// - Database update fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_oauth2_auth_code(auth_code).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_oauth2_auth_code(auth_code).await,
        }
    }

    /// Store an OAuth 2.0 refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Refresh token already exists
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_oauth2_refresh_token(refresh_token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_oauth2_refresh_token(refresh_token).await,
        }
    }

    /// Retrieve an OAuth 2.0 refresh token by token value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        match self {
            Self::SQLite(db) => db.get_oauth2_refresh_token(token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_oauth2_refresh_token(token).await,
        }
    }

    /// Revoke an OAuth 2.0 refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.revoke_oauth2_refresh_token(token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.revoke_oauth2_refresh_token(token).await,
        }
    }

    /// Consume an OAuth 2.0 authorization code (marks as used and validates)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Code has already been used
    /// - Code has expired
    /// - Client ID or redirect URI mismatch
    /// - Database update fails
    /// - Database connection issues
    pub async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        match self {
            Self::SQLite(db) => {
                db.consume_auth_code(code, client_id, redirect_uri, now)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.consume_auth_code(code, client_id, redirect_uri, now)
                    .await
            }
        }
    }

    /// Consume an OAuth 2.0 refresh token (validates and optionally rotates)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Token has been revoked
    /// - Token has expired
    /// - Client ID mismatch
    /// - Database update fails
    /// - Database connection issues
    pub async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        match self {
            Self::SQLite(db) => db.consume_refresh_token(token, client_id, now).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.consume_refresh_token(token, client_id, now).await,
        }
    }

    /// Retrieve an OAuth 2.0 refresh token by its token value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        match self {
            Self::SQLite(db) => db.get_refresh_token_by_value(token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_refresh_token_by_value(token).await,
        }
    }

    /// Store an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authorization code already exists
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_authorization_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_authorization_code(auth_code).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_authorization_code(auth_code).await,
        }
    }

    /// Retrieve and validate an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Client ID or redirect URI mismatch
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<crate::oauth2_server::models::OAuth2AuthCode> {
        match self {
            Self::SQLite(db) => {
                db.get_authorization_code(code, client_id, redirect_uri)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_authorization_code(code, client_id, redirect_uri)
                    .await
            }
        }
    }

    /// Delete an OAuth 2.0 authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Code does not exist
    /// - Client ID or redirect URI mismatch
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.delete_authorization_code(code, client_id, redirect_uri)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.delete_authorization_code(code, client_id, redirect_uri)
                    .await
            }
        }
    }

    /// Store an OAuth 2.0 state parameter for CSRF protection
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State already exists
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_oauth2_state(state).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_oauth2_state(state).await,
        }
    }

    /// Consume an OAuth 2.0 state parameter (validates and marks as used)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State does not exist
    /// - State has already been used
    /// - State has expired
    /// - Client ID mismatch
    /// - Database update fails
    /// - Database connection issues
    pub async fn consume_oauth2_state(
        &self,
        state: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>> {
        match self {
            Self::SQLite(db) => db.consume_oauth2_state(state, client_id, now).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.consume_oauth2_state(state, client_id, now).await,
        }
    }

    // ================================
    // Key Rotation & Security
    // ================================

    /// Store a new encryption key version for key rotation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key version already exists
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_key_version(
        &self,
        tenant_id: Option<Uuid>,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_key_version(tenant_id, version).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_key_version(tenant_id, version).await,
        }
    }

    /// Retrieve all encryption key versions for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_key_versions(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        match self {
            Self::SQLite(db) => db.get_key_versions(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_key_versions(tenant_id).await,
        }
    }

    /// Retrieve the currently active encryption key version for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_current_key_version(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        match self {
            Self::SQLite(db) => db.get_current_key_version(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_current_key_version(tenant_id).await,
        }
    }

    /// Update the active status of an encryption key version
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key version does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_key_version_status(
        &self,
        tenant_id: Option<uuid::Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.update_key_version_status(tenant_id, version, is_active)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.update_key_version_status(tenant_id, version, is_active)
                    .await
            }
        }
    }

    /// Delete old encryption key versions, keeping the most recent ones
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_old_key_versions(
        &self,
        tenant_id: Option<uuid::Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        match self {
            Self::SQLite(db) => db.delete_old_key_versions(tenant_id, keep_count).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.delete_old_key_versions(tenant_id, keep_count).await,
        }
    }

    /// Retrieve all tenants in the system
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        match self {
            Self::SQLite(db) => db.get_all_tenants().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_all_tenants().await,
        }
    }

    /// Store a security audit event for compliance and monitoring
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn store_audit_event(
        &self,
        tenant_id: Option<Uuid>,
        event: &crate::security::audit::AuditEvent,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_audit_event(tenant_id, event).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_audit_event(tenant_id, event).await,
        }
    }

    /// Retrieve audit events with optional filtering by tenant and event type
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_audit_events(
        &self,
        tenant_id: Option<uuid::Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        match self {
            Self::SQLite(db) => db.get_audit_events(tenant_id, event_type, limit).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_audit_events(tenant_id, event_type, limit).await,
        }
    }

    // ================================
    // User OAuth Tokens (Multi-Tenant)
    // ================================

    /// Upsert user OAuth token for multi-tenant OAuth management
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn upsert_user_oauth_token(
        &self,
        token: &crate::models::UserOAuthToken,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                let token_data = crate::database::user_oauth_tokens::OAuthTokenData {
                    id: &token.id,
                    user_id: token.user_id,
                    tenant_id: &token.tenant_id,
                    provider: &token.provider,
                    access_token: &token.access_token,
                    refresh_token: token.refresh_token.as_deref(),
                    token_type: &token.token_type,
                    expires_at: token.expires_at,
                    scope: token.scope.as_deref().unwrap_or(""),
                };
                db.upsert_user_oauth_token(&token_data).await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.upsert_user_oauth_token(token).await,
        }
    }

    /// Get user OAuth token for a specific provider and tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<crate::models::UserOAuthToken>> {
        match self {
            Self::SQLite(db) => db.get_user_oauth_token(user_id, tenant_id, provider).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_oauth_token(user_id, tenant_id, provider).await,
        }
    }

    /// Get all OAuth tokens for a user across all providers
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::UserOAuthToken>> {
        match self {
            Self::SQLite(db) => db.get_user_oauth_tokens(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_oauth_tokens(user_id).await,
        }
    }

    /// Get all OAuth tokens for a specific provider within a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<crate::models::UserOAuthToken>> {
        match self {
            Self::SQLite(db) => db.get_tenant_provider_tokens(tenant_id, provider).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_provider_tokens(tenant_id, provider).await,
        }
    }

    /// Delete a specific OAuth token for a user, tenant, and provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.delete_user_oauth_token(user_id, tenant_id, provider)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.delete_user_oauth_token(user_id, tenant_id, provider)
                    .await
            }
        }
    }

    /// Delete all OAuth tokens for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        match self {
            Self::SQLite(db) => db.delete_user_oauth_tokens(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.delete_user_oauth_tokens(user_id).await,
        }
    }

    /// Refresh user OAuth token with new access and refresh tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.refresh_user_oauth_token(
                    user_id,
                    tenant_id,
                    provider,
                    access_token,
                    refresh_token,
                    expires_at,
                )
                .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.refresh_user_oauth_token(
                    user_id,
                    tenant_id,
                    provider,
                    access_token,
                    refresh_token,
                    expires_at,
                )
                .await
            }
        }
    }

    /// Get user role for a specific tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_user_tenant_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<String>> {
        match self {
            Self::SQLite(db) => {
                db.get_user_tenant_role(&user_id.to_string(), &tenant_id.to_string())
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_tenant_role(user_id, tenant_id).await,
        }
    }

    // ================================
    // User OAuth App Credentials
    // ================================

    /// Store user OAuth app credentials (`client_id`, `client_secret`)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Data encryption fails
    /// - Database connection issues
    pub async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.store_user_oauth_app(user_id, provider, client_id, client_secret, redirect_uri)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.store_user_oauth_app(user_id, provider, client_id, client_secret, redirect_uri)
                    .await
            }
        }
    }

    /// Get user OAuth app credentials for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Data decryption fails
    /// - Database connection issues
    pub async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>> {
        match self {
            Self::SQLite(db) => db.get_user_oauth_app(user_id, provider).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_oauth_app(user_id, provider).await,
        }
    }

    /// List all OAuth app providers configured for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>> {
        match self {
            Self::SQLite(db) => db.list_user_oauth_apps(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_user_oauth_apps(user_id).await,
        }
    }

    /// Remove user OAuth app credentials for a provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.remove_user_oauth_app(user_id, provider).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.remove_user_oauth_app(user_id, provider).await,
        }
    }

    // ================================
    // System Secret Management
    // ================================

    /// Get or create system secret (generates if not exists)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret generation fails
    /// - Database insertion fails
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        match self {
            Self::SQLite(db) => db.get_or_create_system_secret(secret_type).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_or_create_system_secret(secret_type).await,
        }
    }

    /// Get existing system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret does not exist
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        match self {
            Self::SQLite(db) => db.get_system_secret(secret_type).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_system_secret(secret_type).await,
        }
    }

    /// Update system secret (for rotation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_system_secret(secret_type, new_value).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_system_secret(secret_type, new_value).await,
        }
    }

    // ================================
    // OAuth Notifications
    // ================================

    /// Store OAuth completion notification for MCP resource delivery
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => {
                db.store_oauth_notification(user_id, provider, success, message, expires_at)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.store_oauth_notification(user_id, provider, success, message, expires_at)
                    .await
            }
        }
    }

    /// Get unread OAuth notifications for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        match self {
            Self::SQLite(db) => db.get_unread_oauth_notifications(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_unread_oauth_notifications(user_id).await,
        }
    }

    /// Mark OAuth notification as read
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Notification does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        match self {
            Self::SQLite(db) => {
                db.mark_oauth_notification_read(notification_id, user_id)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.mark_oauth_notification_read(notification_id, user_id)
                    .await
            }
        }
    }

    /// Mark all OAuth notifications as read for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues
    pub async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        match self {
            Self::SQLite(db) => db.mark_all_oauth_notifications_read(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.mark_all_oauth_notifications_read(user_id).await,
        }
    }

    /// Get all OAuth notifications for a user (read and unread)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        match self {
            Self::SQLite(db) => db.get_all_oauth_notifications(user_id, limit).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_all_oauth_notifications(user_id, limit).await,
        }
    }

    // ================================
    // Fitness Configuration Management
    // ================================

    /// Save tenant-level fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => {
                db.save_tenant_fitness_config(tenant_id, configuration_name, config)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.save_tenant_fitness_config(tenant_id, configuration_name, config)
                    .await
            }
        }
    }

    /// Save user-specific fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Database insertion fails
    /// - Data serialization fails
    /// - Database connection issues
    pub async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        match self {
            Self::SQLite(db) => {
                db.save_user_fitness_config(tenant_id, user_id, configuration_name, config)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.save_user_fitness_config(tenant_id, user_id, configuration_name, config)
                    .await
            }
        }
    }

    /// Get tenant-level fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        match self {
            Self::SQLite(db) => {
                db.get_tenant_fitness_config(tenant_id, configuration_name)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_tenant_fitness_config(tenant_id, configuration_name)
                    .await
            }
        }
    }

    /// Get user-specific fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        match self {
            Self::SQLite(db) => {
                db.get_user_fitness_config(tenant_id, user_id, configuration_name)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_user_fitness_config(tenant_id, user_id, configuration_name)
                    .await
            }
        }
    }

    /// List all tenant-level fitness configuration names
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        match self {
            Self::SQLite(db) => db.list_tenant_fitness_configurations(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_tenant_fitness_configurations(tenant_id).await,
        }
    }

    /// List all user-specific fitness configuration names
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues
    pub async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        match self {
            Self::SQLite(db) => {
                db.list_user_fitness_configurations(tenant_id, user_id)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.list_user_fitness_configurations(tenant_id, user_id)
                    .await
            }
        }
    }

    /// Delete fitness configuration (tenant or user-specific)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration does not exist
    /// - Database deletion fails
    /// - Database connection issues
    pub async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        match self {
            Self::SQLite(db) => {
                db.delete_fitness_config(tenant_id, user_id, configuration_name)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.delete_fitness_config(tenant_id, user_id, configuration_name)
                    .await
            }
        }
    }

    /// Save RSA keypair to database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Keypair already exists
    /// - Database insertion fails
    /// - Database connection issues
    pub async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: chrono::DateTime<chrono::Utc>,
        is_active: bool,
        key_size_bits: i32,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.save_rsa_keypair(
                    kid,
                    private_key_pem,
                    public_key_pem,
                    created_at,
                    is_active,
                    usize::try_from(key_size_bits).unwrap_or(2048),
                )
                .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.save_rsa_keypair(
                    kid,
                    private_key_pem,
                    public_key_pem,
                    created_at,
                    is_active,
                    key_size_bits,
                )
                .await
            }
        }
    }

    /// Load all RSA keypairs from database
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues
    pub async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, chrono::DateTime<chrono::Utc>, bool)>> {
        match self {
            Self::SQLite(db) => db.load_rsa_keypairs().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.load_rsa_keypairs().await,
        }
    }

    /// Update active status of RSA keypair
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Keypair does not exist
    /// - Database update fails
    /// - Database connection issues
    pub async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_rsa_keypair_active_status(kid, is_active).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_rsa_keypair_active_status(kid, is_active).await,
        }
    }
}

/// Automatically detect database type from connection string
///
/// # Errors
///
/// Returns an error if:
/// - Database URL format is not recognized (must start with 'sqlite:' or 'postgresql://')
/// - `PostgreSQL` URL is provided but `PostgreSQL` feature is not enabled
/// - Connection string is malformed or empty
pub fn detect_database_type(database_url: &str) -> Result<DatabaseType> {
    if database_url.starts_with("sqlite:") {
        Ok(DatabaseType::SQLite)
    } else if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
        #[cfg(feature = "postgresql")]
        return Ok(DatabaseType::PostgreSQL);

        #[cfg(not(feature = "postgresql"))]
        return Err(AppError::config(
            "PostgreSQL connection string detected, but PostgreSQL support is not enabled. \
             Enable the 'postgresql' feature flag in Cargo.toml",
        )
        .into());
    } else {
        Err(AppError::config(format!(
            "Unsupported database URL format: {database_url}. \
             Supported formats: sqlite:path/to/db.sqlite, postgresql://user:pass@host/db"
        ))
        .into())
    }
}
