// ABOUTME: Database factory and provider abstraction for multi-database support
// ABOUTME: Provides unified interface for SQLite and PostgreSQL with runtime database selection
//! Database factory for creating database providers
//!
//! This module provides automatic database type detection and creation
//! based on connection strings.

use super::DatabaseProvider;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::rate_limiting::JwtUsage;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::{debug, info};
use uuid::Uuid;

#[cfg(feature = "postgresql")]
use super::postgres::PostgresDatabase;
use super::sqlite::SqliteDatabase;

/// Supported database types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
}

/// Database instance wrapper that delegates to the appropriate implementation
#[derive(Clone)]
pub enum Database {
    SQLite(SqliteDatabase),
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
                .to_string(),
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(_) => "Database Backend: PostgreSQL\n\
                     Type: Client-server relational database\n\
                     Use Case: Production and cloud deployments\n\
                     Features: Concurrent access, advanced queries, scalability"
                .to_string(),
        }
    }

    /// Create a new database instance based on the connection string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database URL format is unsupported or invalid
    /// - `PostgreSQL` feature is not enabled when `PostgreSQL` URL is provided
    /// - Database connection fails
    /// - Database initialization or migration fails
    /// - Encryption key is invalid
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
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
                let db = PostgresDatabase::new(database_url, encryption_key).await?;
                info!("PostgreSQL database initialized successfully");
                Ok(Self::PostgreSQL(db))
            }
            #[cfg(not(feature = "postgresql"))]
            DatabaseType::PostgreSQL => {
                let err_msg =
                    "PostgreSQL support not enabled. Enable the 'postgresql' feature flag.";
                tracing::error!("{}", err_msg);
                Err(anyhow!(err_msg))
            }
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
        return Err(anyhow!(
            "PostgreSQL connection string detected, but PostgreSQL support is not enabled. \
             Enable the 'postgresql' feature flag in Cargo.toml"
        ));
    } else {
        Err(anyhow!(
            "Unsupported database URL format: {}. \
             Supported formats: sqlite:path/to/db.sqlite, postgresql://user:pass@host/db",
            database_url
        ))
    }
}

