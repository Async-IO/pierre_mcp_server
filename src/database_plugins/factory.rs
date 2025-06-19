//! Database factory for creating database providers
//!
//! This module provides automatic database type detection and creation
//! based on connection strings.

use super::DatabaseProvider;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::{info, debug};

#[cfg(feature = "postgresql")]
use super::postgres::PostgresDatabase;
use super::sqlite::SqliteDatabase;

/// Supported database types
#[derive(Debug, Clone, PartialEq)]
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
    pub fn backend_info(&self) -> &'static str {
        match self {
            Database::SQLite(_) => "SQLite (Local Development)",
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(_) => "PostgreSQL (Cloud-Ready)",
        }
    }

    /// Get the database type enum
    pub fn database_type(&self) -> DatabaseType {
        match self {
            Database::SQLite(_) => DatabaseType::SQLite,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(_) => DatabaseType::PostgreSQL,
        }
    }

    /// Get detailed database information for logging/monitoring
    pub fn info_summary(&self) -> String {
        match self {
            Database::SQLite(_) => {
                format!(
                    "Database Backend: SQLite\n\
                     Type: Embedded file-based database\n\
                     Use Case: Local development and testing\n\
                     Features: Zero-configuration, serverless, lightweight"
                )
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(_) => {
                format!(
                    "Database Backend: PostgreSQL\n\
                     Type: Client-server relational database\n\
                     Use Case: Production and cloud deployments\n\
                     Features: Concurrent access, advanced queries, scalability"
                )
            }
        }
    }

    /// Create a new database instance based on the connection string
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        debug!("Detecting database type from URL: {}", database_url);
        let db_type = detect_database_type(database_url)?;
        info!("🗄️  Detected database type: {:?}", db_type);

        match db_type {
            DatabaseType::SQLite => {
                info!("📁 Initializing SQLite database");
                let db = SqliteDatabase::new(database_url, encryption_key).await?;
                info!("✅ SQLite database initialized successfully");
                Ok(Database::SQLite(db))
            }
            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => {
                info!("🐘 Initializing PostgreSQL database");
                let db = PostgresDatabase::new(database_url, encryption_key).await?;
                info!("✅ PostgreSQL database initialized successfully");
                Ok(Database::PostgreSQL(db))
            }
            #[cfg(not(feature = "postgresql"))]
            DatabaseType::PostgreSQL => {
                let err_msg = "PostgreSQL support not enabled. Enable the 'postgresql' feature flag.";
                tracing::error!("❌ {}", err_msg);
                Err(anyhow!(err_msg))
            }
        }
    }
}

