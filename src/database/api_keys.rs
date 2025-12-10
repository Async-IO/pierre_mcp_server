// ABOUTME: API key management database operations
// ABOUTME: Handles API key generation, validation, and rate limiting storage
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::Database;
use crate::api_keys::{ApiKey, ApiKeyTier, ApiKeyUsage, ApiKeyUsageStats};
use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;
use tracing::{debug, warn};
use uuid::Uuid;

impl Database {
    /// Create a new `API` key
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn create_api_key_impl(&self, api_key: &ApiKey) -> AppResult<()> {
        // Handle enterprise tier unlimited requests by storing NULL
        let rate_limit_requests = if api_key.tier == crate::api_keys::ApiKeyTier::Enterprise {
            None
        } else {
            Some(i32::try_from(api_key.rate_limit_requests).map_err(|e| {
                AppError::internal(format!(
                    "Integer conversion failed for rate_limit_requests: {e}"
                ))
            })?)
        };

        sqlx::query(
            r"
            INSERT INTO api_keys (
                id, user_id, name, description, key_hash, key_prefix, tier,
                rate_limit_requests, rate_limit_window_seconds, is_active,
                expires_at, created_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
            )
            ",
        )
        .bind(&api_key.id)
        .bind(api_key.user_id.to_string())
        .bind(&api_key.name)
        .bind(&api_key.description)
        .bind(&api_key.key_hash)
        .bind(&api_key.key_prefix)
        .bind(api_key.tier.as_str())
        .bind(rate_limit_requests)
        .bind(
            i32::try_from(api_key.rate_limit_window_seconds).map_err(|e| {
                AppError::internal(format!(
                    "Integer conversion failed for rate_limit_window_seconds: {e}"
                ))
            })?,
        )
        .bind(api_key.is_active)
        .bind(api_key.expires_at)
        .bind(api_key.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create API key: {e}")))?;

        Ok(())
    }

    /// Get an `API` key by its prefix (for validation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_api_key_by_prefix_impl(
        &self,
        key_prefix: &str,
        key_hash: &str,
    ) -> AppResult<Option<ApiKey>> {
        let row = sqlx::query(
            r"
            SELECT * FROM api_keys
            WHERE key_prefix = $1 AND key_hash = $2 AND is_active = 1
            ",
        )
        .bind(key_prefix)
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key by prefix: {e}")))?;

        row.as_ref().map(Self::row_to_api_key).transpose()
    }

    /// Get all `API` keys for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_user_api_keys_impl(&self, user_id: Uuid) -> AppResult<Vec<ApiKey>> {
        let rows = sqlx::query(
            r"
            SELECT * FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user API keys: {e}")))?;

        rows.iter().map(Self::row_to_api_key).collect()
    }

    /// Update `API` key last used timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn update_api_key_last_used_impl(&self, api_key_id: &str) -> AppResult<()> {
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

    /// Deactivate an `API` key
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn deactivate_api_key_impl(&self, api_key_id: &str, user_id: Uuid) -> AppResult<()> {
        sqlx::query(
            r"
            UPDATE api_keys
            SET is_active = 0
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(api_key_id)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to deactivate API key: {e}")))?;

        // Idempotent operation - don't error if key doesn't exist
        Ok(())
    }

    /// Get an `API` key by `ID`
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_api_key_by_id_impl(&self, api_key_id: &str) -> AppResult<Option<ApiKey>> {
        let row = sqlx::query(
            r"
            SELECT * FROM api_keys WHERE id = $1
            ",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key by ID: {e}")))?;