// Implement DatabaseProvider for the enum by delegating to the appropriate implementation
#[async_trait]
impl DatabaseProvider for Database {
    /// Create a new database provider instance
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database URL format is unsupported
    /// - Database connection fails
    /// - Migration process fails
    /// - Encryption setup fails
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        Self::new(database_url, encryption_key).await
    }

    /// Run database migrations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - SQL migration statements fail to execute
    /// - Database connection is lost during migration
    /// - Migration scripts are malformed
    /// - Insufficient database permissions
    async fn migrate(&self) -> Result<()> {
        match self {
            Self::SQLite(db) => db.migrate().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.migrate().await,
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
    async fn create_user(&self, user: &crate::models::User) -> Result<uuid::Uuid> {
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
    async fn get_user(&self, user_id: uuid::Uuid) -> Result<Option<crate::models::User>> {
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
    async fn get_user_by_email(&self, email: &str) -> Result<Option<crate::models::User>> {
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
    async fn get_user_by_email_required(&self, email: &str) -> Result<crate::models::User> {
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
    async fn update_last_active(&self, user_id: uuid::Uuid) -> Result<()> {
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
    async fn get_user_count(&self) -> Result<i64> {
        match self {
            Self::SQLite(db) => db.get_user_count().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_count().await,
        }
    }

    /// Update or store Strava OAuth tokens for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Token encryption fails
    /// - Database update fails
    /// - Database connection issues
    /// - Token data is invalid
    async fn update_strava_token(
        &self,
        user_id: uuid::Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        scope: String,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.update_strava_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.update_strava_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
        }
    }

    /// Get decrypted Strava token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Token decryption fails
    /// - Data deserialization fails
    /// - Database connection issues
    async fn get_strava_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<crate::models::DecryptedToken>> {
        match self {
            Self::SQLite(db) => db.get_strava_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_strava_token(user_id).await,
        }
    }

    /// Update or store Fitbit OAuth tokens for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User does not exist
    /// - Token encryption fails
    /// - Database update fails
    /// - Database connection issues
    /// - Token data is invalid
    async fn update_fitbit_token(
        &self,
        user_id: uuid::Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        scope: String,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.update_fitbit_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.update_fitbit_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
        }
    }

    /// Get decrypted Fitbit token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Token decryption fails
    /// - Data deserialization fails
    /// - Database connection issues
    async fn get_fitbit_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<crate::models::DecryptedToken>> {
        match self {
            Self::SQLite(db) => db.get_fitbit_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_fitbit_token(user_id).await,
        }
    }

    /// Clear/delete Strava token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database delete operation fails
    /// - User does not exist
    /// - Database connection issues
    async fn clear_strava_token(&self, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Self::SQLite(db) => db.clear_strava_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.clear_strava_token(user_id).await,
        }
    }

    /// Clear/delete Fitbit token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database delete operation fails
    /// - User does not exist
    /// - Database connection issues
    async fn clear_fitbit_token(&self, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Self::SQLite(db) => db.clear_fitbit_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.clear_fitbit_token(user_id).await,
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
    async fn upsert_user_profile(
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
    async fn get_user_profile(&self, user_id: uuid::Uuid) -> Result<Option<serde_json::Value>> {
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
    async fn create_goal(
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
    async fn get_user_goals(&self, user_id: uuid::Uuid) -> Result<Vec<serde_json::Value>> {
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
    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
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
    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
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
    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
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
    async fn store_insight(
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
    async fn get_user_insights(
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
    async fn create_api_key(&self, api_key: &crate::api_keys::ApiKey) -> Result<()> {
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
    async fn get_api_key_by_prefix(
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
    async fn get_user_api_keys(&self, user_id: uuid::Uuid) -> Result<Vec<crate::api_keys::ApiKey>> {
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
    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
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
    async fn deactivate_api_key(&self, api_key_id: &str, user_id: uuid::Uuid) -> Result<()> {
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
    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<crate::api_keys::ApiKey>> {
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
    async fn get_api_keys_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => {
                db.get_api_keys_filtered(user_email, active_only, limit, offset)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_api_keys_filtered(user_email, active_only, limit, offset)
                    .await
            }
        }
    }

    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        match self {
            Self::SQLite(db) => db.cleanup_expired_api_keys().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.cleanup_expired_api_keys().await,
        }
    }

    async fn get_expired_api_keys(&self) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Self::SQLite(db) => db.get_expired_api_keys().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_expired_api_keys().await,
        }
    }

    async fn record_api_key_usage(&self, usage: &crate::api_keys::ApiKeyUsage) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_api_key_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_api_key_usage(usage).await,
        }
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        match self {
            Self::SQLite(db) => db.get_api_key_current_usage(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_api_key_current_usage(api_key_id).await,
        }
    }

    async fn get_api_key_usage_stats(
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

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_jwt_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_jwt_usage(usage).await,
        }
    }

    async fn get_jwt_current_usage(&self, user_id: uuid::Uuid) -> Result<u32> {
        match self {
            Self::SQLite(db) => db.get_jwt_current_usage(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_jwt_current_usage(user_id).await,
        }
    }

    async fn get_request_logs(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        match self {
            Self::SQLite(db) => {
                db.get_request_logs(api_key_id, start_time, end_time, status_filter, tool_filter)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.get_request_logs(api_key_id, start_time, end_time, status_filter, tool_filter)
                    .await
            }
        }
    }

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
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
    async fn create_a2a_client(
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
    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client(client_id).await,
        }
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_by_name(name).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_by_name(name).await,
        }
    }

    async fn list_a2a_clients(&self, user_id: &uuid::Uuid) -> Result<Vec<A2AClient>> {
        match self {
            Self::SQLite(db) => db.list_a2a_clients(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_a2a_clients(user_id).await,
        }
    }

    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_a2a_client(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_a2a_client(client_id).await,
        }
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_credentials(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_credentials(client_id).await,
        }
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.invalidate_a2a_client_sessions(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.invalidate_a2a_client_sessions(client_id).await,
        }
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_client_api_keys(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_client_api_keys(client_id).await,
        }
    }

    async fn create_a2a_session(
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

    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        match self {
            Self::SQLite(db) => db.get_a2a_session(session_token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_session(session_token).await,
        }
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.update_a2a_session_activity(session_token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.update_a2a_session_activity(session_token).await,
        }
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        match self {
            Self::SQLite(db) => db.get_active_a2a_sessions(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_active_a2a_sessions(client_id).await,
        }
    }

    async fn create_a2a_task(
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

    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        match self {
            Self::SQLite(db) => db.get_a2a_task(task_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_task(task_id).await,
        }
    }

    async fn list_a2a_tasks(
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

    async fn update_a2a_task_status(
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

    async fn record_a2a_usage(&self, usage: &crate::database::A2AUsage) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_a2a_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_a2a_usage(usage).await,
        }
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        match self {
            Self::SQLite(db) => db.get_a2a_client_current_usage(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_a2a_client_current_usage(client_id).await,
        }
    }

    async fn get_a2a_usage_stats(
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

    async fn get_a2a_client_usage_history(
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

    async fn get_provider_last_sync(
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

    async fn update_provider_last_sync(
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

    async fn get_top_tools_analysis(
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
    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        match self {
            Self::SQLite(db) => db.create_admin_token(request).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_admin_token(request).await,
        }
    }

    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        match self {
            Self::SQLite(db) => db.get_admin_token_by_id(token_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_admin_token_by_id(token_id).await,
        }
    }

    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        match self {
            Self::SQLite(db) => db.get_admin_token_by_prefix(token_prefix).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_admin_token_by_prefix(token_prefix).await,
        }
    }

    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        match self {
            Self::SQLite(db) => db.list_admin_tokens(include_inactive).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_admin_tokens(include_inactive).await,
        }
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.deactivate_admin_token(token_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.deactivate_admin_token(token_id).await,
        }
    }

    async fn update_admin_token_last_used(
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

    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.record_admin_token_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.record_admin_token_usage(usage).await,
        }
    }

    async fn get_admin_token_usage_history(
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

    async fn record_admin_provisioned_key(
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

    async fn get_admin_provisioned_keys(
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
    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        match self {
            Self::SQLite(db) => db.create_tenant(tenant).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_tenant(tenant).await,
        }
    }

    async fn get_tenant_by_id(&self, tenant_id: uuid::Uuid) -> Result<crate::models::Tenant> {
        match self {
            Self::SQLite(db) => db.get_tenant_by_id(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_by_id(tenant_id).await,
        }
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        match self {
            Self::SQLite(db) => db.get_tenant_by_slug(slug).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_by_slug(slug).await,
        }
    }

    async fn list_tenants_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<crate::models::Tenant>> {
        match self {
            Self::SQLite(db) => db.list_tenants_for_user(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_tenants_for_user(user_id).await,
        }
    }

    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_tenant_oauth_credentials(credentials).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_tenant_oauth_credentials(credentials).await,
        }
    }

    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: uuid::Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        match self {
            Self::SQLite(db) => db.get_tenant_oauth_providers(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_tenant_oauth_providers(tenant_id).await,
        }
    }

    async fn get_tenant_oauth_credentials(
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
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        match self {
            Self::SQLite(db) => db.create_oauth_app(app).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.create_oauth_app(app).await,
        }
    }

    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp> {
        match self {
            Self::SQLite(db) => db.get_oauth_app_by_client_id(client_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_oauth_app_by_client_id(client_id).await,
        }
    }

    async fn list_oauth_apps_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        match self {
            Self::SQLite(db) => db.list_oauth_apps_for_user(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.list_oauth_apps_for_user(user_id).await,
        }
    }

    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => {
                db.store_authorization_code(code, client_id, redirect_uri, scope, user_id)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => {
                db.store_authorization_code(code, client_id, redirect_uri, scope, user_id)
                    .await
            }
        }
    }

    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode> {
        match self {
            Self::SQLite(db) => db.get_authorization_code(code).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_authorization_code(code).await,
        }
    }

    async fn delete_authorization_code(&self, code: &str) -> Result<()> {
        match self {
            Self::SQLite(db) => db.delete_authorization_code(code).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.delete_authorization_code(code).await,
        }
    }

    // ================================
    // Key Rotation & Security
    // ================================

    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_key_version(version).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_key_version(version).await,
        }
    }

    async fn get_key_versions(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        match self {
            Self::SQLite(db) => db.get_key_versions(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_key_versions(tenant_id).await,
        }
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        match self {
            Self::SQLite(db) => db.get_current_key_version(tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_current_key_version(tenant_id).await,
        }
    }

    async fn update_key_version_status(
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

    async fn delete_old_key_versions(
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

    async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        match self {
            Self::SQLite(db) => db.get_all_tenants().await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_all_tenants().await,
        }
    }

    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> Result<()> {
        match self {
            Self::SQLite(db) => db.store_audit_event(event).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.store_audit_event(event).await,
        }
    }

    async fn get_audit_events(
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

    async fn upsert_user_oauth_token(&self, token: &crate::models::UserOAuthToken) -> Result<()> {
        match self {
            Self::SQLite(db) => db.upsert_user_oauth_token(token).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.upsert_user_oauth_token(token).await,
        }
    }

    async fn get_user_oauth_token(
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

    async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::UserOAuthToken>> {
        match self {
            Self::SQLite(db) => db.get_user_oauth_tokens(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_oauth_tokens(user_id).await,
        }
    }

    async fn get_tenant_provider_tokens(
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

    async fn delete_user_oauth_token(
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

    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        match self {
            Self::SQLite(db) => db.delete_user_oauth_tokens(user_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.delete_user_oauth_tokens(user_id).await,
        }
    }

    async fn refresh_user_oauth_token(
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
    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>> {
        match self {
            Self::SQLite(db) => db.get_user_tenant_role(user_id, tenant_id).await,
            #[cfg(feature = "postgresql")]
            Self::PostgreSQL(db) => db.get_user_tenant_role(user_id, tenant_id).await,
        }
    }
}
