//! SQLite database implementation
//!
//! This module wraps the existing SQLite database functionality
//! to implement the DatabaseProvider trait.

use super::DatabaseProvider;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::database::A2AUsage;
use crate::models::{DecryptedToken, User};
use crate::rate_limiting::JwtUsage;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// SQLite database implementation
#[derive(Clone)]
pub struct SqliteDatabase {
    /// The underlying database instance
    inner: crate::database::Database,
}

impl SqliteDatabase {
    /// Get a reference to the inner database for methods not yet migrated
    pub fn inner(&self) -> &crate::database::Database {
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
        self.inner.update_strava_token(user_id, &token).await
    }

    async fn get_strava_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        self.inner.get_strava_token(user_id).await
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
        self.inner.update_fitbit_token(user_id, &token).await
    }

    async fn get_fitbit_token(&self, user_id: Uuid) -> Result<Option<DecryptedToken>> {
        self.inner.get_fitbit_token(user_id).await
    }

    async fn clear_strava_token(&self, user_id: Uuid) -> Result<()> {
        self.inner.clear_strava_token(user_id).await
    }

    async fn clear_fitbit_token(&self, user_id: Uuid) -> Result<()> {
        self.inner.clear_fitbit_token(user_id).await
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

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String> {
        self.inner
            .store_insight(user_id, None, "general", insight_data)
            .await
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>> {
        let insights = self
            .inner
            .get_user_insights(user_id, limit.unwrap_or(10) as i32)
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
                api_key_name: "Unknown".to_string(),
                tool_name: "Unknown".to_string(),
                status_code: log.status_code,
                response_time_ms: log.response_time_ms,
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

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        // Query API key usage for this user within the time range
        let query = r#"
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
        "#;

        let rows = sqlx::query(query)
            .bind(user_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_all(self.inner.pool())
            .await?;

        let mut tool_usage = Vec::new();
        for row in rows {
            use sqlx::Row;

            let tool_name: String = row
                .try_get("tool_name")
                .unwrap_or_else(|_| "unknown".to_string());
            let usage_count: i64 = row.try_get("usage_count").unwrap_or(0);
            let avg_response_time: Option<f64> = row.try_get("avg_response_time").ok();
            let success_count: i64 = row.try_get("success_count").unwrap_or(0);
            let _error_count: i64 = row.try_get("error_count").unwrap_or(0);

            tool_usage.push(crate::dashboard_routes::ToolUsage {
                tool_name,
                request_count: usage_count as u64,
                success_rate: if usage_count > 0 {
                    (success_count as f64) / (usage_count as f64)
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
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        use crate::admin::{
            jwt::AdminJwtManager,
            models::{AdminPermissions, GeneratedAdminToken},
        };
        use uuid::Uuid;

        // Generate unique token ID
        let token_id = format!("admin_{}", Uuid::new_v4().simple());

        // Generate JWT secret and manager
        let jwt_secret = AdminJwtManager::generate_jwt_secret();
        let jwt_manager = AdminJwtManager::with_secret(&jwt_secret);

        // Get permissions
        let permissions = match &request.permissions {
            Some(perms) => AdminPermissions::new(perms.clone()),
            None => {
                if request.is_super_admin {
                    AdminPermissions::super_admin()
                } else {
                    AdminPermissions::default_admin()
                }
            }
        };

        // Calculate expiration
        let expires_at = request
            .expires_in_days
            .map(|days| chrono::Utc::now() + chrono::Duration::days(days as i64));

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
        let query = r#"
            INSERT INTO admin_tokens (
                id, service_name, service_description, token_hash, token_prefix,
                jwt_secret_hash, permissions, is_super_admin, is_active,
                created_at, expires_at, usage_count
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

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
        let query = r#"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE id = ?
        "#;

        let row = sqlx::query(query)
            .bind(token_id)
            .fetch_optional(self.inner.pool())
            .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_admin_token(row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        let query = r#"
            SELECT id, service_name, service_description, token_hash, token_prefix,
                   jwt_secret_hash, permissions, is_super_admin, is_active,
                   created_at, expires_at, last_used_at, last_used_ip, usage_count
            FROM admin_tokens WHERE token_prefix = ?
        "#;

        let row = sqlx::query(query)
            .bind(token_prefix)
            .fetch_optional(self.inner.pool())
            .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_admin_token(row)?))
        } else {
            Ok(None)
        }
    }

    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        let query = if include_inactive {
            r#"
                SELECT id, service_name, service_description, token_hash, token_prefix,
                       jwt_secret_hash, permissions, is_super_admin, is_active,
                       created_at, expires_at, last_used_at, last_used_ip, usage_count
                FROM admin_tokens ORDER BY created_at DESC
            "#
        } else {
            r#"
                SELECT id, service_name, service_description, token_hash, token_prefix,
                       jwt_secret_hash, permissions, is_super_admin, is_active,
                       created_at, expires_at, last_used_at, last_used_ip, usage_count
                FROM admin_tokens WHERE is_active = 1 ORDER BY created_at DESC
            "#
        };

        let rows = sqlx::query(query).fetch_all(self.inner.pool()).await?;

        let mut tokens = Vec::new();
        for row in rows {
            tokens.push(self.row_to_admin_token(row)?);
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
        let query = r#"
            UPDATE admin_tokens 
            SET last_used_at = ?, last_used_ip = ?, usage_count = usage_count + 1
            WHERE id = ?
        "#;

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
        let query = r#"
            INSERT INTO admin_token_usage (
                admin_token_id, timestamp, action, target_resource,
                ip_address, user_agent, request_size_bytes, success,
                error_message, response_time_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

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
        let query = r#"
            SELECT id, admin_token_id, timestamp, action, target_resource,
                   ip_address, user_agent, request_size_bytes, success,
                   error_message, response_time_ms
            FROM admin_token_usage 
            WHERE admin_token_id = ? AND timestamp BETWEEN ? AND ?
            ORDER BY timestamp DESC
        "#;

        let rows = sqlx::query(query)
            .bind(token_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(self.inner.pool())
            .await?;

        let mut usage_history = Vec::new();
        for row in rows {
            usage_history.push(self.row_to_admin_token_usage(row)?);
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
        let query = r#"
            INSERT INTO admin_provisioned_keys (
                admin_token_id, api_key_id, user_email, requested_tier,
                provisioned_at, provisioned_by_service, rate_limit_requests,
                rate_limit_period, key_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        // Get service name from admin token
        let service_name = if let Some(token) = self.get_admin_token_by_id(admin_token_id).await? {
            token.service_name
        } else {
            "unknown".to_string()
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
        use sqlx::Row;

        let (query, bind_values) = if let Some(token_id) = admin_token_id {
            (
                r#"
                SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                       provisioned_at, provisioned_by_service, rate_limit_requests,
                       rate_limit_period, key_status, revoked_at, revoked_reason
                FROM admin_provisioned_keys 
                WHERE admin_token_id = ? AND provisioned_at BETWEEN ? AND ?
                ORDER BY provisioned_at DESC
                "#,
                vec![
                    token_id.to_string(),
                    start_date.to_rfc3339(),
                    end_date.to_rfc3339(),
                ],
            )
        } else {
            (
                r#"
                SELECT id, admin_token_id, api_key_id, user_email, requested_tier,
                       provisioned_at, provisioned_by_service, rate_limit_requests,
                       rate_limit_period, key_status, revoked_at, revoked_reason
                FROM admin_provisioned_keys 
                WHERE provisioned_at BETWEEN ? AND ?
                ORDER BY provisioned_at DESC
                "#,
                vec![start_date.to_rfc3339(), end_date.to_rfc3339()],
            )
        };

        let mut sqlx_query = sqlx::query(query);
        for value in bind_values {
            sqlx_query = sqlx_query.bind(value);
        }

        let rows = sqlx_query.fetch_all(self.inner.pool()).await?;

        let mut results = Vec::new();
        for row in rows {
            let mut record = serde_json::Map::new();

            if let Ok(id) = row.try_get::<i64, _>("id") {
                record.insert("id".to_string(), serde_json::Value::Number(id.into()));
            }
            if let Ok(admin_token_id) = row.try_get::<String, _>("admin_token_id") {
                record.insert(
                    "admin_token_id".to_string(),
                    serde_json::Value::String(admin_token_id),
                );
            }
            if let Ok(api_key_id) = row.try_get::<String, _>("api_key_id") {
                record.insert(
                    "api_key_id".to_string(),
                    serde_json::Value::String(api_key_id),
                );
            }
            if let Ok(user_email) = row.try_get::<String, _>("user_email") {
                record.insert(
                    "user_email".to_string(),
                    serde_json::Value::String(user_email),
                );
            }
            if let Ok(requested_tier) = row.try_get::<String, _>("requested_tier") {
                record.insert(
                    "requested_tier".to_string(),
                    serde_json::Value::String(requested_tier),
                );
            }
            if let Ok(provisioned_at) = row.try_get::<String, _>("provisioned_at") {
                record.insert(
                    "provisioned_at".to_string(),
                    serde_json::Value::String(provisioned_at),
                );
            }
            if let Ok(provisioned_by_service) = row.try_get::<String, _>("provisioned_by_service") {
                record.insert(
                    "provisioned_by_service".to_string(),
                    serde_json::Value::String(provisioned_by_service),
                );
            }
            if let Ok(rate_limit_requests) = row.try_get::<i64, _>("rate_limit_requests") {
                record.insert(
                    "rate_limit_requests".to_string(),
                    serde_json::Value::Number(rate_limit_requests.into()),
                );
            }
            if let Ok(rate_limit_period) = row.try_get::<String, _>("rate_limit_period") {
                record.insert(
                    "rate_limit_period".to_string(),
                    serde_json::Value::String(rate_limit_period),
                );
            }
            if let Ok(key_status) = row.try_get::<String, _>("key_status") {
                record.insert(
                    "key_status".to_string(),
                    serde_json::Value::String(key_status),
                );
            }
            if let Ok(revoked_at) = row.try_get::<Option<String>, _>("revoked_at") {
                record.insert(
                    "revoked_at".to_string(),
                    revoked_at
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
            }
            if let Ok(revoked_reason) = row.try_get::<Option<String>, _>("revoked_reason") {
                record.insert(
                    "revoked_reason".to_string(),
                    revoked_reason
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
            }

            results.push(serde_json::Value::Object(record));
        }

        Ok(results)
    }
}

impl SqliteDatabase {
    /// Convert database row to AdminToken
    fn row_to_admin_token(
        &self,
        row: sqlx::sqlite::SqliteRow,
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
            usage_count: row.try_get::<i64, _>("usage_count")? as u64,
        })
    }

    /// Convert database row to AdminTokenUsage
    fn row_to_admin_token_usage(
        &self,
        row: sqlx::sqlite::SqliteRow,
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
                .map(|v| v as u32),
            success: row.try_get("success")?,
            error_message: row.try_get("error_message")?,
            response_time_ms: row
                .try_get::<Option<i32>, _>("response_time_ms")?
                .map(|v| v as u32),
        })
    }
}
