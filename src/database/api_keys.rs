// ABOUTME: API key management database operations
// ABOUTME: Handles API key generation, validation, and rate limiting storage

use super::Database;
use crate::api_keys::{ApiKey, ApiKeyTier, ApiKeyUsage, ApiKeyUsageStats};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

impl Database {
    /// Create `API` key tables
    pub(super) async fn migrate_api_keys(&self) -> Result<()> {
        // Create api_keys table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                description TEXT,
                key_hash TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                tier TEXT NOT NULL CHECK (tier IN ('trial', 'starter', 'professional', 'enterprise')),
                rate_limit_requests INTEGER,
                rate_limit_window_seconds INTEGER,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                expires_at DATETIME,
                last_used_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(key_hash),
                UNIQUE(key_prefix)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create api_key_usage table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS api_key_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                tool_name TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                response_time_ms INTEGER,
                error_message TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address TEXT,
                user_agent TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_key_prefix ON api_keys(key_prefix)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_key_id ON api_key_usage(api_key_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_key_usage_timestamp ON api_key_usage(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a new `API` key
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        // Handle enterprise tier unlimited requests by storing NULL
        let rate_limit_requests = if api_key.tier == crate::api_keys::ApiKeyTier::Enterprise {
            None
        } else {
            Some(i32::try_from(api_key.rate_limit_requests)?)
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
        .bind(i32::try_from(api_key.rate_limit_window_seconds)?)
        .bind(api_key.is_active)
        .bind(api_key.expires_at)
        .bind(api_key.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an `API` key by its prefix (for validation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_api_key_by_prefix(
        &self,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<Option<ApiKey>> {
        let row = sqlx::query(
            r"
            SELECT * FROM api_keys
            WHERE key_prefix = $1 AND key_hash = $2 AND is_active = 1
            ",
        )
        .bind(key_prefix)
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(Self::row_to_api_key).transpose()
    }

    /// Get all `API` keys for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r"
            SELECT * FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(Self::row_to_api_key).collect()
    }

    /// Update `API` key last used timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
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

    /// Deactivate an `API` key
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
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
        .await?;

        // Idempotent operation - don't error if key doesn't exist
        Ok(())
    }

    /// Get an `API` key by `ID`
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>> {
        let row = sqlx::query(
            r"
            SELECT * FROM api_keys WHERE id = $1
            ",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await?;

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
    ) -> Result<Vec<ApiKey>> {
        let mut query = String::from("SELECT * FROM api_keys WHERE 1=1");
        let mut bind_values = vec![];

        if let Some(uid) = user_id {
            query.push_str(" AND user_id = ?");
            bind_values.push(uid.to_string());
        }

        if let Some(t) = tier {
            query.push_str(" AND tier = ?");
            bind_values.push(t.as_str().to_string());
        }

        if let Some(active) = is_active {
            query.push_str(" AND is_active = ?");
            bind_values.push(if active { "1" } else { "0" }.to_string());
        }

        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut sql_query = sqlx::query(&query);
        for value in bind_values {
            sql_query = sql_query.bind(value);
        }
        sql_query = sql_query.bind(limit).bind(offset);

        let rows = sql_query.fetch_all(&self.pool).await?;

        rows.iter().map(Self::row_to_api_key).collect()
    }

    /// Clean up expired `API` keys
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn cleanup_expired_api_keys(&self) -> Result<u64> {
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
        .await?;

        Ok(result.rows_affected())
    }

    /// Get expired `API` keys
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r"
            SELECT * FROM api_keys
            WHERE expires_at IS NOT NULL 
            AND expires_at < CURRENT_TIMESTAMP 
            AND is_active = 1
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(Self::row_to_api_key).collect()
    }

    /// Record `API` key usage
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
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
        .bind(usage.response_time_ms.map(i32::try_from).transpose()?)
        .bind(usage.request_size_bytes.map(i32::try_from).transpose()?)
        .bind(usage.response_size_bytes.map(i32::try_from).transpose()?)
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get current usage count for an `API` key (for rate limiting)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or the API key is not found
    pub async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        // Get the API key to determine its rate limit window
        let api_key = self
            .get_api_key_by_id(api_key_id)
            .await?
            .ok_or_else(|| anyhow!("API key not found"))?;

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
        .await?;

        Ok(u32::try_from(count)?)
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
    ) -> Result<ApiKeyUsageStats> {
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
        .await?;

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
        .await?;

        let mut tool_usage = serde_json::Map::new();
        for row in tool_usage_stats {
            let tool_name: String = row.get("tool_name");
            let tool_count: i32 = row.get("tool_count");
            let avg_response_time: Option<f64> = row.get("avg_response_time");
            let success_count: i32 = row.get("success_count");

            tool_usage.insert(
                tool_name,
                serde_json::json!({
                    "count": tool_count,
                    "success_count": success_count,
                    "avg_response_time_ms": avg_response_time.unwrap_or(0.0),
                    "success_rate": if tool_count > 0 { f64::from(success_count) / f64::from(tool_count) } else { 0.0 }
                }),
            );
        }

        Ok(ApiKeyUsageStats {
            api_key_id: api_key_id.to_string(),
            period_start: start_date,
            period_end: end_date,
            total_requests: u32::try_from(total_requests)?,
            successful_requests: u32::try_from(successful_requests)?,
            failed_requests: u32::try_from(failed_requests)?,
            total_response_time_ms: total_response_time
                .map_or(0, |t| u64::try_from(t).unwrap_or(0)),
            tool_usage: serde_json::Value::Object(tool_usage),
        })
    }

    /// Convert database row to `ApiKey`
    fn row_to_api_key(row: &sqlx::sqlite::SqliteRow) -> Result<ApiKey> {
        let tier_str: String = row.get("tier");
        let tier = tier_str.parse::<ApiKeyTier>()?;

        // Handle enterprise tier with unlimited requests (stored as NULL)
        let rate_limit_requests = if tier == crate::api_keys::ApiKeyTier::Enterprise {
            u32::MAX // Unlimited for enterprise
        } else {
            u32::try_from(row.get::<i32, _>("rate_limit_requests"))?
        };

        Ok(ApiKey {
            id: row.get("id"),
            user_id: Uuid::parse_str(row.get::<String, _>("user_id").as_str())?,
            name: row.get("name"),
            description: row.get("description"),
            key_hash: row.get("key_hash"),
            key_prefix: row.get("key_prefix"),
            tier,
            rate_limit_requests,
            rate_limit_window_seconds: u32::try_from(
                row.get::<i32, _>("rate_limit_window_seconds"),
            )?,
            is_active: row.get("is_active"),
            expires_at: row.get("expires_at"),
            last_used_at: row.get("last_used_at"),
            created_at: row.get("created_at"),
        })
    }
}
