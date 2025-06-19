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
        self.inner
            .update_strava_token(user_id, access_token, refresh_token, expires_at, scope)
            .await
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
        self.inner
            .update_fitbit_token(user_id, access_token, refresh_token, expires_at, scope)
            .await
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
            .get_user_insights(user_id, limit.map(|l| l as i32))
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

    async fn get_request_logs(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        self.inner
            .get_request_logs(api_key_id, start_time, end_time, status_filter, tool_filter)
            .await
    }

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        self.inner.get_system_stats().await
    }

    async fn create_a2a_client(&self, client: &A2AClient, api_key_id: &str) -> Result<String> {
        self.inner.create_a2a_client(client, api_key_id).await
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
        // The underlying method returns Vec<A2AUsage>, but we need to transform it
        // to match the expected signature. For now, return a stub implementation.
        let _usage = self
            .inner
            .get_a2a_client_usage_history(client_id, Some(days as i32))
            .await?;

        // TODO: Transform A2AUsage into the expected format
        // This is a simplified stub - in practice, you'd aggregate the usage data
        Ok(vec![])
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        // This method doesn't exist in the current database.rs implementation
        // For now, return an empty result. This will need to be implemented
        // by moving the logic from dashboard_routes.rs to database.rs
        let _ = (user_id, start_time, end_time); // Silence unused warnings
        Ok(vec![])
    }
}
