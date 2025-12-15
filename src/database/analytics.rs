// ABOUTME: Analytics and usage tracking database operations
// ABOUTME: Stores and retrieves usage metrics and performance analytics
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::dashboard_routes::ToolUsage;
use crate::errors::{AppError, AppResult};
use crate::rate_limiting::JwtUsage;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::{error, warn};
use uuid::Uuid;

/// Log entry for API requests including timing and status information
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestLog {
    /// Database record ID (UUID stored as text in database)
    pub id: String,
    /// Optional user ID for authenticated requests
    pub user_id: Option<Uuid>,
    /// Optional API key ID used for the request
    pub api_key_id: Option<String>,
    /// When the request was made
    pub timestamp: DateTime<Utc>,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// API endpoint path
    pub endpoint: String,
    /// HTTP response status code
    pub status_code: u16,
    /// Response time in milliseconds
    pub response_time_ms: Option<u32>,
    /// Error message if request failed
    pub error_message: Option<String>,
}

impl Database {
    /// Record JWT usage for rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn record_jwt_usage_impl(&self, usage: &JwtUsage) -> AppResult<()> {
        sqlx::query(
            r"
            INSERT INTO jwt_usage (
                user_id, timestamp, endpoint, method, status_code,
                response_time_ms, request_size_bytes, response_size_bytes,
                ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ",
        )
        .bind(usage.user_id.to_string())
        .bind(usage.timestamp)
        .bind(&usage.endpoint)
        .bind(&usage.method)
        .bind(i32::from(usage.status_code))
        .bind(usage.response_time_ms.map(|t| {
            i32::try_from(t).unwrap_or_else(|e| {
                warn!(
                    user_id = ?usage.user_id,
                    endpoint = %usage.endpoint,
                    response_time_ms = t,
                    fallback = i32::MAX,
                    error = %e,
                    "Response time conversion failed for usage recording, using i32::MAX"
                );
                i32::MAX
            })
        }))
        .bind(usage.request_size_bytes.map(|s| {
            i32::try_from(s).unwrap_or_else(|e| {
                warn!(
                    user_id = ?usage.user_id,
                    endpoint = %usage.endpoint,
                    request_size_bytes = s,
                    fallback = i32::MAX,
                    error = %e,
                    "Request size conversion failed for usage recording, using i32::MAX"
                );
                i32::MAX
            })
        }))
        .bind(usage.response_size_bytes.map(|s| {
            i32::try_from(s).unwrap_or_else(|e| {
                warn!(
                    user_id = ?usage.user_id,
                    endpoint = %usage.endpoint,
                    response_size_bytes = s,
                    fallback = i32::MAX,
                    error = %e,
                    "Response size conversion failed for usage recording, using i32::MAX"
                );
                i32::MAX
            })
        }))
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to record JWT usage: {e}")))?;

        Ok(())
    }

    /// Get current JWT usage count for a user (for rate limiting)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_jwt_current_usage_impl(&self, user_id: Uuid) -> AppResult<u32> {
        let window_start = Utc::now() - Duration::hours(1); // 1 hour window

        let count: i32 = sqlx::query_scalar(
            r"
            SELECT COUNT(*) FROM jwt_usage
            WHERE user_id = $1 AND timestamp > $2
            ",
        )
        .bind(user_id.to_string())
        .bind(window_start)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get JWT current usage: {e}")))?;

