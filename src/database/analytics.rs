// ABOUTME: Analytics and usage tracking database operations
// ABOUTME: Stores and retrieves usage metrics and performance analytics

use super::Database;
use crate::rate_limiting::JwtUsage;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: i64,
    pub user_id: Option<Uuid>,
    pub api_key_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub endpoint: String,
    pub status_code: u16,
    pub response_time_ms: Option<u32>,
    pub error_message: Option<String>,
}

impl Database {
    /// Create analytics tables
    pub(super) async fn migrate_analytics(&self) -> Result<()> {
        // Create JWT usage tracking table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS jwt_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                endpoint TEXT NOT NULL,
                method TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                response_time_ms INTEGER,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address TEXT,
                user_agent TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create goals table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                goal_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create insights table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS insights (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                activity_id TEXT,
                insight_type TEXT NOT NULL,
                insight_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create request_logs table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
                api_key_id TEXT REFERENCES api_keys(id) ON DELETE CASCADE,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                method TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                response_time_ms INTEGER,
                error_message TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_user_id ON jwt_usage(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jwt_usage_timestamp ON jwt_usage(timestamp)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_goals_user_id ON goals(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_insights_user_id ON insights(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_insights_activity_id ON insights(activity_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_timestamp ON request_logs(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record JWT usage for rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
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
        .bind(
            usage
                .response_time_ms
                .map(|t| i32::try_from(t).unwrap_or(i32::MAX)),
        )
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
        .await?;

        Ok(())
    }

    /// Get current JWT usage count for a user (for rate limiting)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
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
        .await?;

        Ok(u32::try_from(count).unwrap_or(0))
    }

    /// Create a goal for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON serialization fails.
    pub async fn create_goal(&self, user_id: Uuid, goal_data: serde_json::Value) -> Result<String> {
        let goal_id = Uuid::new_v4().to_string();
        let goal_json = serde_json::to_string(&goal_data)?;

        sqlx::query(
            r"
            INSERT INTO goals (id, user_id, goal_data)
            VALUES ($1, $2, $3)
            ",
        )
        .bind(&goal_id)
        .bind(user_id.to_string())
        .bind(goal_json)
        .execute(&self.pool)
        .await?;

        Ok(goal_id)
    }

    /// Get all goals for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON deserialization fails.
    pub async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r"
            SELECT id, goal_data FROM goals
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

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
    pub async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        // Get the current goal data
        let row = sqlx::query(
            r"
            SELECT goal_data FROM goals WHERE id = $1
            ",
        )
        .bind(goal_id)
        .fetch_one(&self.pool)
        .await?;

        let goal_data_str: String = row.get("goal_data");
        let mut goal_data: serde_json::Value = serde_json::from_str(&goal_data_str)?;

        // Update the current_value in the JSON
        if let Some(obj) = goal_data.as_object_mut() {
            obj.insert(
                "current_value".into(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(current_value).ok_or_else(|| {
                        anyhow::anyhow!("Invalid current_value: {}", current_value)
                    })?,
                ),
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
                                anyhow::anyhow!(
                                    "Invalid progress_percentage: {}",
                                    progress_percentage
                                )
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
        .await?;

        Ok(())
    }

    /// Store an insight for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON serialization fails.
    pub async fn store_insight(
        &self,
        user_id: Uuid,
        activity_id: Option<String>,
        insight_type: &str,
        insight_data: serde_json::Value,
    ) -> Result<String> {
        let insight_id = Uuid::new_v4().to_string();
        let insight_json = serde_json::to_string(&insight_data)?;

        sqlx::query(
            r"
            INSERT INTO insights (id, user_id, activity_id, insight_type, insight_data)
            VALUES ($1, $2, $3, $4, $5)
            ",
        )
        .bind(&insight_id)
        .bind(user_id.to_string())
        .bind(activity_id)
        .bind(insight_type)
        .bind(insight_json)
        .execute(&self.pool)
        .await?;

        Ok(insight_id)
    }

    /// Get recent insights for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if JSON deserialization fails.
    pub async fn get_user_insights(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>> {
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
        .await?;

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
    ) -> Result<Vec<RequestLog>> {
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

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut logs = Vec::new();
        for row in rows {
            let user_id_str: Option<String> = row.get("user_id");
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()?;

            logs.push(RequestLog {
                id: row.get("id"),
                user_id,
                api_key_id: row.get("api_key_id"),
                timestamp: row.get("timestamp"),
                method: row.get("method"),
                endpoint: row.get("endpoint"),
                status_code: u16::try_from(row.get::<i32, _>("status_code")).unwrap_or(0),
                response_time_ms: row
                    .get::<Option<i32>, _>("response_time_ms")
                    .and_then(|t| u32::try_from(t).ok()),
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
    pub async fn get_system_stats(&self) -> Result<(u64, u64)> {
        // Get total users
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        // Get total API keys
        let api_key_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&self.pool)
            .await?;

        Ok((
            u64::try_from(user_count).unwrap_or(0),
            u64::try_from(api_key_count).unwrap_or(0),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{User, UserTier};

    async fn create_test_user(db: &Database) -> User {
        let user = User {
            id: Uuid::new_v4(),
            email: format!("analytics_{}@example.com", Uuid::new_v4()),
            display_name: Some("Analytics User".into()),
            password_hash: "hashed".into(),
            tier: UserTier::Professional,
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        };

        db.create_user(&user).await.expect("Failed to create user");
        user
    }

    #[tokio::test]
    async fn test_jwt_usage_tracking() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let user = create_test_user(&db).await;

        // Record JWT usage
        let usage = JwtUsage {
            id: None,
            user_id: user.id,
            timestamp: Utc::now(),
            endpoint: "/api/v1/profile".into(),
            method: "GET".into(),
            status_code: 200,
            response_time_ms: Some(25),
            request_size_bytes: Some(128),
            response_size_bytes: Some(512),
            ip_address: Some("192.168.1.1".into()),
            user_agent: Some("TestClient/1.0".into()),
        };

        db.record_jwt_usage(&usage)
            .await
            .expect("Failed to record JWT usage");

        // Check current usage (use a more generous time window for tests)
        let current_usage = db
            .get_jwt_current_usage(user.id)
            .await
            .expect("Failed to get current JWT usage");
        assert_eq!(current_usage, 1);
    }

    #[tokio::test]
    async fn test_goals_management() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let user = create_test_user(&db).await;

        // Create a goal
        let goal_data = serde_json::json!({
            "type": "weekly_distance",
            "target": 50.0,
            "unit": "km",
            "current": 0.0
        });

        let goal_id = db
            .create_goal(user.id, goal_data.clone())
            .await
            .expect("Failed to create goal");

        // Get user goals
        let goals = db
            .get_user_goals(user.id)
            .await
            .expect("Failed to get user goals");
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0]["type"], "weekly_distance");

        // Update goal progress
        db.update_goal_progress(&goal_id, 25.0)
            .await
            .expect("Failed to update goal progress");
    }

    #[tokio::test]
    async fn test_insights_storage() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let user = create_test_user(&db).await;

        // Store an insight
        let insight_data = serde_json::json!({
            "type": "performance_trend",
            "message": "Your pace has improved by 5% over the last month",
            "severity": "positive"
        });

        let insight_id = db
            .store_insight(
                user.id,
                Some("activity_123".into()),
                "performance",
                insight_data,
            )
            .await
            .expect("Failed to store insight");

        // Verify the insight was stored with a valid ID
        assert!(!insight_id.is_empty());

        // Get user insights
        let insights = db
            .get_user_insights(user.id, 10)
            .await
            .expect("Failed to get user insights");
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0]["type"], "performance_trend");
    }

    #[tokio::test]
    async fn test_system_stats() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        // Create multiple users
        for i in 0..3 {
            let user = User {
                id: Uuid::new_v4(),
                email: format!("stats_user_{i}@example.com"),
                display_name: None,
                password_hash: "hashed".into(),
                tier: UserTier::Starter,
                strava_token: None,
                fitbit_token: None,
                is_active: true,
                created_at: chrono::Utc::now(),
                last_active: chrono::Utc::now(),
            };
            db.create_user(&user).await.expect("Failed to create user");
        }

        // Get system stats (user_count, api_key_count)
        let (user_count, api_key_count) = db
            .get_system_stats()
            .await
            .expect("Failed to get system stats");

        assert_eq!(user_count, 3);
        assert_eq!(api_key_count, 0); // No API keys created yet
    }
}
