//! Analytics and usage tracking database operations

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
            r#"
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
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create goals table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                goal_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create insights table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS insights (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                activity_id TEXT,
                insight_type TEXT NOT NULL,
                insight_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create request_logs table
        sqlx::query(
            r#"
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
            "#,
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
    pub async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO jwt_usage (
                user_id, timestamp, endpoint, method, status_code,
                response_time_ms, request_size_bytes, response_size_bytes,
                ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(usage.user_id.to_string())
        .bind(usage.timestamp)
        .bind(&usage.endpoint)
        .bind(&usage.method)
        .bind(usage.status_code as i32)
        .bind(usage.response_time_ms.map(|t| t as i32))
        .bind(usage.request_size_bytes.map(|s| s as i32))
        .bind(usage.response_size_bytes.map(|s| s as i32))
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get current JWT usage count for a user (for rate limiting)
    pub async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
        let window_start = Utc::now() - Duration::hours(1); // 1 hour window

        let count: i32 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM jwt_usage
            WHERE user_id = $1 AND timestamp > $2
            "#,
        )
        .bind(user_id.to_string())
        .bind(window_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u32)
    }

    /// Create a goal for a user
    pub async fn create_goal(&self, user_id: Uuid, goal_data: serde_json::Value) -> Result<String> {
        let goal_id = Uuid::new_v4().to_string();
        let goal_json = serde_json::to_string(&goal_data)?;

        sqlx::query(
            r#"
            INSERT INTO goals (id, user_id, goal_data)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(&goal_id)
        .bind(user_id.to_string())
        .bind(goal_json)
        .execute(&self.pool)
        .await?;

        Ok(goal_id)
    }

    /// Get all goals for a user
    pub async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r#"
            SELECT id, goal_data FROM goals
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
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
                map.insert("id".to_string(), serde_json::Value::String(goal_id));
            }

            goals.push(goal);
        }

        Ok(goals)
    }

    /// Update goal progress
    pub async fn update_goal_progress(&self, goal_id: &str, _current_value: f64) -> Result<()> {
        // This is a simplified version - in reality you'd update the JSON data
        sqlx::query(
            r#"
            UPDATE goals
            SET updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            "#,
        )
        .bind(goal_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store an insight for a user
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
            r#"
            INSERT INTO insights (id, user_id, activity_id, insight_type, insight_data)
            VALUES ($1, $2, $3, $4, $5)
            "#,
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
    pub async fn get_user_insights(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r#"
            SELECT insight_data FROM insights
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
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
    pub async fn get_request_logs(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RequestLog>> {
        let mut query = String::from(
            r#"
            SELECT id, user_id, api_key_id, timestamp, method, endpoint, 
                   status_code, response_time_ms, error_message
            FROM request_logs
            WHERE 1=1
            "#,
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
                status_code: row.get::<i32, _>("status_code") as u16,
                response_time_ms: row
                    .get::<Option<i32>, _>("response_time_ms")
                    .map(|t| t as u32),
                error_message: row.get("error_message"),
            });
        }

        Ok(logs)
    }

    /// Get system-wide statistics
    pub async fn get_system_stats(&self) -> Result<(u64, u64)> {
        // Get total users
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        // Get total API keys
        let api_key_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&self.pool)
            .await?;

        Ok((user_count as u64, api_key_count as u64))
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
            display_name: Some("Analytics User".to_string()),
            password_hash: "hashed".to_string(),
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
            endpoint: "/api/v1/profile".to_string(),
            method: "GET".to_string(),
            status_code: 200,
            response_time_ms: Some(25),
            request_size_bytes: Some(128),
            response_size_bytes: Some(512),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("TestClient/1.0".to_string()),
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

        let _insight_id = db
            .store_insight(
                user.id,
                Some("activity_123".to_string()),
                "performance",
                insight_data,
            )
            .await
            .expect("Failed to store insight");

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
                email: format!("stats_user_{}@example.com", i),
                display_name: None,
                password_hash: "hashed".to_string(),
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