        row.as_ref().map(Self::row_to_api_key).transpose()
    }

    /// Get `API` keys with filtering
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_api_keys_filtered(
        &self,
        user_id: Option<Uuid>,
        tier: Option<ApiKeyTier>,
        is_active: Option<bool>,
        limit: i32,
        offset: i32,
    ) -> AppResult<Vec<ApiKey>> {
        let mut query = String::from("SELECT * FROM api_keys WHERE 1=1");
        let mut bind_values = vec![];

        if let Some(uid) = user_id {
            query.push_str(" AND user_id = ?");
            bind_values.push(uid.to_string());
        }

        if let Some(t) = tier {
            query.push_str(" AND tier = ?");
            bind_values.push(t.as_str().to_owned());
        }

        if let Some(active) = is_active {
            query.push_str(" AND is_active = ?");
            bind_values.push(if active { "1" } else { "0" }.to_owned());
        }

        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut sql_query = sqlx::query(&query);
        for value in bind_values {
            sql_query = sql_query.bind(value);
        }
        sql_query = sql_query.bind(limit).bind(offset);

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Failed to get filtered API keys: {e}")))?;

        rows.iter().map(Self::row_to_api_key).collect()
    }

    /// Clean up expired `API` keys
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn cleanup_expired_api_keys_impl(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r"
            UPDATE api_keys
            SET is_active = 0
            WHERE expires_at IS NOT NULL 
            AND expires_at < CURRENT_TIMESTAMP 
            AND is_active = 1
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to cleanup expired API keys: {e}")))?;

        Ok(result.rows_affected())
    }

    /// Get expired `API` keys
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_expired_api_keys_impl(&self) -> AppResult<Vec<ApiKey>> {
        let rows = sqlx::query(
            r"
            SELECT * FROM api_keys
            WHERE expires_at IS NOT NULL 
            AND expires_at < CURRENT_TIMESTAMP 
            AND is_active = 1
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get expired API keys: {e}")))?;

        rows.iter().map(Self::row_to_api_key).collect()
    }

    /// Record `API` key usage
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn record_api_key_usage_impl(&self, usage: &ApiKeyUsage) -> AppResult<()> {
        sqlx::query(
            r"
            INSERT INTO api_key_usage (
                api_key_id, timestamp, tool_name, status_code,
                response_time_ms, request_size_bytes, response_size_bytes,
                ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ",
        )
        .bind(&usage.api_key_id)
        .bind(usage.timestamp)
        .bind(&usage.tool_name)
        .bind(i32::from(usage.status_code))
        .bind(
            usage
                .response_time_ms
                .map(i32::try_from)
                .transpose()
                .map_err(|e| {
                    AppError::internal(format!(
                        "Integer conversion failed for response_time_ms: {e}"
                    ))
                })?,
        )
        .bind(
            usage
                .request_size_bytes
                .map(i32::try_from)
                .transpose()
                .map_err(|e| {
                    AppError::internal(format!(
                        "Integer conversion failed for request_size_bytes: {e}"
                    ))
                })?,
        )
        .bind(
            usage
                .response_size_bytes
                .map(i32::try_from)
                .transpose()
                .map_err(|e| {
                    AppError::internal(format!(
                        "Integer conversion failed for response_size_bytes: {e}"
                    ))
                })?,
        )
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to record API key usage: {e}")))?;

        Ok(())
    }

    /// Get current usage count for an `API` key (for rate limiting)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or the API key is not found
    pub async fn get_api_key_current_usage_impl(&self, api_key_id: &str) -> AppResult<u32> {
        // Get the API key to determine its rate limit window
        let api_key = self
            .get_api_key_by_id(api_key_id)
            .await?
            .ok_or_else(|| AppError::not_found("API key"))?;

        let window_start =
            Utc::now() - Duration::seconds(i64::from(api_key.rate_limit_window_seconds));

        let count: i32 = sqlx::query_scalar(
            r"
            SELECT COUNT(*) FROM api_key_usage
            WHERE api_key_id = $1 AND timestamp > $2
            ",
        )
        .bind(api_key_id)
        .bind(window_start)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key current usage: {e}")))?;

        u32::try_from(count).map_err(|e| {
            AppError::internal(format!("Integer conversion failed for usage count: {e}"))
        })
    }

    /// Get `API` key usage statistics
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<ApiKeyUsageStats> {
        let stats = sqlx::query(
            r"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as failed_requests,
                SUM(response_time_ms) as total_response_time,
                MAX(response_time_ms) as max_response_time,
                SUM(request_size_bytes) as total_request_bytes,
                SUM(response_size_bytes) as total_response_bytes
            FROM api_key_usage
            WHERE api_key_id = $1 AND timestamp >= $2 AND timestamp <= $3
            ",
        )
        .bind(api_key_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key usage stats: {e}")))?;

        let total_requests: i32 = stats.get(0);
        let successful_requests: i32 = stats.get(1);
        let failed_requests: i32 = stats.get(2);
        let total_response_time: Option<i64> = stats.get(3);

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
        .await
        .map_err(|e| AppError::database(format!("Failed to get API key tool usage stats: {e}")))?;

        let mut tool_usage = serde_json::Map::new();
        for row in tool_usage_stats {
            let tool_name: String = row.get("tool_name");
            let tool_count: i32 = row.get("tool_count");
            let avg_response_time: Option<f64> = row.get("avg_response_time");
            let success_count: i32 = row.get("success_count");

            let avg_time = avg_response_time.unwrap_or_else(|| {
                debug!(
                    api_key_id = %api_key_id,
                    tool_name = %tool_name,
                    "No average response time available for tool usage stats"
                );
                0.0
            });

            tool_usage.insert(
                tool_name,
                serde_json::json!({
                    "count": tool_count,
                    "success_count": success_count,
                    "avg_response_time_ms": avg_time,
                    "success_rate": if tool_count > 0 { f64::from(success_count) / f64::from(tool_count) } else { 0.0 }
                }),
            );
        }

        let total_time = total_response_time.map_or(0, |t| {
            u64::try_from(t).unwrap_or_else(|e| {
                warn!(
                    api_key_id = %api_key_id,
                    total_response_time = t,
                    error = %e,
                    "Failed to convert total response time for API key usage stats, using 0"
                );
                0
            })
        });

        Ok(ApiKeyUsageStats {
            api_key_id: api_key_id.to_owned(),
            period_start: start_date,
            period_end: end_date,
            total_requests: u32::try_from(total_requests).map_err(|e| {
                AppError::internal(format!("Integer conversion failed for total_requests: {e}"))
            })?,
            successful_requests: u32::try_from(successful_requests).map_err(|e| {
                AppError::internal(format!(
                    "Integer conversion failed for successful_requests: {e}"
                ))
            })?,
            failed_requests: u32::try_from(failed_requests).map_err(|e| {
                AppError::internal(format!(
                    "Integer conversion failed for failed_requests: {e}"
                ))
            })?,
            total_response_time_ms: total_time,
            tool_usage: serde_json::Value::Object(tool_usage),
        })
    }

    /// Convert database row to `ApiKey`
    fn row_to_api_key(row: &sqlx::sqlite::SqliteRow) -> AppResult<ApiKey> {
        let tier_str: String = row.get("tier");
        let tier = tier_str
            .parse::<ApiKeyTier>()
            .map_err(|e| AppError::internal(format!("Failed to parse tier: {e}")))?;

        // Handle enterprise tier with unlimited requests (stored as NULL)
        let rate_limit_requests = if tier == crate::api_keys::ApiKeyTier::Enterprise {
            u32::MAX // Unlimited for enterprise
        } else {
            u32::try_from(row.get::<i32, _>("rate_limit_requests")).map_err(|e| {
                AppError::internal(format!(
                    "Integer conversion failed for rate_limit_requests: {e}"
                ))
            })?
        };

        Ok(ApiKey {
            id: row.get("id"),
            user_id: Uuid::parse_str(row.get::<String, _>("user_id").as_str())
                .map_err(|e| AppError::internal(format!("Failed to parse user_id UUID: {e}")))?,
            name: row.get("name"),
            description: row.get("description"),
            key_hash: row.get("key_hash"),
            key_prefix: row.get("key_prefix"),
            tier,
            rate_limit_requests,
            rate_limit_window_seconds: u32::try_from(
                row.get::<i32, _>("rate_limit_window_seconds"),
            )
            .map_err(|e| {
                AppError::internal(format!(
                    "Integer conversion failed for rate_limit_window_seconds: {e}"
                ))
            })?,
            is_active: row.get("is_active"),
            expires_at: row.get("expires_at"),
            last_used_at: row.get("last_used_at"),
            created_at: row.get("created_at"),
        })
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Create API key (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn create_api_key(&self, api_key: &ApiKey) -> AppResult<()> {
        self.create_api_key_impl(api_key).await
    }

    /// Get API key by prefix and hash (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_api_key_by_prefix(
        &self,
        key_prefix: &str,
        key_hash: &str,
    ) -> AppResult<Option<ApiKey>> {
        self.get_api_key_by_prefix_impl(key_prefix, key_hash).await
    }

    /// Get user API keys (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_user_api_keys(&self, user_id: Uuid) -> AppResult<Vec<ApiKey>> {
        self.get_user_api_keys_impl(user_id).await
    }

    /// Update API key last used timestamp (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn update_api_key_last_used(&self, api_key_id: &str) -> AppResult<()> {
        self.update_api_key_last_used_impl(api_key_id).await
    }

    /// Deactivate API key (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> AppResult<()> {
        self.deactivate_api_key_impl(api_key_id, user_id).await
    }

    /// Get API key by ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_api_key_by_id(&self, api_key_id: &str) -> AppResult<Option<ApiKey>> {
        self.get_api_key_by_id_impl(api_key_id).await
    }

    /// Cleanup expired API keys (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn cleanup_expired_api_keys(&self) -> AppResult<u64> {
        self.cleanup_expired_api_keys_impl().await
    }

    /// Get expired API keys (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_expired_api_keys(&self) -> AppResult<Vec<ApiKey>> {
        self.get_expired_api_keys_impl().await
    }

    /// Record API key usage (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> AppResult<()> {
        self.record_api_key_usage_impl(usage).await
    }

    /// Get API key current usage (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_api_key_current_usage(&self, api_key_id: &str) -> AppResult<u32> {
        self.get_api_key_current_usage_impl(api_key_id).await
    }
}
