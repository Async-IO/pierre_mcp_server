// ABOUTME: SQLite database implementation for local development and single-user deployments
// ABOUTME: Provides embedded database support with encryption and file-based storage
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! `SQLite` database implementation
//!
//! This module wraps the existing `SQLite` database functionality
//! to implement the `DatabaseProvider` trait.

use super::DatabaseProvider;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::database::errors::DatabaseError;
use crate::database::A2AUsage;
use crate::errors::AppError;
use crate::models::{User, UserOAuthApp, UserOAuthToken};
use crate::rate_limiting::JwtUsage;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

/// Safe cast from i64 to f64 with precision awareness
#[inline]
const fn safe_i64_to_f64(value: i64) -> f64 {
    // i64 to f64 conversion can lose precision for very large values
    // but for database counts/usage statistics, this is acceptable
    #[allow(clippy::cast_precision_loss)]
    {
        value as f64
    }
}

/// `SQLite` database implementation
#[derive(Clone)]
pub struct SqliteDatabase {
    /// The underlying database instance
    inner: crate::database::Database,
}

impl SqliteDatabase {
    /// Get a reference to the inner database for shared functionality
    #[must_use]
    pub const fn inner(&self) -> &crate::database::Database {
        &self.inner
    }
}

#[async_trait]
impl DatabaseProvider for SqliteDatabase {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        let inner = crate::database::Database::new(database_url, encryption_key).await?;
        Ok(Self { inner })
    }

    async fn migrate(&self) -> Result<()> {
        self.inner.migrate().await
    }

    async fn create_user(&self, user: &User) -> Result<Uuid> {
        self.inner.create_user(user).await
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        self.inner.get_user(user_id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.inner.get_user_by_email(email).await
    }

    async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        self.inner.get_user_by_email_required(email).await
    }

    async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        self.inner.update_last_active(user_id).await
    }

    async fn get_user_count(&self) -> Result<i64> {
        self.inner.get_user_count().await
    }

    async fn get_users_by_status(&self, status: &str) -> Result<Vec<User>> {
        self.inner.get_users_by_status(status).await
    }

    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<User>> {
        self.inner.get_users_by_status_cursor(status, params).await
    }

    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<User> {
        self.inner
            .update_user_status(user_id, new_status, admin_token_id)
            .await
    }

    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> Result<()> {
        self.inner.update_user_tenant_id(user_id, tenant_id).await
    }

    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> Result<()> {
        self.inner.upsert_user_profile(user_id, profile_data).await
    }

    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<Value>> {
        self.inner.get_user_profile(user_id).await
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String> {
        self.inner.create_goal(user_id, goal_data).await
    }

    async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<Value>> {
        self.inner.get_user_goals(user_id).await
    }

    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        self.inner
            .update_goal_progress(goal_id, current_value)
            .await
    }

    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        // First ensure the user_configurations table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_configurations (
                user_id TEXT PRIMARY KEY,
                config_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(self.inner.pool())
        .await?;

        let query = "SELECT config_data FROM user_configurations WHERE user_id = ?";

        let row = sqlx::query(query)
            .bind(user_id)
            .fetch_optional(self.inner.pool())
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
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(self.inner.pool())
        .await?;

        // Insert or update configuration
        let query = r"
            INSERT INTO user_configurations (user_id, config_data, updated_at) 
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET 
                config_data = excluded.config_data,
                updated_at = CURRENT_TIMESTAMP
        ";

        sqlx::query(query)
            .bind(user_id)
            .bind(config_json)
            .execute(self.inner.pool())
            .await?;

        Ok(())
    }

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String> {
        self.inner.store_insight(user_id, insight_data).await
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>> {
        let insights = self
            .inner
            .get_user_insights(user_id, insight_type, limit)
            .await?;

        // Filter by insight_type if specified
        if let Some(filter_type) = insight_type {
            Ok(insights
                .into_iter()
                .filter(|insight| insight.get("type").and_then(|t| t.as_str()) == Some(filter_type))
                .collect())
        } else {
            Ok(insights)
        }
    }

    async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        self.inner.create_api_key(api_key).await
    }

    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> Result<Option<ApiKey>> {
        self.inner.get_api_key_by_prefix(prefix, hash).await
    }

    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        self.inner.get_user_api_keys(user_id).await
    }

    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        self.inner.update_api_key_last_used(api_key_id).await
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
        self.inner.deactivate_api_key(api_key_id, user_id).await
    }

    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>> {
        self.inner.get_api_key_by_id(api_key_id).await
    }

    async fn get_api_keys_filtered(
        &self,
        _user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>> {
        self.inner
            .get_api_keys_filtered(
                None,
                None,
                Some(active_only),
                limit.unwrap_or(10),
                offset.unwrap_or(0),
            )
            .await
    }

    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        self.inner.cleanup_expired_api_keys().await
    }

    async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        self.inner.get_expired_api_keys().await
    }

    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
        self.inner.record_api_key_usage(usage).await
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        self.inner.get_api_key_current_usage(api_key_id).await
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats> {
        self.inner
            .get_api_key_usage_stats(api_key_id, start_date, end_date)
            .await
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        self.inner.record_jwt_usage(usage).await
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
        self.inner.get_jwt_current_usage(user_id).await
    }

    async fn get_request_logs(
        &self,
        _api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        _status_filter: Option<&str>,
        _tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        let analytics_logs = self
            .inner
            .get_request_logs(None, start_time, end_time, 10, 0)
            .await?;

        // Convert analytics::RequestLog to dashboard_routes::RequestLog
        Ok(analytics_logs
            .into_iter()
            .map(|log| crate::dashboard_routes::RequestLog {
                id: log.id.to_string(),
                timestamp: log.timestamp,
                api_key_id: log.api_key_id.unwrap_or_default(),
                api_key_name: "Unknown".into(),
                tool_name: "Unknown".into(),
                status_code: i32::from(log.status_code),
                response_time_ms: log.response_time_ms.and_then(|ms| i32::try_from(ms).ok()),
                error_message: log.error_message,
                request_size_bytes: None,
                response_size_bytes: None,
            })
            .collect())
    }

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        self.inner.get_system_stats().await
    }

    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        self.inner
            .create_a2a_client(client, client_secret, api_key_id)
            .await
    }

    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        self.inner.get_a2a_client(client_id).await
    }

    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> Result<Option<A2AClient>> {
        self.inner.get_a2a_client_by_api_key_id(api_key_id).await
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        self.inner.get_a2a_client_by_name(name).await
    }

    async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        self.inner.list_a2a_clients(user_id).await
    }

    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        self.inner.deactivate_a2a_client(client_id).await
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        self.inner.get_a2a_client_credentials(client_id).await
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        self.inner.invalidate_a2a_client_sessions(client_id).await
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        self.inner.deactivate_client_api_keys(client_id).await
    }

    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        self.inner
            .create_a2a_session(client_id, user_id, granted_scopes, expires_in_hours)
            .await
    }

    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        self.inner.get_a2a_session(session_token).await
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        self.inner.update_a2a_session_activity(session_token).await
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        self.inner.get_active_a2a_sessions(client_id).await
    }

    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String> {
        self.inner
            .create_a2a_task(client_id, session_id, task_type, input_data)
            .await
    }

    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        self.inner.get_a2a_task(task_id).await
    }

    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        self.inner
            .list_a2a_tasks(client_id, status_filter, limit, offset)
            .await
    }

    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        self.inner
            .update_a2a_task_status(task_id, status, result, error)
            .await
    }

    async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()> {
        self.inner.record_a2a_usage(usage).await
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        self.inner.get_a2a_client_current_usage(client_id).await
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        self.inner
            .get_a2a_usage_stats(client_id, start_date, end_date)
            .await
    }

    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        // The new database method already returns aggregated data
        self.inner
            .get_a2a_client_usage_history(client_id, days)
            .await
    }

    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        self.inner.get_provider_last_sync(user_id, provider).await
    }

    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        self.inner
            .update_provider_last_sync(user_id, provider, sync_time)
            .await
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        // Query API key usage for this user within the time range
        let query = r"
            SELECT tool_name, COUNT(*) as usage_count,
                   AVG(response_time_ms) as avg_response_time,
                   SUM(CASE WHEN status_code < 400 THEN 1 ELSE 0 END) as success_count,
                   SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as error_count
            FROM api_key_usage aku
            JOIN api_keys ak ON aku.api_key_id = ak.id
            WHERE ak.user_id = ? AND aku.timestamp BETWEEN ? AND ?
            GROUP BY tool_name
            ORDER BY usage_count DESC
            LIMIT 10
        ";

        let rows = sqlx::query(query)
            .bind(user_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_all(self.inner.pool())
            .await?;

        let mut tool_usage = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;

            let tool_name: String = row
                .try_get("tool_name")
                .unwrap_or_else(|_| "unknown".into());
            let usage_count: i64 = row.try_get("usage_count").unwrap_or(0);
            let avg_response_time: Option<f64> = row.try_get("avg_response_time").ok();
            let success_count: i64 = row.try_get("success_count").unwrap_or(0);
            let error_count: i64 = row.try_get("error_count").unwrap_or(0);

            // Log error rate for monitoring
            if error_count > 0 {
                let error_rate = safe_i64_to_f64(error_count) / safe_i64_to_f64(usage_count);
                if error_rate > 0.1 {
                    tracing::warn!(
                        "High error rate for tool {}: {:.2}% ({} errors out of {} requests)",
                        tool_name,
                        error_rate * 100.0,
                        error_count,
                        usage_count
                    );
                }
            }

            tool_usage.push(crate::dashboard_routes::ToolUsage {
                tool_name,
                request_count: u64::try_from(usage_count)?,
                success_rate: if usage_count > 0 {
                    {
                        safe_i64_to_f64(success_count) / safe_i64_to_f64(usage_count)
                    }
                } else {
                    0.0
                },
                average_response_time: avg_response_time.unwrap_or(0.0),
            });
        }

        Ok(tool_usage)
    }

    // ================================
    // Admin Token Management
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
        let token_id = format!("admin_{}", Uuid::new_v4().simple());

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
        let expires_at = request.expires_in_days.and_then(|days| {
            i64::try_from(days)
                .ok()
                .map(|d| chrono::Utc::now() + chrono::Duration::days(d))
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            .bind(0) // usage_count
            .execute(self.inner.pool())
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
            FROM admin_tokens WHERE id = ?
        ";

        let row = sqlx::query(query)
            .bind(token_id)
            .fetch_optional(self.inner.pool())
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
            FROM admin_tokens WHERE token_prefix = ?
        ";

        let row = sqlx::query(query)
            .bind(token_prefix)
            .fetch_optional(self.inner.pool())
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
                FROM admin_tokens WHERE is_active = 1 ORDER BY created_at DESC
            "
        };

        let rows = sqlx::query(query).fetch_all(self.inner.pool()).await?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(Self::row_to_admin_token(&row)?);
        }

        Ok(tokens)
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        let query = "UPDATE admin_tokens SET is_active = 0 WHERE id = ?";

        sqlx::query(query)
            .bind(token_id)
            .execute(self.inner.pool())
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
            SET last_used_at = ?, last_used_ip = ?, usage_count = usage_count + 1
            WHERE id = ?
        ";

        sqlx::query(query)
            .bind(chrono::Utc::now())
            .bind(ip_address)
            .bind(token_id)
            .execute(self.inner.pool())
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
                error_message, response_time_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        sqlx::query(query)
            .bind(&usage.admin_token_id)
            .bind(usage.timestamp)
            .bind(usage.action.to_string())
            .bind(&usage.target_resource)
            .bind(&usage.ip_address)
            .bind(&usage.user_agent)
            .bind(usage.request_size_bytes)
            .bind(usage.success)
            .bind(&usage.error_message)
            .bind(usage.response_time_ms)
            .execute(self.inner.pool())
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
                   error_message, response_time_ms
            FROM admin_token_usage 
            WHERE admin_token_id = ? AND timestamp BETWEEN ? AND ?
            ORDER BY timestamp DESC
        ";

        let rows = sqlx::query(query)
            .bind(token_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(self.inner.pool())
            .await?;

        let mut usage_history = Vec::with_capacity(rows.len());
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            .bind(rate_limit_requests)
            .bind(rate_limit_period)
            .bind("active")
            .execute(self.inner.pool())
            .await?;

        Ok(())
    }

    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>> {
        let (query, bind_values) = admin_token_id.map_or_else(
            || {
                (
                    r"
                SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                       provisioned_at, provisioned_by_service, rate_limit_requests,
                       rate_limit_period, key_status, revoked_at, revoked_reason
                FROM admin_provisioned_keys 
                WHERE provisioned_at BETWEEN ? AND ?
                ORDER BY provisioned_at DESC
                ",
                    vec![start_date.to_rfc3339(), end_date.to_rfc3339()],
                )
            },
            |token_id| {
                (
                    r"
                SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                       provisioned_at, provisioned_by_service, rate_limit_requests,
                       rate_limit_period, key_status, revoked_at, revoked_reason
                FROM admin_provisioned_keys 
                WHERE admin_token_id = ? AND provisioned_at BETWEEN ? AND ?
                ORDER BY provisioned_at DESC
                ",
                    vec![
                        token_id.to_string(),
                        start_date.to_rfc3339(),
                        end_date.to_rfc3339(),
                    ],
                )
            },
        );

        let mut sqlx_query = sqlx::query(query);
        for value in bind_values {
            sqlx_query = sqlx_query.bind(value);
        }

        let rows = sqlx_query.fetch_all(self.inner.pool()).await?;

        let results = rows
            .iter()
            .map(Self::row_to_provisioned_key_json)
            .collect::<Vec<_>>();

        Ok(results)
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
        key_size_bits: usize,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO rsa_keypairs (kid, private_key_pem, public_key_pem, created_at, is_active, key_size_bits)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(kid) DO UPDATE SET
                private_key_pem = excluded.private_key_pem,
                public_key_pem = excluded.public_key_pem,
                is_active = excluded.is_active
            ",
        )
        .bind(kid)
        .bind(private_key_pem)
        .bind(public_key_pem)
        .bind(created_at)
        .bind(is_active)
        .bind(i64::try_from(key_size_bits).context("RSA key size exceeds maximum supported value")?)
        .execute(self.inner.pool())
        .await?;

        Ok(())
    }

    /// Load all RSA keypairs from database
    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>> {
        let rows = sqlx::query(
            "SELECT kid, private_key_pem, public_key_pem, created_at, is_active FROM rsa_keypairs ORDER BY created_at DESC",
        )
        .fetch_all(self.inner.pool())
        .await?;

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
    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()> {
        sqlx::query("UPDATE rsa_keypairs SET is_active = ?1 WHERE kid = ?2")
            .bind(is_active)
            .bind(kid)
            .execute(self.inner.pool())
            .await?;

        Ok(())
    }

    // ================================
    // Multi-Tenant Management
    // ================================

    /// Create a new tenant
    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        let query = r"
            INSERT INTO tenants (id, name, slug, domain, plan, owner_user_id, is_active, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ";

        sqlx::query(query)
            .bind(tenant.id.to_string())
            .bind(&tenant.name)
            .bind(&tenant.slug)
            .bind(&tenant.domain)
            .bind(&tenant.plan)
            .bind(tenant.owner_user_id.to_string())
            .bind(true)
            .bind(tenant.created_at)
            .bind(tenant.updated_at)
            .execute(self.inner.pool())
            .await
            .context("Failed to create tenant")?;

        // Add the owner as an admin of the tenant
        let tenant_user_query = r"
            INSERT INTO tenant_users (tenant_id, user_id, role, joined_at)
            VALUES (?1, ?2, 'owner', CURRENT_TIMESTAMP)
        ";

        sqlx::query(tenant_user_query)
            .bind(tenant.id.to_string())
            .bind(tenant.owner_user_id.to_string())
            .execute(self.inner.pool())
            .await
            .context("Failed to add owner to tenant")?;

        tracing::info!(
            "Created tenant: {} ({}) and added owner to tenant_users",
            tenant.name,
            tenant.id
        );
        Ok(())
    }

    /// Get tenant by ID
    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        let query = r"
            SELECT id, name, slug, domain, plan, owner_user_id, created_at, updated_at
            FROM tenants 
            WHERE id = ?1 AND is_active = true
        ";

        let row = sqlx::query(query)
            .bind(tenant_id.to_string())
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch tenant")?;

        match row {
            Some(row) => {
                let tenant = crate::models::Tenant {
                    id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("id"))?,
                    name: row.get("name"),
                    slug: row.get("slug"),
                    domain: row.get("domain"),
                    plan: row.get("plan"),
                    owner_user_id: crate::utils::uuid::parse_uuid(
                        &row.get::<String, _>("owner_user_id"),
                    )?,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                Ok(tenant)
            }
            None => Err(DatabaseError::NotFound {
                entity_type: "Tenant",
                entity_id: tenant_id.to_string(),
            }
            .into()),
        }
    }

    /// Get tenant by slug
    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        let query = r"
            SELECT id, name, slug, domain, plan, owner_user_id, created_at, updated_at
            FROM tenants 
            WHERE slug = ?1 AND is_active = true
        ";

        let row = sqlx::query(query)
            .bind(slug)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch tenant by slug")?;

        match row {
            Some(row) => {
                let tenant = crate::models::Tenant {
                    id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("id"))?,
                    name: row.get("name"),
                    slug: row.get("slug"),
                    domain: row.get("domain"),
                    plan: row.get("plan"),
                    owner_user_id: crate::utils::uuid::parse_uuid(
                        &row.get::<String, _>("owner_user_id"),
                    )?,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                Ok(tenant)
            }
            None => Err(DatabaseError::NotFound {
                entity_type: "Tenant",
                entity_id: slug.to_string(),
            }
            .into()),
        }
    }

    /// List tenants for a user
    async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        let query = r"
            SELECT id, name, slug, domain, plan, owner_user_id, created_at, updated_at
            FROM tenants 
            WHERE owner_user_id = ?1 AND is_active = true
            ORDER BY created_at DESC
        ";

        let rows = sqlx::query(query)
            .bind(user_id.to_string())
            .fetch_all(self.inner.pool())
            .await
            .context("Failed to fetch tenants for user")?;

        let mut tenants = Vec::with_capacity(rows.len());
        for row in rows {
            let tenant = crate::models::Tenant {
                id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("id"))?,
                name: row.get("name"),
                slug: row.get("slug"),
                domain: row.get("domain"),
                plan: row.get("plan"),
                owner_user_id: crate::utils::uuid::parse_uuid(
                    &row.get::<String, _>("owner_user_id"),
                )?,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            tenants.push(tenant);
        }

        Ok(tenants)
    }

    /// Store tenant OAuth credentials
    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        // Encrypt the client secret
        let encrypted_secret = self
            .inner
            .encrypt_data(&credentials.client_secret)
            .context("Failed to encrypt OAuth client secret")?;

        let scopes_json = serde_json::to_string(&credentials.scopes)
            .context("Failed to serialize OAuth scopes")?;

        let query = r"
            INSERT OR REPLACE INTO tenant_oauth_credentials 
            (tenant_id, provider, client_id, client_secret_encrypted, redirect_uri, scopes, rate_limit_per_day, is_active)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ";

        sqlx::query(query)
            .bind(credentials.tenant_id.to_string())
            .bind(&credentials.provider)
            .bind(&credentials.client_id)
            .bind(&encrypted_secret)
            .bind(&credentials.redirect_uri)
            .bind(&scopes_json)
            .bind(i32::try_from(credentials.rate_limit_per_day).unwrap_or(i32::MAX))
            .bind(true)
            .execute(self.inner.pool())
            .await
            .context("Failed to store tenant OAuth credentials")?;

        tracing::info!(
            "Stored OAuth credentials for tenant {} provider {}",
            credentials.tenant_id,
            credentials.provider
        );
        Ok(())
    }

    /// Get tenant OAuth providers
    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        let query = r"
            SELECT tenant_id, provider, client_id, client_secret_encrypted, redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_credentials 
            WHERE tenant_id = ?1 AND is_active = true
            ORDER BY provider
        ";

        let rows = sqlx::query(query)
            .bind(tenant_id.to_string())
            .fetch_all(self.inner.pool())
            .await
            .context("Failed to fetch tenant OAuth providers")?;

        let mut credentials = Vec::with_capacity(rows.len());
        for row in rows {
            // Decrypt the client secret
            let encrypted_secret: String = row.get("client_secret_encrypted");
            let decrypted_secret = self
                .inner
                .decrypt_data(&encrypted_secret)
                .context("Failed to decrypt OAuth client secret")?;

            let scopes_json: String = row.get("scopes");
            let scopes: Vec<String> =
                serde_json::from_str(&scopes_json).context("Failed to deserialize OAuth scopes")?;

            let cred = crate::tenant::TenantOAuthCredentials {
                tenant_id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("tenant_id"))?,
                provider: row.get("provider"),
                client_id: row.get("client_id"),
                client_secret: decrypted_secret,
                redirect_uri: row.get("redirect_uri"),
                scopes,
                rate_limit_per_day: u32::try_from(row.get::<i32, _>("rate_limit_per_day"))
                    .unwrap_or(0),
            };
            credentials.push(cred);
        }

        Ok(credentials)
    }

    /// Get tenant OAuth credentials for specific provider
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        let query = r"
            SELECT tenant_id, provider, client_id, client_secret_encrypted, redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_credentials 
            WHERE tenant_id = ?1 AND provider = ?2 AND is_active = true
        ";

        let row = sqlx::query(query)
            .bind(tenant_id.to_string())
            .bind(provider)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch tenant OAuth credentials")?;

        match row {
            Some(row) => {
                // Decrypt the client secret
                let encrypted_secret: String = row.get("client_secret_encrypted");
                let decrypted_secret = self
                    .inner
                    .decrypt_data(&encrypted_secret)
                    .context("Failed to decrypt OAuth client secret")?;

                let scopes_json: String = row.get("scopes");
                let scopes: Vec<String> = serde_json::from_str(&scopes_json)
                    .context("Failed to deserialize OAuth scopes")?;

                let cred = crate::tenant::TenantOAuthCredentials {
                    tenant_id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("tenant_id"))?,
                    provider: row.get("provider"),
                    client_id: row.get("client_id"),
                    client_secret: decrypted_secret,
                    redirect_uri: row.get("redirect_uri"),
                    scopes,
                    rate_limit_per_day: u32::try_from(row.get::<i32, _>("rate_limit_per_day"))
                        .unwrap_or(0),
                };
                Ok(Some(cred))
            }
            None => Ok(None),
        }
    }

    // ================================
    // OAuth App Registration
    // ================================

    /// Create OAuth application
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        // Hash the client secret for secure storage
        let client_secret_hash = self
            .inner
            .hash_data(&app.client_secret)
            .context("Failed to hash OAuth client secret")?;

        let redirect_uris_json = serde_json::to_string(&app.redirect_uris)
            .context("Failed to serialize redirect URIs")?;

        let scopes_json =
            serde_json::to_string(&app.scopes).context("Failed to serialize scopes")?;

        let query = r"
            INSERT INTO oauth_apps 
            (id, client_id, client_secret_hash, name, description, redirect_uris, scopes, app_type, owner_user_id, is_active, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ";

        sqlx::query(query)
            .bind(app.id.to_string())
            .bind(&app.client_id)
            .bind(&client_secret_hash)
            .bind(&app.name)
            .bind(&app.description)
            .bind(&redirect_uris_json)
            .bind(&scopes_json)
            .bind(&app.app_type)
            .bind(app.owner_user_id.to_string())
            .bind(true)
            .bind(app.created_at)
            .bind(app.updated_at)
            .execute(self.inner.pool())
            .await
            .context("Failed to create OAuth app")?;

        tracing::info!("Created OAuth app: {} ({})", app.name, app.client_id);
        Ok(())
    }

    /// Get OAuth app by client ID
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp> {
        let query = r"
            SELECT id, client_id, client_secret_hash, name, description, redirect_uris, scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps 
            WHERE client_id = ?1 AND is_active = true
        ";

        let row = sqlx::query(query)
            .bind(client_id)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch OAuth app")?;

        match row {
            Some(row) => {
                let redirect_uris_json: String = row.get("redirect_uris");
                let redirect_uris: Vec<String> = serde_json::from_str(&redirect_uris_json)
                    .context("Failed to deserialize redirect URIs")?;

                let scopes_json: String = row.get("scopes");
                let scopes: Vec<String> =
                    serde_json::from_str(&scopes_json).context("Failed to deserialize scopes")?;

                let app = crate::models::OAuthApp {
                    id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("id"))?,
                    client_id: row.get("client_id"),
                    client_secret: row.get("client_secret_hash"), // Store hash, not original
                    name: row.get("name"),
                    description: row.get("description"),
                    redirect_uris,
                    scopes,
                    app_type: row.get("app_type"),
                    owner_user_id: crate::utils::uuid::parse_uuid(
                        &row.get::<String, _>("owner_user_id"),
                    )?,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                Ok(app)
            }
            None => Err(DatabaseError::NotFound {
                entity_type: "OAuth app",
                entity_id: client_id.to_string(),
            }
            .into()),
        }
    }

    /// List OAuth apps for a user
    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        let query = r"
            SELECT id, client_id, client_secret_hash, name, description, redirect_uris, scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps 
            WHERE owner_user_id = ?1 AND is_active = true
            ORDER BY created_at DESC
        ";

        let rows = sqlx::query(query)
            .bind(user_id.to_string())
            .fetch_all(self.inner.pool())
            .await
            .context("Failed to fetch OAuth apps for user")?;

        let mut apps = Vec::with_capacity(rows.len());
        for row in rows {
            let redirect_uris_json: String = row.get("redirect_uris");
            let redirect_uris: Vec<String> = serde_json::from_str(&redirect_uris_json)
                .context("Failed to deserialize redirect URIs")?;

            let scopes_json: String = row.get("scopes");
            let scopes: Vec<String> =
                serde_json::from_str(&scopes_json).context("Failed to deserialize scopes")?;

            let app = crate::models::OAuthApp {
                id: crate::utils::uuid::parse_uuid(&row.get::<String, _>("id"))?,
                client_id: row.get("client_id"),
                client_secret: "[REDACTED]".to_string(), // Never return actual secret
                name: row.get("name"),
                description: row.get("description"),
                redirect_uris,
                scopes,
                app_type: row.get("app_type"),
                owner_user_id: crate::utils::uuid::parse_uuid(
                    &row.get::<String, _>("owner_user_id"),
                )?,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            apps.push(app);
        }

        Ok(apps)
    }

    // ================================
    // OAuth 2.0 Server (RFC 7591)
    // ================================

    /// Store OAuth 2.0 client registration
    async fn store_oauth2_client(
        &self,
        client: &crate::oauth2::models::OAuth2Client,
    ) -> Result<()> {
        let query = r"
            INSERT INTO oauth2_clients
            (id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ";

        // Serialize JSON arrays
        let redirect_uris_json = serde_json::to_string(&client.redirect_uris)
            .context("Failed to serialize redirect_uris")?;
        let grant_types_json = serde_json::to_string(&client.grant_types)
            .context("Failed to serialize grant_types")?;
        let response_types_json = serde_json::to_string(&client.response_types)
            .context("Failed to serialize response_types")?;

        sqlx::query(query)
            .bind(&client.id)
            .bind(&client.client_id)
            .bind(&client.client_secret_hash)
            .bind(&redirect_uris_json)
            .bind(&grant_types_json)
            .bind(&response_types_json)
            .bind(&client.client_name)
            .bind(&client.client_uri)
            .bind(&client.scope)
            .bind(client.created_at)
            .bind(client.expires_at)
            .execute(self.inner.pool())
            .await
            .context("Failed to store OAuth2 client")?;

        Ok(())
    }

    /// Get OAuth 2.0 client by client_id
    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2Client>> {
        let query = r"
            SELECT id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at
            FROM oauth2_clients
            WHERE client_id = ?1
        ";

        let row = sqlx::query(query)
            .bind(client_id)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch OAuth2 client")?;

        if let Some(row) = row {
            // Deserialize JSON arrays
            let redirect_uris: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("redirect_uris"))
                    .context("Failed to deserialize redirect_uris")?;
            let grant_types: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("grant_types"))
                    .context("Failed to deserialize grant_types")?;
            let response_types: Vec<String> =
                serde_json::from_str(&row.get::<String, _>("response_types"))
                    .context("Failed to deserialize response_types")?;

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

    /// Store OAuth 2.0 authorization code
    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2::models::OAuth2AuthCode,
    ) -> Result<()> {
        let query = r"
            INSERT INTO oauth2_auth_codes
            (code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ";

        sqlx::query(query)
            .bind(&auth_code.code)
            .bind(&auth_code.client_id)
            .bind(auth_code.user_id.to_string())
            .bind(&auth_code.tenant_id)
            .bind(&auth_code.redirect_uri)
            .bind(&auth_code.scope)
            .bind(auth_code.expires_at)
            .bind(auth_code.used)
            .bind(&auth_code.state)
            .bind(&auth_code.code_challenge)
            .bind(&auth_code.code_challenge_method)
            .execute(self.inner.pool())
            .await
            .context("Failed to store OAuth2 auth code")?;

        Ok(())
    }

    /// Get OAuth 2.0 authorization code
    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2AuthCode>> {
        let query = r"
            SELECT code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method
            FROM oauth2_auth_codes
            WHERE code = ?1
        ";

        let row = sqlx::query(query)
            .bind(code)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch OAuth2 auth code")?;

        if let Some(row) = row {
            let user_id = Uuid::parse_str(&row.get::<String, _>("user_id"))
                .context("Invalid user_id UUID format")?;

            Ok(Some(crate::oauth2::models::OAuth2AuthCode {
                code: row.get("code"),
                client_id: row.get("client_id"),
                user_id,
                tenant_id: row.get("tenant_id"),
                redirect_uri: row.get("redirect_uri"),
                scope: row.get("scope"),
                expires_at: row.get("expires_at"),
                used: row.get("used"),
                state: row.get("state"),
                code_challenge: row.get("code_challenge"),
                code_challenge_method: row.get("code_challenge_method"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update OAuth 2.0 authorization code (mark as used)
    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2::models::OAuth2AuthCode,
    ) -> Result<()> {
        let query = r"
            UPDATE oauth2_auth_codes
            SET used = ?1
            WHERE code = ?2
        ";

        sqlx::query(query)
            .bind(auth_code.used)
            .bind(&auth_code.code)
            .execute(self.inner.pool())
            .await
            .context("Failed to update OAuth2 auth code")?;

        Ok(())
    }

    /// Store OAuth 2.0 refresh token
    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2::models::OAuth2RefreshToken,
    ) -> Result<()> {
        let query = r"
            INSERT INTO oauth2_refresh_tokens
            (token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ";

        sqlx::query(query)
            .bind(&refresh_token.token)
            .bind(&refresh_token.client_id)
            .bind(refresh_token.user_id.to_string())
            .bind(&refresh_token.tenant_id)
            .bind(&refresh_token.scope)
            .bind(refresh_token.expires_at)
            .bind(refresh_token.created_at)
            .bind(refresh_token.revoked)
            .execute(self.inner.pool())
            .await
            .context("Failed to store OAuth2 refresh token")?;

        Ok(())
    }

    /// Get OAuth 2.0 refresh token
    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2::models::OAuth2RefreshToken>> {
        let query = r"
            SELECT token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
            FROM oauth2_refresh_tokens
            WHERE token = ?1
        ";

        let row = sqlx::query(query)
            .bind(token)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to get OAuth2 refresh token")?;

        if let Some(row) = row {
            let user_id_str: String = row.try_get("user_id")?;
            let user_id =
                Uuid::parse_str(&user_id_str).context("Failed to parse user_id as UUID")?;

            Ok(Some(crate::oauth2::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id,
                tenant_id: row.try_get("tenant_id")?,
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
        let query = r"
            UPDATE oauth2_refresh_tokens
            SET revoked = 1
            WHERE token = ?1
        ";

        sqlx::query(query)
            .bind(token)
            .execute(self.inner.pool())
            .await
            .context("Failed to revoke OAuth2 refresh token")?;

        Ok(())
    }

    /// Atomically consume OAuth 2.0 authorization code
    ///
    /// Implements atomic check-and-set using UPDATE...WHERE...RETURNING (SQLite 3.35.0+)
    /// to prevent TOCTOU race conditions in concurrent token exchange requests.
    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2::models::OAuth2AuthCode>> {
        let query = r"
            UPDATE oauth2_auth_codes
            SET used = 1
            WHERE code = ?1
              AND client_id = ?2
              AND redirect_uri = ?3
              AND used = 0
              AND expires_at > ?4
            RETURNING code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method
        ";

        let row = sqlx::query(query)
            .bind(code)
            .bind(client_id)
            .bind(redirect_uri)
            .bind(now)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to atomically consume OAuth2 auth code")?;

        if let Some(row) = row {
            let user_id = Uuid::parse_str(&row.try_get::<String, _>("user_id")?)
                .context("Invalid user_id UUID format in consumed auth code")?;

            Ok(Some(crate::oauth2::models::OAuth2AuthCode {
                code: row.try_get("code")?,
                client_id: row.try_get("client_id")?,
                user_id,
                tenant_id: row.try_get("tenant_id")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scope: row.try_get("scope")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get("used")?,
                state: row.try_get("state")?,
                code_challenge: row.try_get("code_challenge")?,
                code_challenge_method: row.try_get("code_challenge_method")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Atomically consume OAuth 2.0 refresh token
    ///
    /// Implements atomic check-and-revoke using UPDATE...WHERE...RETURNING (SQLite 3.35.0+)
    /// to prevent TOCTOU race conditions in concurrent refresh requests.
    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2::models::OAuth2RefreshToken>> {
        let query = r"
            UPDATE oauth2_refresh_tokens
            SET revoked = 1
            WHERE token = ?1
              AND client_id = ?2
              AND revoked = 0
              AND expires_at > ?3
            RETURNING token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
        ";

        let row = sqlx::query(query)
            .bind(token)
            .bind(client_id)
            .bind(now)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to atomically consume OAuth2 refresh token")?;

        if let Some(row) = row {
            let user_id_str: String = row.try_get("user_id")?;
            let user_id = Uuid::parse_str(&user_id_str)
                .context("Invalid user_id UUID format in consumed refresh token")?;

            Ok(Some(crate::oauth2::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id,
                tenant_id: row.try_get("tenant_id")?,
                scope: row.try_get("scope")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                revoked: row.try_get("revoked")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Store OAuth2 state for CSRF protection
    async fn store_oauth2_state(&self, state: &crate::oauth2::models::OAuth2State) -> Result<()> {
        let query = r"
            INSERT INTO oauth2_states
            (state, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, created_at, expires_at, used)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ";

        sqlx::query(query)
            .bind(&state.state)
            .bind(&state.client_id)
            .bind(state.user_id.map(|id| id.to_string()))
            .bind(&state.tenant_id)
            .bind(&state.redirect_uri)
            .bind(&state.scope)
            .bind(&state.code_challenge)
            .bind(&state.code_challenge_method)
            .bind(state.created_at)
            .bind(state.expires_at)
            .bind(state.used)
            .execute(self.inner.pool())
            .await
            .context("Failed to store OAuth2 state")?;

        Ok(())
    }

    /// Consume OAuth2 state (atomically check and mark as used)
    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2::models::OAuth2State>> {
        let query = r"
            UPDATE oauth2_states
            SET used = 1
            WHERE state = ?1
              AND client_id = ?2
              AND used = 0
              AND expires_at > ?3
            RETURNING state, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, created_at, expires_at, used
        ";

        let row = sqlx::query(query)
            .bind(state_value)
            .bind(client_id)
            .bind(now)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to atomically consume OAuth2 state")?;

        if let Some(row) = row {
            let user_id_str: Option<String> = row.try_get("user_id")?;
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()
                .context("Invalid user_id UUID format in consumed state")?;

            Ok(Some(crate::oauth2::models::OAuth2State {
                state: row.try_get("state")?,
                client_id: row.try_get("client_id")?,
                user_id,
                tenant_id: row.try_get("tenant_id")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scope: row.try_get("scope")?,
                code_challenge: row.try_get("code_challenge")?,
                code_challenge_method: row.try_get("code_challenge_method")?,
                created_at: row.try_get("created_at")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get("used")?,
            }))
        } else {
            Ok(None)
        }
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
        // Authorization codes expire in 10 minutes
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

        let query = r"
            INSERT INTO authorization_codes 
            (code, client_id, redirect_uri, scope, user_id, expires_at, is_used)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ";

        sqlx::query(query)
            .bind(code)
            .bind(client_id)
            .bind(redirect_uri)
            .bind(scope)
            .bind(user_id.to_string())
            .bind(expires_at)
            .bind(false)
            .execute(self.inner.pool())
            .await
            .context("Failed to store authorization code")?;

        tracing::debug!("Stored authorization code for client: {client_id}");
        Ok(())
    }

    /// Get authorization code data
    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode> {
        let query = r"
            SELECT code, client_id, redirect_uri, scope, user_id, expires_at, is_used
            FROM authorization_codes 
            WHERE code = ?1 AND is_used = false AND expires_at > CURRENT_TIMESTAMP
        ";

        let row = sqlx::query(query)
            .bind(code)
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to fetch authorization code")?;

        match row {
            Some(row) => {
                let user_id_str: Option<String> = row.get("user_id");
                let user_id = if let Some(ref uid) = user_id_str {
                    Some(crate::utils::uuid::parse_uuid(uid)?)
                } else {
                    None
                };

                let auth_code = crate::models::AuthorizationCode {
                    code: row.get("code"),
                    client_id: row.get("client_id"),
                    redirect_uri: row.get("redirect_uri"),
                    scope: row.get("scope"),
                    user_id,
                    expires_at: row.get("expires_at"),
                    created_at: row.get("created_at"),
                    is_used: row.get("is_used"),
                };
                Ok(auth_code)
            }
            None => Err(DatabaseError::NotFound {
                entity_type: "Authorization code",
                entity_id: code.to_string(),
            }
            .into()),
        }
    }

    /// Delete authorization code
    async fn delete_authorization_code(&self, code: &str) -> Result<()> {
        let query = r"
            UPDATE authorization_codes 
            SET is_used = true, used_at = CURRENT_TIMESTAMP
            WHERE code = ?1
        ";

        sqlx::query(query)
            .bind(code)
            .execute(self.inner.pool())
            .await
            .context("Failed to mark authorization code as used")?;

        tracing::debug!("Marked authorization code as used: {code}");
        Ok(())
    }

    // ================================
    // Key Rotation & Security
    // ================================

    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        let query = r"
            INSERT INTO key_versions (tenant_id, version, created_at, expires_at, is_active, algorithm)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT (tenant_id, version) DO UPDATE SET
                expires_at = excluded.expires_at,
                is_active = excluded.is_active,
                algorithm = excluded.algorithm
        ";

        sqlx::query(query)
            .bind(version.tenant_id.map(|id| id.to_string()))
            .bind(i64::from(version.version))
            .bind(version.created_at)
            .bind(version.expires_at)
            .bind(version.is_active)
            .bind(&version.algorithm)
            .execute(self.inner.pool())
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
        tenant_id: Option<uuid::Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        let query = r"
            SELECT tenant_id, version, created_at, expires_at, is_active, algorithm
            FROM key_versions
            WHERE CASE 
                WHEN ?1 IS NULL THEN tenant_id IS NULL
                ELSE tenant_id = ?1
            END
            ORDER BY version DESC
        ";

        let rows = sqlx::query(query)
            .bind(tenant_id.map(|id| id.to_string()))
            .fetch_all(self.inner.pool())
            .await
            .context("Failed to get key versions")?;

        let versions = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                let tenant_id_str: Option<String> = row.try_get("tenant_id")?;
                let tenant_id = tenant_id_str
                    .map(|s| uuid::Uuid::parse_str(&s))
                    .transpose()
                    .context("Invalid tenant UUID")?;

                Ok(crate::security::key_rotation::KeyVersion {
                    // Safe: version numbers are always positive and well within u32 range
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    version: row.try_get::<i64, _>("version")? as u32,
                    created_at: row.try_get("created_at")?,
                    expires_at: row.try_get("expires_at")?,
                    is_active: row.try_get("is_active")?,
                    tenant_id,
                    algorithm: row.try_get("algorithm")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(versions)
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<uuid::Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        let query = r"
            SELECT tenant_id, version, created_at, expires_at, is_active, algorithm
            FROM key_versions
            WHERE CASE 
                WHEN ?1 IS NULL THEN tenant_id IS NULL
                ELSE tenant_id = ?1
            END AND is_active = true
            ORDER BY version DESC
            LIMIT 1
        ";

        let row = sqlx::query(query)
            .bind(tenant_id.map(|id| id.to_string()))
            .fetch_optional(self.inner.pool())
            .await
            .context("Failed to get current key version")?;

        if let Some(row) = row {
            use sqlx::Row;
            let tenant_id_str: Option<String> = row.try_get("tenant_id")?;
            let tenant_id = tenant_id_str
                .map(|s| uuid::Uuid::parse_str(&s))
                .transpose()
                .context("Invalid tenant UUID")?;

            Ok(Some(crate::security::key_rotation::KeyVersion {
                // Safe: version numbers are always positive and well within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                version: row.try_get::<i64, _>("version")? as u32,
                created_at: row.try_get("created_at")?,
                expires_at: row.try_get("expires_at")?,
                is_active: row.try_get("is_active")?,
                tenant_id,
                algorithm: row.try_get("algorithm")?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_key_version_status(
        &self,
        tenant_id: Option<uuid::Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()> {
        // First, deactivate all versions for this tenant if we're activating a new one
        if is_active {
            let deactivate_query = r"
                UPDATE key_versions 
                SET is_active = false 
                WHERE CASE 
                    WHEN ?1 IS NULL THEN tenant_id IS NULL
                    ELSE tenant_id = ?1
                END
            ";

            sqlx::query(deactivate_query)
                .bind(tenant_id.map(|id| id.to_string()))
                .execute(self.inner.pool())
                .await
                .context("Failed to deactivate existing key versions")?;
        }

        // Now update the specific version
        let query = r"
            UPDATE key_versions 
            SET is_active = ?3 
            WHERE CASE 
                WHEN ?1 IS NULL THEN tenant_id IS NULL
                ELSE tenant_id = ?1
            END AND version = ?2
        ";

        sqlx::query(query)
            .bind(tenant_id.map(|id| id.to_string()))
            .bind(i64::from(version))
            .bind(is_active)
            .execute(self.inner.pool())
            .await
            .context("Failed to update key version status")?;

        tracing::debug!(
            "Updated key version {} status to {} for tenant {:?}",
            version,
            is_active,
            tenant_id
        );
        Ok(())
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<uuid::Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        let query = r"
            DELETE FROM key_versions
            WHERE CASE 
                WHEN ?1 IS NULL THEN tenant_id IS NULL
                ELSE tenant_id = ?1
            END
            AND version NOT IN (
                SELECT version FROM key_versions 
                WHERE CASE 
                    WHEN ?1 IS NULL THEN tenant_id IS NULL
                    ELSE tenant_id = ?1
                END
                ORDER BY version DESC 
                LIMIT ?2
            )
        ";

        let result = sqlx::query(query)
            .bind(tenant_id.map(|id| id.to_string()))
            .bind(i64::from(keep_count))
            .execute(self.inner.pool())
            .await
            .context("Failed to delete old key versions")?;

        let deleted_count = result.rows_affected();
        tracing::debug!(
            "Deleted {} old key versions for tenant {:?}",
            deleted_count,
            tenant_id
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
            .fetch_all(self.inner.pool())
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        let event_type_str = format!("{:?}", event.event_type);
        let severity_str = format!("{:?}", event.severity);
        let metadata_json = serde_json::to_string(&event.metadata)?;
        let timestamp_str = event.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

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
            .bind(&timestamp_str)
            .execute(self.inner.pool())
            .await?;

        Ok(())
    }

    // Long function: Parses all audit event types and severity levels from database strings
    #[allow(clippy::too_many_lines)]
    async fn get_audit_events(
        &self,
        tenant_id: Option<uuid::Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        use std::fmt::Write;

        let mut query = r"
            SELECT id, event_type, severity, message, source, result,
                   tenant_id, user_id, ip_address, user_agent, metadata, timestamp
            FROM audit_events
            WHERE 1=1
        "
        .to_string();

        let mut bind_count = 0;
        if tenant_id.is_some() {
            bind_count += 1;
            if write!(query, " AND tenant_id = ?{bind_count}").is_err() {
                return Err(DatabaseError::QueryError {
                    context: "Failed to write tenant_id clause to query".to_string(),
                }
                .into());
            }
        }
        if event_type.is_some() {
            bind_count += 1;
            if write!(query, " AND event_type = ?{bind_count}").is_err() {
                return Err(DatabaseError::QueryError {
                    context: "Failed to write event_type clause to query".to_string(),
                }
                .into());
            }
        }

        query.push_str(" ORDER BY timestamp DESC");

        if let Some(_limit) = limit {
            bind_count += 1;
            if write!(query, " LIMIT ?{bind_count}").is_err() {
                return Err(DatabaseError::QueryError {
                    context: "Failed to write LIMIT clause to query".to_string(),
                }
                .into());
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
            sql_query = sql_query.bind(i64::from(l));
        }

        let rows = sql_query
            .fetch_all(self.inner.pool())
            .await
            .context("Failed to get audit events")?;

        let events = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                let tenant_id_str: Option<String> = row.try_get("tenant_id")?;
                let tenant_id = tenant_id_str
                    .map(|s| uuid::Uuid::parse_str(&s))
                    .transpose()
                    .context("Invalid tenant UUID")?;

                let user_id_str: Option<String> = row.try_get("user_id")?;
                let user_id = user_id_str
                    .map(|s| uuid::Uuid::parse_str(&s))
                    .transpose()
                    .context("Invalid user UUID")?;

                let metadata_json: String = row.try_get("metadata")?;
                let metadata = serde_json::from_str(&metadata_json)
                    .context("Failed to deserialize audit event metadata")?;

                let event_type_str: String = row.try_get("event_type")?;
                let severity_str: String = row.try_get("severity")?;

                // Parse the stored enum strings back to enums
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
                    "TenantUserRemoved" => {
                        crate::security::audit::AuditEventType::TenantUserRemoved
                    }
                    "TenantUserRoleChanged" => {
                        crate::security::audit::AuditEventType::TenantUserRoleChanged
                    }
                    "DataEncrypted" => crate::security::audit::AuditEventType::DataEncrypted,
                    "DataDecrypted" => crate::security::audit::AuditEventType::DataDecrypted,
                    "KeyRotated" => crate::security::audit::AuditEventType::KeyRotated,
                    "EncryptionFailed" => crate::security::audit::AuditEventType::EncryptionFailed,
                    "ToolExecuted" => crate::security::audit::AuditEventType::ToolExecuted,
                    "ToolExecutionFailed" => {
                        crate::security::audit::AuditEventType::ToolExecutionFailed
                    }
                    "ProviderApiCalled" => {
                        crate::security::audit::AuditEventType::ProviderApiCalled
                    }
                    "ConfigurationChanged" => {
                        crate::security::audit::AuditEventType::ConfigurationChanged
                    }
                    "SystemMaintenance" => {
                        crate::security::audit::AuditEventType::SystemMaintenance
                    }
                    "SecurityPolicyViolation" => {
                        crate::security::audit::AuditEventType::SecurityPolicyViolation
                    }
                    _ => crate::security::audit::AuditEventType::SecurityPolicyViolation, // Default fallback
                };

                let severity = match severity_str.as_str() {
                    "Warning" => crate::security::audit::AuditSeverity::Warning,
                    "Error" => crate::security::audit::AuditSeverity::Error,
                    "Critical" => crate::security::audit::AuditSeverity::Critical,
                    _ => crate::security::audit::AuditSeverity::Info, // Default fallback includes "Info" and unknowns
                };

                Ok(crate::security::audit::AuditEvent {
                    event_id: uuid::Uuid::parse_str(&row.try_get::<String, _>("id")?)
                        .context("Invalid event UUID")?,
                    event_type,
                    severity,
                    timestamp: row.try_get("timestamp")?,
                    user_id,
                    tenant_id,
                    source_ip: row.try_get("ip_address")?,
                    user_agent: row.try_get("user_agent")?,
                    session_id: None, // Not stored in database schema
                    description: row.try_get("message")?,
                    metadata,
                    resource: None, // Can be extracted from metadata if needed
                    action: "security".to_string(), // Default action
                    result: row.try_get("result")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(events)
    }

    // ================================
    // User OAuth Tokens (Multi-Tenant)
    // ================================

    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> Result<()> {
        use crate::database::user_oauth_tokens::OAuthTokenData;

        let token_data = OAuthTokenData {
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

        self.inner.upsert_user_oauth_token(&token_data).await
    }

    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>> {
        self.inner
            .get_user_oauth_token(user_id, tenant_id, provider)
            .await
    }

    async fn get_user_oauth_tokens(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>> {
        self.inner.get_user_oauth_tokens(user_id).await
    }

    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>> {
        self.inner
            .get_tenant_provider_tokens(tenant_id, provider)
            .await
    }

    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        self.inner
            .delete_user_oauth_token(user_id, tenant_id, provider)
            .await
    }

    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        self.inner.delete_user_oauth_tokens(user_id).await
    }

    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        self.inner
            .refresh_user_oauth_token(
                user_id,
                tenant_id,
                provider,
                access_token,
                refresh_token,
                expires_at,
            )
            .await
    }

    /// Get user role for a specific tenant
    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>> {
        self.inner
            .get_user_tenant_role(&user_id.to_string(), &tenant_id.to_string())
            .await
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
        let encrypted_client_secret = self.inner.encrypt_data(client_secret)?;

        sqlx::query(
            r"
            INSERT OR REPLACE INTO user_oauth_app_credentials 
            (user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ",
        )
        .bind(user_id.to_string())
        .bind(provider)
        .bind(client_id)
        .bind(encrypted_client_secret)
        .bind(redirect_uri)
        .execute(self.inner.pool())
        .await?;

        Ok(())
    }

    /// Get user OAuth app credentials for a provider
    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_app_credentials
            WHERE user_id = ? AND provider = ?
            ",
        )
        .bind(user_id.to_string())
        .bind(provider)
        .fetch_optional(self.inner.pool())
        .await?;

        if let Some(row) = row {
            let encrypted_client_secret: String = row.try_get("client_secret")?;
            let decrypted_client_secret = self.inner.decrypt_data(&encrypted_client_secret)?;

            Ok(Some(UserOAuthApp {
                id: row.try_get("id")?,
                user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                provider: row.try_get("provider")?,
                client_id: row.try_get("client_id")?,
                client_secret: decrypted_client_secret,
                redirect_uri: row.try_get("redirect_uri")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all OAuth app providers configured for a user
    async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_app_credentials
            WHERE user_id = ?
            ORDER BY provider ASC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(self.inner.pool())
        .await?;

        let mut apps = Vec::new();
        for row in rows {
            let encrypted_client_secret: String = row.try_get("client_secret")?;
            let decrypted_client_secret = self.inner.decrypt_data(&encrypted_client_secret)?;

            apps.push(UserOAuthApp {
                id: row.try_get("id")?,
                user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                provider: row.try_get("provider")?,
                client_id: row.try_get("client_id")?,
                client_secret: decrypted_client_secret,
                redirect_uri: row.try_get("redirect_uri")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(apps)
    }

    /// Remove user OAuth app credentials for a provider
    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_app_credentials
            WHERE user_id = ? AND provider = ?
            ",
        )
        .bind(user_id.to_string())
        .bind(provider)
        .execute(self.inner.pool())
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
            "admin_jwt_secret" => {
                let new_secret = crate::admin::jwt::AdminJwtManager::generate_jwt_secret();
                tracing::info!("Generated new JWT secret for admin authentication");
                new_secret
            }
            _ => {
                return Err(
                    AppError::invalid_input(format!("Unknown secret type: {secret_type}")).into(),
                )
            }
        };

        // Store in database
        sqlx::query("INSERT INTO system_secrets (secret_type, secret_value) VALUES (?, ?)")
            .bind(secret_type)
            .bind(&secret_value)
            .execute(self.inner.pool())
            .await?;

        Ok(secret_value)
    }

    /// Get existing system secret
    async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        let row = sqlx::query("SELECT secret_value FROM system_secrets WHERE secret_type = ?")
            .bind(secret_type)
            .fetch_one(self.inner.pool())
            .await?;

        Ok(row.try_get("secret_value")?)
    }

    /// Update system secret (for rotation)
    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        if secret_type == "admin_jwt_secret" {
            tracing::info!("Rotating JWT secret for admin authentication");
        }

        sqlx::query(
            "UPDATE system_secrets SET secret_value = ?, updated_at = CURRENT_TIMESTAMP WHERE secret_type = ?",
        )
        .bind(new_value)
        .bind(secret_type)
        .execute(self.inner.pool())
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
        self.inner
            .store_oauth_notification(user_id, provider, success, message, expires_at)
            .await
    }

    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        self.inner.get_unread_oauth_notifications(user_id).await
    }

    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        self.inner
            .mark_oauth_notification_read(notification_id, user_id)
            .await
    }

    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        self.inner.mark_all_oauth_notifications_read(user_id).await
    }

    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        self.inner.get_all_oauth_notifications(user_id, limit).await
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
        let manager = self.inner.fitness_configurations();
        manager
            .save_tenant_config(tenant_id, configuration_name, config)
            .await
    }

    /// Save user-specific fitness configuration
    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let manager = self.inner.fitness_configurations();
        manager
            .save_user_config(tenant_id, user_id, configuration_name, config)
            .await
    }

    /// Get tenant-level fitness configuration
    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = self.inner.fitness_configurations();
        manager
            .get_tenant_config(tenant_id, configuration_name)
            .await
    }

    /// Get user-specific fitness configuration
    async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = self.inner.fitness_configurations();
        manager
            .get_user_config(tenant_id, user_id, configuration_name)
            .await
    }

    /// List all tenant-level fitness configuration names
    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let manager = self.inner.fitness_configurations();
        manager.list_tenant_configurations(tenant_id).await
    }

    /// List all user-specific fitness configuration names
    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        let manager = self.inner.fitness_configurations();
        manager.list_user_configurations(tenant_id, user_id).await
    }

    /// Delete fitness configuration (tenant or user-specific)
    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        let manager = self.inner.fitness_configurations();
        manager
            .delete_config(tenant_id, user_id, configuration_name)
            .await
    }
}

impl SqliteDatabase {
    /// Convert database row to JSON for provisioned keys
    fn row_to_provisioned_key_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
        use sqlx::Row;
        let mut record = serde_json::Map::new();

        if let Ok(id) = row.try_get::<i64, _>("id") {
            record.insert("id".into(), serde_json::Value::Number(id.into()));
        }
        if let Ok(admin_token_id) = row.try_get::<String, _>("admin_token_id") {
            record.insert(
                "admin_token_id".into(),
                serde_json::Value::String(admin_token_id),
            );
        }
        if let Ok(api_key_id) = row.try_get::<String, _>("api_key_id") {
            record.insert("api_key_id".into(), serde_json::Value::String(api_key_id));
        }
        if let Ok(user_email) = row.try_get::<String, _>("user_email") {
            record.insert("user_email".into(), serde_json::Value::String(user_email));
        }
        if let Ok(requested_tier) = row.try_get::<String, _>("requested_tier") {
            record.insert(
                "requested_tier".into(),
                serde_json::Value::String(requested_tier),
            );
        }
        if let Ok(provisioned_at) = row.try_get::<String, _>("provisioned_at") {
            record.insert(
                "provisioned_at".into(),
                serde_json::Value::String(provisioned_at),
            );
        }
        if let Ok(provisioned_by_service) = row.try_get::<String, _>("provisioned_by_service") {
            record.insert(
                "provisioned_by_service".into(),
                serde_json::Value::String(provisioned_by_service),
            );
        }
        if let Ok(rate_limit_requests) = row.try_get::<i64, _>("rate_limit_requests") {
            record.insert(
                "rate_limit_requests".into(),
                serde_json::Value::Number(rate_limit_requests.into()),
            );
        }
        if let Ok(rate_limit_period) = row.try_get::<String, _>("rate_limit_period") {
            record.insert(
                "rate_limit_period".into(),
                serde_json::Value::String(rate_limit_period),
            );
        }
        if let Ok(key_status) = row.try_get::<String, _>("key_status") {
            record.insert("key_status".into(), serde_json::Value::String(key_status));
        }
        if let Ok(revoked_at) = row.try_get::<Option<String>, _>("revoked_at") {
            record.insert(
                "revoked_at".into(),
                revoked_at.map_or(serde_json::Value::Null, serde_json::Value::String),
            );
        }
        if let Ok(revoked_reason) = row.try_get::<Option<String>, _>("revoked_reason") {
            record.insert(
                "revoked_reason".into(),
                revoked_reason.map_or(serde_json::Value::Null, serde_json::Value::String),
            );
        }

        serde_json::Value::Object(record)
    }

    /// Convert database row to `AdminToken`
    ///
    /// # Errors
    ///
    /// Returns an error if the database row cannot be converted to an `AdminToken`
    fn row_to_admin_token(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<crate::admin::models::AdminToken> {
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
            usage_count: u64::try_from(row.try_get::<i64, _>("usage_count")?)?,
        })
    }

    /// Convert database row to `AdminTokenUsage`
    ///
    /// # Errors
    ///
    /// Returns an error if the database row cannot be converted to an `AdminTokenUsage`
    fn row_to_admin_token_usage(
        row: &sqlx::sqlite::SqliteRow,
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
                .and_then(|v| u32::try_from(v).ok()),
            success: row.try_get("success")?,
            error_message: row.try_get("error_message")?,
            response_time_ms: row
                .try_get::<Option<i32>, _>("response_time_ms")?
                .and_then(|v| u32::try_from(v).ok()),
        })
    }
}