        Ok(u32::try_from(count).unwrap_or_else(|e| {
            error!(
                user_id = %user_id,
                count = count,
                error = %e,
                "Rate limiting: negative JWT usage count detected, using 0 (potential security issue)"
            );
            0
        }))
    }

    /// Create a goal for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON serialization fails.
    pub async fn create_goal_impl(
        &self,
        user_id: Uuid,
        goal_data: serde_json::Value,
    ) -> AppResult<String> {
        let goal_id = Uuid::new_v4().to_string();
        let goal_json = serde_json::to_string(&goal_data)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO goals (id, user_id, goal_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            ",
        )
        .bind(&goal_id)
        .bind(user_id.to_string())
        .bind(goal_json)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create goal: {e}")))?;

        Ok(goal_id)
    }

    /// Get all goals for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON deserialization fails.
    pub async fn get_user_goals_impl(&self, user_id: Uuid) -> AppResult<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r"
            SELECT id, goal_data FROM goals
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user goals: {e}")))?;

        let mut goals = Vec::new();
        for row in rows {
            let goal_id: String = row.get("id");
            let goal_json: String = row.get("goal_data");
            let mut goal: serde_json::Value = serde_json::from_str(&goal_json)?;

            // Add the goal ID to the JSON object
            if let serde_json::Value::Object(ref mut map) = goal {
                map.insert("id".into(), serde_json::Value::String(goal_id));
            }

            goals.push(goal);
        }

        Ok(goals)
    }

    /// Update goal progress
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON operations fail.
    pub async fn update_goal_progress_impl(
        &self,
        goal_id: &str,
        current_value: f64,
    ) -> AppResult<()> {
        // Get the current goal data
        let row = sqlx::query(
            r"
            SELECT goal_data FROM goals WHERE id = $1
            ",
        )
        .bind(goal_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get goal data for update: {e}")))?;

        let goal_data_str: String = row.get("goal_data");
        let mut goal_data: serde_json::Value = serde_json::from_str(&goal_data_str)?;

        // Update the current_value in the JSON
        if let Some(obj) = goal_data.as_object_mut() {
            obj.insert(
                "current_value".into(),
                serde_json::Value::Number(serde_json::Number::from_f64(current_value).ok_or_else(
                    || AppError::internal(format!("Invalid current_value: {current_value}")),
                )?),
            );

            // Update last_updated timestamp
            obj.insert(
                "last_updated".into(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );

            // Calculate progress percentage if target_value exists
            if let Some(target) = obj.get("target_value").and_then(serde_json::Value::as_f64) {
                if target > 0.0 {
                    let progress_percentage = (current_value / target * 100.0).clamp(0.0, 100.0);
                    obj.insert(
                        "progress_percentage".into(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(progress_percentage).ok_or_else(|| {
                                AppError::internal(format!(
                                    "Invalid progress_percentage: {progress_percentage}"
                                ))
                            })?,
                        ),
                    );
                }
            }
        }

        // Save the updated goal data back to the database
        let updated_goal_json = serde_json::to_string(&goal_data)?;

        sqlx::query(
            r"
            UPDATE goals
            SET goal_data = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            ",
        )
        .bind(updated_goal_json)
        .bind(goal_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to update goal progress: {e}")))?;

        Ok(())
    }

    /// Store an insight for a user (full 4-parameter version)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON serialization fails.
    pub async fn store_insight_full(
        &self,
        user_id: Uuid,
        activity_id: Option<String>,
        insight_type: &str,
        insight_data: serde_json::Value,
    ) -> AppResult<String> {
        let insight_id = Uuid::new_v4().to_string();
        let insight_json = serde_json::to_string(&insight_data)?;
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO insights (id, user_id, activity_id, insight_type, insight_data, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ",
        )
        .bind(&insight_id)
        .bind(user_id.to_string())
        .bind(activity_id)
        .bind(insight_type)
        .bind(insight_json)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store insight: {e}")))?;

        Ok(insight_id)
    }

    /// Store an insight for a user (simplified 2-parameter version for trait compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON serialization fails.
    pub async fn store_insight_impl(
        &self,
        user_id: Uuid,
        insight_data: serde_json::Value,
    ) -> AppResult<String> {
        // Extract insight type from the JSON data or use a default
        let insight_type = insight_data
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        // Call the full 4-parameter version with defaults
        self.store_insight_full(user_id, None, insight_type, insight_data.clone()) // Safe: JSON value ownership for insight storage
            .await
    }

    /// Get recent insights for a user (trait-compatible 3-parameter version)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON deserialization fails.
    pub async fn get_user_insights(
        &self,
        user_id: Uuid,
        _insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<serde_json::Value>> {
        // Safe: limit represents small positive query limit (1-1000)
        #[allow(clippy::cast_possible_wrap)]
        let actual_limit = limit.unwrap_or(10) as i32;
        self.get_user_insights_simple(user_id, actual_limit).await
    }

    /// Get recent insights for a user (simple 2-parameter version)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON deserialization fails.
    pub async fn get_user_insights_simple(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> AppResult<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r"
            SELECT insight_data FROM insights
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            ",
        )
        .bind(user_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user insights: {e}")))?;

        let mut insights = Vec::new();
        for row in rows {
            let insight_json: String = row.get("insight_data");
            let insight: serde_json::Value = serde_json::from_str(&insight_json)?;
            insights.push(insight);
        }

        Ok(insights)
    }

    /// Get request logs with filtering
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if UUID parsing fails.
    pub async fn get_request_logs(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i32,
        offset: i32,
    ) -> AppResult<Vec<RequestLog>> {
        let mut query = String::from(
            r"
            SELECT id, user_id, api_key_id, timestamp, method, endpoint, 
                   status_code, response_time_ms, error_message
            FROM request_logs
            WHERE 1=1
            ",
        );

        let mut bind_values = vec![];

        if let Some(uid) = user_id {
            query.push_str(" AND user_id = ?");
            bind_values.push(uid.to_string());
        }

        if let Some(start) = start_date {
            query.push_str(" AND timestamp >= ?");
            bind_values.push(start.to_rfc3339());
        }

        if let Some(end) = end_date {
            query.push_str(" AND timestamp <= ?");
            bind_values.push(end.to_rfc3339());
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut sql_query = sqlx::query(&query);
        for value in bind_values {
            sql_query = sql_query.bind(value);
        }
        sql_query = sql_query.bind(limit).bind(offset);

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get request logs: {e}")))?;

        let mut logs = Vec::new();
        for row in rows {
            let user_id_str: Option<String> = row.get("user_id");
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()?;

            let log_id: String = row.get("id");
            let status_code = u16::try_from(row.get::<i32, _>("status_code")).unwrap_or_else(|e| {
                warn!(
                    log_id = %log_id,
                    user_id = ?user_id,
                    error = %e,
                    "Failed to convert status_code for request log, using 0"
                );
                0
            });

            let response_time_ms = row.get::<Option<i32>, _>("response_time_ms").and_then(|t| {
                u32::try_from(t)
                    .inspect_err(|e| {
                        warn!(
                            log_id = %log_id,
                            user_id = ?user_id,
                            response_time_i32 = t,
                            error = %e,
                            "Failed to convert response_time_ms for request log"
                        );
                    })
                    .ok()
            });

            logs.push(RequestLog {
                id: log_id,
                user_id,
                api_key_id: row.get("api_key_id"),
                timestamp: row.get("timestamp"),
                method: row.get("method"),
                endpoint: row.get("endpoint"),
                status_code,
                response_time_ms,
                error_message: row.get("error_message"),
            });
        }

        Ok(logs)
    }

    /// Get system-wide statistics
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_system_stats_impl(&self) -> AppResult<(u64, u64)> {
        // Get total users
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user count: {e}")))?;

        // Get total API keys
        let api_key_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get API key count: {e}")))?;

        Ok((
            u64::try_from(user_count).unwrap_or_else(|e| {
                error!(
                    user_count = user_count,
                    error = %e,
                    operation = "get_system_stats",
                    "System stats: negative user count detected (database integrity issue), using 0"
                );
                0
            }),
            u64::try_from(api_key_count).unwrap_or_else(|e| {
                error!(
                    api_key_count = api_key_count,
                    error = %e,
                    operation = "get_system_stats",
                    "System stats: negative API key count detected (database integrity issue), using 0"
                );
                0
            }),
        ))
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Record JWT usage (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn record_jwt_usage(&self, usage: &JwtUsage) -> AppResult<()> {
        self.record_jwt_usage_impl(usage).await
    }

    /// Get JWT current usage (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_jwt_current_usage(&self, user_id: Uuid) -> AppResult<u32> {
        self.get_jwt_current_usage_impl(user_id).await
    }

    /// Create user goal (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn create_goal(
        &self,
        user_id: Uuid,
        goal_data: serde_json::Value,
    ) -> AppResult<String> {
        self.create_goal_impl(user_id, goal_data).await
    }

    /// Get user goals (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_goals(&self, user_id: Uuid) -> AppResult<Vec<serde_json::Value>> {
        self.get_user_goals_impl(user_id).await
    }

    /// Update goal progress (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> AppResult<()> {
        self.update_goal_progress_impl(goal_id, current_value).await
    }

    /// Store user insight (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn store_insight(
        &self,
        user_id: Uuid,
        insight_data: serde_json::Value,
    ) -> AppResult<String> {
        self.store_insight_impl(user_id, insight_data).await
    }

    /// Get system statistics (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_system_stats(&self) -> AppResult<(u64, u64)> {
        self.get_system_stats_impl().await
    }

    /// Get top tools analysis for a user within a time range (internal implementation)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_top_tools_analysis_impl(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AppResult<Vec<ToolUsage>> {
        let rows = sqlx::query(
            r"
            SELECT
                aku.endpoint,
                COUNT(*) as usage_count,
                AVG(CAST(aku.response_time_ms AS REAL)) as avg_response_time,
                COUNT(CASE WHEN aku.status_code < 400 THEN 1 END) as success_count,
                COUNT(CASE WHEN aku.status_code >= 400 THEN 1 END) as error_count
            FROM api_key_usage aku
            JOIN api_keys ak ON aku.api_key_id = ak.id
            WHERE ak.user_id = $1 AND aku.timestamp BETWEEN $2 AND $3
            GROUP BY aku.endpoint
            ORDER BY usage_count DESC
            LIMIT 10
            ",
        )
        .bind(user_id.to_string())
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get top tools analysis: {e}")))?;

        let mut tool_usage = Vec::with_capacity(rows.len());
        for row in rows {
            let endpoint: String = row.get("endpoint");
            let usage_count: i64 = row.get("usage_count");
            let success_count: i64 = row.get("success_count");
            let avg_response_time: Option<f64> = row.get("avg_response_time");

            #[allow(clippy::cast_precision_loss)]
            let success_rate = if usage_count > 0 {
                (success_count as f64 / usage_count as f64) * 100.0
            } else {
                0.0
            };

            tool_usage.push(ToolUsage {
                tool_name: endpoint,
                #[allow(clippy::cast_sign_loss)]
                request_count: usage_count as u64,
                success_rate,
                average_response_time: avg_response_time.unwrap_or(0.0),
            });
        }

        Ok(tool_usage)
    }

    /// Get top tools analysis for a user (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AppResult<Vec<ToolUsage>> {
        self.get_top_tools_analysis_impl(user_id, start_time, end_time)
            .await
    }
}