/// Automatically detect database type from connection string
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
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        Self::new(database_url, encryption_key).await
    }

    async fn migrate(&self) -> Result<()> {
        match self {
            Database::SQLite(db) => db.migrate().await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.migrate().await,
        }
    }

    async fn create_user(&self, user: &crate::models::User) -> Result<uuid::Uuid> {
        match self {
            Database::SQLite(db) => db.create_user(user).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.create_user(user).await,
        }
    }

    async fn get_user(&self, user_id: uuid::Uuid) -> Result<Option<crate::models::User>> {
        match self {
            Database::SQLite(db) => db.get_user(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user(user_id).await,
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<crate::models::User>> {
        match self {
            Database::SQLite(db) => db.get_user_by_email(email).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_by_email(email).await,
        }
    }

    async fn get_user_by_email_required(&self, email: &str) -> Result<crate::models::User> {
        match self {
            Database::SQLite(db) => db.get_user_by_email_required(email).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_by_email_required(email).await,
        }
    }

    async fn update_last_active(&self, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Database::SQLite(db) => db.update_last_active(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.update_last_active(user_id).await,
        }
    }

    async fn get_user_count(&self) -> Result<i64> {
        match self {
            Database::SQLite(db) => db.get_user_count().await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_count().await,
        }
    }

    async fn update_strava_token(
        &self,
        user_id: uuid::Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        scope: String,
    ) -> Result<()> {
        match self {
            Database::SQLite(db) => {
                db.update_strava_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.update_strava_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
        }
    }

    async fn get_strava_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<crate::models::DecryptedToken>> {
        match self {
            Database::SQLite(db) => db.get_strava_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_strava_token(user_id).await,
        }
    }

    async fn update_fitbit_token(
        &self,
        user_id: uuid::Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        scope: String,
    ) -> Result<()> {
        match self {
            Database::SQLite(db) => {
                db.update_fitbit_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.update_fitbit_token(user_id, access_token, refresh_token, expires_at, scope)
                    .await
            }
        }
    }

    async fn get_fitbit_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<crate::models::DecryptedToken>> {
        match self {
            Database::SQLite(db) => db.get_fitbit_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_fitbit_token(user_id).await,
        }
    }

    async fn clear_strava_token(&self, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Database::SQLite(db) => db.clear_strava_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.clear_strava_token(user_id).await,
        }
    }

    async fn clear_fitbit_token(&self, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Database::SQLite(db) => db.clear_fitbit_token(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.clear_fitbit_token(user_id).await,
        }
    }

    async fn upsert_user_profile(
        &self,
        user_id: uuid::Uuid,
        profile_data: serde_json::Value,
    ) -> Result<()> {
        match self {
            Database::SQLite(db) => db.upsert_user_profile(user_id, profile_data).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.upsert_user_profile(user_id, profile_data).await,
        }
    }

    async fn get_user_profile(&self, user_id: uuid::Uuid) -> Result<Option<serde_json::Value>> {
        match self {
            Database::SQLite(db) => db.get_user_profile(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_profile(user_id).await,
        }
    }

    async fn create_goal(
        &self,
        user_id: uuid::Uuid,
        goal_data: serde_json::Value,
    ) -> Result<String> {
        match self {
            Database::SQLite(db) => db.create_goal(user_id, goal_data).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.create_goal(user_id, goal_data).await,
        }
    }

    async fn get_user_goals(&self, user_id: uuid::Uuid) -> Result<Vec<serde_json::Value>> {
        match self {
            Database::SQLite(db) => db.get_user_goals(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_goals(user_id).await,
        }
    }

    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        match self {
            Database::SQLite(db) => db.update_goal_progress(goal_id, current_value).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.update_goal_progress(goal_id, current_value).await,
        }
    }

    async fn store_insight(
        &self,
        user_id: uuid::Uuid,
        insight_data: serde_json::Value,
    ) -> Result<String> {
        match self {
            Database::SQLite(db) => db.store_insight(user_id, insight_data).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.store_insight(user_id, insight_data).await,
        }
    }

    async fn get_user_insights(
        &self,
        user_id: uuid::Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<serde_json::Value>> {
        match self {
            Database::SQLite(db) => db.get_user_insights(user_id, insight_type, limit).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_insights(user_id, insight_type, limit).await,
        }
    }

    async fn create_api_key(&self, api_key: &crate::api_keys::ApiKey) -> Result<()> {
        match self {
            Database::SQLite(db) => db.create_api_key(api_key).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.create_api_key(api_key).await,
        }
    }

    async fn get_api_key_by_prefix(
        &self,
        prefix: &str,
        hash: &str,
    ) -> Result<Option<crate::api_keys::ApiKey>> {
        match self {
            Database::SQLite(db) => db.get_api_key_by_prefix(prefix, hash).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_api_key_by_prefix(prefix, hash).await,
        }
    }

    async fn get_user_api_keys(&self, user_id: uuid::Uuid) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Database::SQLite(db) => db.get_user_api_keys(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_user_api_keys(user_id).await,
        }
    }

    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        match self {
            Database::SQLite(db) => db.update_api_key_last_used(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.update_api_key_last_used(api_key_id).await,
        }
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: uuid::Uuid) -> Result<()> {
        match self {
            Database::SQLite(db) => db.deactivate_api_key(api_key_id, user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.deactivate_api_key(api_key_id, user_id).await,
        }
    }

    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        match self {
            Database::SQLite(db) => db.cleanup_expired_api_keys().await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.cleanup_expired_api_keys().await,
        }
    }

    async fn get_expired_api_keys(&self) -> Result<Vec<crate::api_keys::ApiKey>> {
        match self {
            Database::SQLite(db) => db.get_expired_api_keys().await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_expired_api_keys().await,
        }
    }

    async fn record_api_key_usage(&self, usage: &crate::api_keys::ApiKeyUsage) -> Result<()> {
        match self {
            Database::SQLite(db) => db.record_api_key_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.record_api_key_usage(usage).await,
        }
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        match self {
            Database::SQLite(db) => db.get_api_key_current_usage(api_key_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_api_key_current_usage(api_key_id).await,
        }
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::api_keys::ApiKeyUsageStats> {
        match self {
            Database::SQLite(db) => {
                db.get_api_key_usage_stats(api_key_id, start_date, end_date)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.get_api_key_usage_stats(api_key_id, start_date, end_date)
                    .await
            }
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
            Database::SQLite(db) => {
                db.get_request_logs(api_key_id, start_time, end_time, status_filter, tool_filter)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.get_request_logs(api_key_id, start_time, end_time, status_filter, tool_filter)
                    .await
            }
        }
    }

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        match self {
            Database::SQLite(db) => db.get_system_stats().await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_system_stats().await,
        }
    }

    async fn create_a2a_client(&self, client: &A2AClient, api_key_id: &str) -> Result<String> {
        match self {
            Database::SQLite(db) => db.create_a2a_client(client, api_key_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.create_a2a_client(client, api_key_id).await,
        }
    }

    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        match self {
            Database::SQLite(db) => db.get_a2a_client(client_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_a2a_client(client_id).await,
        }
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        match self {
            Database::SQLite(db) => db.get_a2a_client_by_name(name).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_a2a_client_by_name(name).await,
        }
    }

    async fn list_a2a_clients(&self, user_id: &uuid::Uuid) -> Result<Vec<A2AClient>> {
        match self {
            Database::SQLite(db) => db.list_a2a_clients(user_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.list_a2a_clients(user_id).await,
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
            Database::SQLite(db) => {
                db.create_a2a_session(client_id, user_id, granted_scopes, expires_in_hours)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.create_a2a_session(client_id, user_id, granted_scopes, expires_in_hours)
                    .await
            }
        }
    }

    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        match self {
            Database::SQLite(db) => db.get_a2a_session(session_token).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_a2a_session(session_token).await,
        }
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        match self {
            Database::SQLite(db) => db.update_a2a_session_activity(session_token).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.update_a2a_session_activity(session_token).await,
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
            Database::SQLite(db) => {
                db.create_a2a_task(client_id, session_id, task_type, input_data)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.create_a2a_task(client_id, session_id, task_type, input_data)
                    .await
            }
        }
    }

    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        match self {
            Database::SQLite(db) => db.get_a2a_task(task_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_a2a_task(task_id).await,
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
            Database::SQLite(db) => {
                db.update_a2a_task_status(task_id, status, result, error)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.update_a2a_task_status(task_id, status, result, error)
                    .await
            }
        }
    }

    async fn record_a2a_usage(&self, usage: &crate::database::A2AUsage) -> Result<()> {
        match self {
            Database::SQLite(db) => db.record_a2a_usage(usage).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.record_a2a_usage(usage).await,
        }
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        match self {
            Database::SQLite(db) => db.get_a2a_client_current_usage(client_id).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_a2a_client_current_usage(client_id).await,
        }
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        match self {
            Database::SQLite(db) => {
                db.get_a2a_usage_stats(client_id, start_date, end_date)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
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
            Database::SQLite(db) => db.get_a2a_client_usage_history(client_id, days).await,
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => db.get_a2a_client_usage_history(client_id, days).await,
        }
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: uuid::Uuid,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        match self {
            Database::SQLite(db) => {
                db.get_top_tools_analysis(user_id, start_time, end_time)
                    .await
            }
            #[cfg(feature = "postgresql")]
            Database::PostgreSQL(db) => {
                db.get_top_tools_analysis(user_id, start_time, end_time)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_database_type() {
        // SQLite URLs
        assert_eq!(
            detect_database_type("sqlite:./data/test.db").unwrap(),
            DatabaseType::SQLite
        );
        assert_eq!(
            detect_database_type("sqlite::memory:").unwrap(),
            DatabaseType::SQLite
        );

        // PostgreSQL URLs (only test detection, not creation)
        #[cfg(feature = "postgresql")]
        {
            assert_eq!(
                detect_database_type("postgresql://user:pass@localhost/db").unwrap(),
                DatabaseType::PostgreSQL
            );
            assert_eq!(
                detect_database_type("postgres://user:pass@localhost/db").unwrap(),
                DatabaseType::PostgreSQL
            );
        }

        // Invalid URLs
        assert!(detect_database_type("mysql://user:pass@localhost/db").is_err());
        assert!(detect_database_type("invalid_url").is_err());
    }
}
