//! A2A (Agent-to-Agent) database operations

use super::Database;
use crate::a2a::{
    auth::A2AClient,
    client::A2ASession,
    protocol::{A2ATask, TaskStatus},
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AUsage {
    pub id: Option<i64>, // Allow None for new records
    pub client_id: String,
    pub session_token: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub response_time_ms: Option<u32>,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub request_size_bytes: Option<u32>,
    pub response_size_bytes: Option<u32>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub protocol_version: String,
    pub client_capabilities: Vec<String>,
    pub granted_scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AUsageStats {
    pub client_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub avg_response_time_ms: Option<u32>,
    pub total_request_bytes: Option<u64>,
    pub total_response_bytes: Option<u64>,
}

impl Database {
    /// Create A2A tables
    pub(super) async fn migrate_a2a(&self) -> Result<()> {
        // Create a2a_clients table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS a2a_clients (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                public_key TEXT NOT NULL,
                client_secret TEXT NOT NULL,
                permissions TEXT NOT NULL,
                rate_limit_requests INTEGER NOT NULL DEFAULT 1000,
                rate_limit_window_seconds INTEGER NOT NULL DEFAULT 3600,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(public_key)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create a2a_sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS a2a_sessions (
                session_token TEXT PRIMARY KEY,
                client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
                user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
                granted_scopes TEXT NOT NULL,
                expires_at DATETIME NOT NULL,
                last_activity DATETIME DEFAULT CURRENT_TIMESTAMP,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                requests_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create a2a_tasks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS a2a_tasks (
                id TEXT PRIMARY KEY,
                client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
                task_type TEXT NOT NULL,
                input_data TEXT NOT NULL,
                output_data TEXT,
                status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
                error_message TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create a2a_usage table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS a2a_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
                session_token TEXT REFERENCES a2a_sessions(session_token) ON DELETE SET NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                tool_name TEXT NOT NULL,
                response_time_ms INTEGER,
                status_code INTEGER NOT NULL,
                error_message TEXT,
                request_size_bytes INTEGER,
                response_size_bytes INTEGER,
                ip_address TEXT,
                user_agent TEXT,
                protocol_version TEXT NOT NULL DEFAULT '1.0',
                client_capabilities TEXT NOT NULL DEFAULT '[]',
                granted_scopes TEXT NOT NULL DEFAULT '[]'
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create a2a_client_api_keys junction table for API key associations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS a2a_client_api_keys (
                client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (client_id, api_key_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_a2a_sessions_client_id ON a2a_sessions(client_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_a2a_sessions_expires_at ON a2a_sessions(expires_at)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_tasks_client_id ON a2a_tasks(client_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_tasks_status ON a2a_tasks(status)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_client_id ON a2a_usage(client_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_a2a_usage_timestamp ON a2a_usage(timestamp)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a new A2A client
    pub async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        sqlx::query(
            r#"
            INSERT INTO a2a_clients (
                id, name, description, public_key, client_secret, permissions,
                rate_limit_requests, rate_limit_window_seconds, is_active,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(&client.id)
        .bind(&client.name)
        .bind(&client.description)
        .bind(&client.public_key)
        .bind(client_secret)
        .bind(serde_json::to_string(&client.permissions)?)
        .bind(client.rate_limit_requests as i32)
        .bind(client.rate_limit_window_seconds as i32)
        .bind(client.is_active)
        .bind(client.created_at)
        .bind(client.updated_at)
        .execute(&self.pool)
        .await?;

        // Associate A2A client with API key
        sqlx::query(
            r#"
            INSERT INTO a2a_client_api_keys (client_id, api_key_id, created_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(&client.id)
        .bind(api_key_id)
        .bind(chrono::Utc::now())
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            "Created A2A client {} with API key {} association",
            client.id,
            api_key_id
        );

        Ok(client.id.clone())
    }

    /// Get an A2A client by ID
    pub async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, public_key, permissions,
                   rate_limit_requests, rate_limit_window_seconds, is_active,
                   created_at, updated_at
            FROM a2a_clients
            WHERE id = $1
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            Ok(Some(A2AClient {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities: vec![],  // Map from permissions or set default
                redirect_uris: vec![], // Set default
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: row.get::<i32, _>("rate_limit_requests") as u32,
                rate_limit_window_seconds: row.get::<i32, _>("rate_limit_window_seconds") as u32,
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// List all A2A clients for a user (or all clients if user_id is nil)
    pub async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        let rows = if user_id == &Uuid::nil() {
            // Admin/system-wide query - list all active A2A clients
            let query = r#"
                SELECT c.id, c.name, c.description, c.public_key, c.permissions,
                       c.rate_limit_requests, c.rate_limit_window_seconds, c.is_active,
                       c.created_at, c.updated_at
                FROM a2a_clients c 
                WHERE c.is_active = 1
                ORDER BY c.created_at DESC
            "#;

            sqlx::query(query).fetch_all(&self.pool).await?
        } else {
            // User-specific query - filter by user_id through their associated API keys
            let query = r#"
                SELECT DISTINCT c.id, c.name, c.description, c.public_key, c.permissions,
                       c.rate_limit_requests, c.rate_limit_window_seconds, c.is_active,
                       c.created_at, c.updated_at
                FROM a2a_clients c 
                INNER JOIN a2a_client_api_keys cak ON c.id = cak.client_id
                INNER JOIN api_keys k ON cak.api_key_id = k.id 
                WHERE c.is_active = 1 AND k.user_id = ? AND k.is_active = 1
                ORDER BY c.created_at DESC
            "#;

            sqlx::query(query)
                .bind(user_id.to_string())
                .fetch_all(&self.pool)
                .await?
        };

        let mut clients = Vec::new();
        for row in rows {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            clients.push(A2AClient {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities: vec![],  // Map from permissions or set default
                redirect_uris: vec![], // Set default
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: row.get::<i32, _>("rate_limit_requests") as u32,
                rate_limit_window_seconds: row.get::<i32, _>("rate_limit_window_seconds") as u32,
                updated_at: row.get("updated_at"),
            });
        }

        Ok(clients)
    }

    /// Deactivate an A2A client
    pub async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        let query = "UPDATE a2a_clients SET is_active = 0, updated_at = ? WHERE id = ?";
        let now = chrono::Utc::now();

        let result = sqlx::query(query)
            .bind(now)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("A2A client not found: {}", client_id));
        }

        Ok(())
    }

    /// Get client credentials for authentication
    pub async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        let query = "SELECT id, client_secret FROM a2a_clients WHERE id = ? AND is_active = 1";

        let row = sqlx::query(query)
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let id: String = row.get("id");
            let secret: String = row.get("client_secret");
            Ok(Some((id, secret)))
        } else {
            Ok(None)
        }
    }

    /// Invalidate all active sessions for a client
    pub async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        let query =
            "UPDATE a2a_sessions SET expires_at = datetime('now', '-1 hour') WHERE client_id = ?";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Deactivate all API keys associated with a client
    pub async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        // Get API keys associated with the client through the a2a_clients table
        let query = "UPDATE api_keys SET is_active = 0 WHERE id IN (SELECT api_key_id FROM a2a_client_api_keys WHERE client_id = ?)";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get A2A client by name
    pub async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, public_key, permissions,
                   rate_limit_requests, rate_limit_window_seconds, is_active,
                   created_at, updated_at
            FROM a2a_clients
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            Ok(Some(A2AClient {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities: vec![],  // Map from permissions or set default
                redirect_uris: vec![], // Set default
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: row.get::<i32, _>("rate_limit_requests") as u32,
                rate_limit_window_seconds: row.get::<i32, _>("rate_limit_window_seconds") as u32,
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a new A2A session
    pub async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        let session_token = format!("sess_{}", Uuid::new_v4());
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(expires_in_hours);

        sqlx::query(
            r#"
            INSERT INTO a2a_sessions (
                session_token, client_id, user_id, granted_scopes,
                expires_at, last_activity, created_at, requests_count
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&session_token)
        .bind(client_id)
        .bind(user_id.map(|u| u.to_string()))
        .bind(granted_scopes.join(","))
        .bind(expires_at)
        .bind(now)
        .bind(now)
        .bind(0) // Initial requests count
        .execute(&self.pool)
        .await?;

        Ok(session_token)
    }

    /// Get an A2A session by token
    pub async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        let row = sqlx::query(
            r#"
            SELECT session_token, client_id, user_id, granted_scopes, 
                   expires_at, last_activity, created_at, requests_count
            FROM a2a_sessions
            WHERE session_token = $1 AND expires_at > datetime('now')
            "#,
        )
        .bind(session_token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let user_id_str: Option<String> = row.get("user_id");
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()?;

            let granted_scopes_str: String = row.get("granted_scopes");
            let granted_scopes = granted_scopes_str
                .split(',')
                .map(|s| s.to_string())
                .collect();

            Ok(Some(A2ASession {
                id: row.get("session_token"),
                client_id: row.get("client_id"),
                user_id,
                granted_scopes,
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_activity: row.get("last_activity"),
                requests_count: row.get::<i32, _>("requests_count") as u64,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update A2A session activity timestamp
    pub async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE a2a_sessions 
            SET last_activity = datetime('now'), requests_count = requests_count + 1
            WHERE session_token = $1
            "#,
        )
        .bind(session_token)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get active sessions for a specific client
    pub async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        let rows = sqlx::query(
            r#"
            SELECT session_token, client_id, user_id, granted_scopes, 
                   expires_at, last_activity, created_at, requests_count
            FROM a2a_sessions
            WHERE client_id = $1 AND expires_at > datetime('now')
            ORDER BY last_activity DESC
            "#,
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let user_id_str: Option<String> = row.get("user_id");
            let user_id = user_id_str
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()?;

            let granted_scopes_str: String = row.get("granted_scopes");
            let granted_scopes = granted_scopes_str
                .split(',')
                .map(|s| s.to_string())
                .collect();

            sessions.push(A2ASession {
                id: row.get("session_token"),
                client_id: row.get("client_id"),
                user_id,
                granted_scopes,
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_activity: row.get("last_activity"),
                requests_count: row.get::<i32, _>("requests_count") as u64,
            });
        }

        Ok(sessions)
    }

    /// Create a new A2A task
    pub async fn create_a2a_task(
        &self,
        client_id: &str,
        _session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String> {
        let task_id = format!("task_{}", Uuid::new_v4());
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO a2a_tasks (
                id, client_id, task_type, input_data, output_data,
                status, error_message, created_at, updated_at, completed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(&task_id)
        .bind(client_id)
        .bind(task_type)
        .bind(serde_json::to_string(input_data)?)
        .bind(None::<String>) // output_data
        .bind(TaskStatus::Pending.to_string())
        .bind(None::<String>) // error_message
        .bind(now)
        .bind(now)
        .bind(None::<DateTime<Utc>>) // completed_at
        .execute(&self.pool)
        .await?;

        Ok(task_id)
    }

    /// Get an A2A task by ID
    pub async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        let row = sqlx::query(
            r#"
            SELECT id, client_id, task_type, input_data, output_data,
                   status, error_message, created_at, updated_at, completed_at
            FROM a2a_tasks
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let input_data_json: String = row.get("input_data");
            let input_data = serde_json::from_str(&input_data_json)?;

            let output_data = if let Some(output_json) = row.get::<Option<String>, _>("output_data")
            {
                Some(serde_json::from_str(&output_json)?)
            } else {
                None
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "pending" => TaskStatus::Pending,
                "running" => TaskStatus::Running,
                "completed" => TaskStatus::Completed,
                "failed" => TaskStatus::Failed,
                "cancelled" => TaskStatus::Cancelled,
                _ => return Err(anyhow!("Invalid task status: {}", status_str)),
            };

            Ok(Some(A2ATask {
                id: row.get("id"),
                status,
                created_at: row.get("created_at"),
                completed_at: row.get("completed_at"),
                result: output_data.clone(),
                error: row.get("error_message"),
                client_id: row.get("client_id"),
                task_type: row.get("task_type"),
                input_data,
                output_data,
                error_message: row.get("error_message"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update A2A task status
    pub async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        let output_json = result.map(serde_json::to_string).transpose()?;

        let completed_at = match status {
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => Some(Utc::now()),
            _ => None,
        };

        sqlx::query(
            r#"
            UPDATE a2a_tasks 
            SET status = $2, output_data = $3, error_message = $4,
                updated_at = datetime('now'), completed_at = $5
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .bind(status.to_string())
        .bind(output_json)
        .bind(error)
        .bind(completed_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record A2A usage for rate limiting and analytics
    pub async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO a2a_usage (
                client_id, session_token, timestamp, tool_name, response_time_ms,
                status_code, error_message, request_size_bytes, response_size_bytes,
                ip_address, user_agent, protocol_version, client_capabilities, granted_scopes
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(&usage.client_id)
        .bind(&usage.session_token)
        .bind(usage.timestamp)
        .bind(&usage.tool_name)
        .bind(usage.response_time_ms.map(|t| t as i32))
        .bind(usage.status_code as i32)
        .bind(&usage.error_message)
        .bind(usage.request_size_bytes.map(|s| s as i32))
        .bind(usage.response_size_bytes.map(|s| s as i32))
        .bind(&usage.ip_address)
        .bind(&usage.user_agent)
        .bind(&usage.protocol_version)
        .bind(serde_json::to_string(&usage.client_capabilities)?)
        .bind(serde_json::to_string(&usage.granted_scopes)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get current usage count for an A2A client (for rate limiting)
    pub async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        // Get the client to determine its rate limit window
        let client = self
            .get_a2a_client(client_id)
            .await?
            .ok_or_else(|| anyhow!("A2A client not found: {}", client_id))?;

        let window_start =
            Utc::now() - chrono::Duration::seconds(client.rate_limit_window_seconds as i64);

        let count: i32 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM a2a_usage
            WHERE client_id = $1 AND timestamp > $2
            "#,
        )
        .bind(client_id)
        .bind(window_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u32)
    }

    /// Get A2A usage statistics for a client
    pub async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<A2AUsageStats> {
        let stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as failed_requests,
                AVG(response_time_ms) as avg_response_time,
                SUM(request_size_bytes) as total_request_bytes,
                SUM(response_size_bytes) as total_response_bytes
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp >= $2 AND timestamp <= $3
            "#,
        )
        .bind(client_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        let total_requests: i32 = stats.get(0);
        let successful_requests: i32 = stats.get(1);
        let failed_requests: i32 = stats.get(2);
        let avg_response_time: Option<f64> = stats.get(3);
        let total_request_bytes: Option<i64> = stats.get(4);
        let total_response_bytes: Option<i64> = stats.get(5);

        Ok(A2AUsageStats {
            client_id: client_id.to_string(),
            period_start: start_date,
            period_end: end_date,
            total_requests: total_requests as u32,
            successful_requests: successful_requests as u32,
            failed_requests: failed_requests as u32,
            avg_response_time_ms: avg_response_time.map(|t| t as u32),
            total_request_bytes: total_request_bytes.map(|b| b as u64),
            total_response_bytes: total_response_bytes.map(|b| b as u64),
        })
    }

    /// Get A2A client usage history (daily aggregates with success/error counts)
    pub async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        let start_date = Utc::now() - chrono::Duration::days(days as i64);

        let rows = sqlx::query(
            r#"
            SELECT 
                date(timestamp) as usage_date,
                COUNT(CASE WHEN status_code >= 200 AND status_code < 400 THEN 1 END) as success_count,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as error_count
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp >= $2
            GROUP BY date(timestamp)
            ORDER BY usage_date DESC
            "#,
        )
        .bind(client_id)
        .bind(start_date)
        .fetch_all(&self.pool)
        .await?;

        let mut history = Vec::new();
        for row in rows {
            let date_str: String = row.get("usage_date");
            let success_count: i32 = row.get("success_count");
            let error_count: i32 = row.get("error_count");

            // Parse date string (YYYY-MM-DD format from SQLite date())
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();

            history.push((date, success_count as u32, error_count as u32));
        }

        Ok(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Note: A2APermission should be defined - using String for now

    async fn create_test_client(db: &Database) -> (A2AClient, Uuid) {
        let unique_id = Uuid::new_v4();

        // First create a test user
        let test_user_id = Uuid::new_v4();
        let user = crate::models::User {
            id: test_user_id,
            email: format!("test_{}@example.com", unique_id),
            display_name: Some(format!("Test User {}", unique_id)),
            password_hash: "dummy_hash".to_string(),
            tier: crate::models::UserTier::Professional,
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            created_at: Utc::now(),
            last_active: Utc::now(),
        };
        db.create_user(&user)
            .await
            .expect("Failed to create test user");

        // Create a test API key for the user
        let api_key = crate::api_keys::ApiKey {
            id: format!("test_api_key_{}", unique_id),
            user_id: test_user_id,
            name: format!("Test API Key {}", unique_id),
            description: Some("Test API key for A2A client".to_string()),
            key_prefix: format!("pk_test_{}", &unique_id.to_string()[0..8]),
            key_hash: "dummy_hash".to_string(),
            tier: crate::api_keys::ApiKeyTier::Professional,
            rate_limit_requests: 1000,
            rate_limit_window_seconds: 3600,
            is_active: true,
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
        };
        db.create_api_key(&api_key)
            .await
            .expect("Failed to create test API key");

        let client = A2AClient {
            id: format!("test_client_{}", unique_id),
            name: format!("Test Client {}", unique_id),
            description: format!("Test A2A client {}", unique_id),
            public_key: format!("test_public_key_{}", unique_id), // Make unique
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: vec!["https://test.example.com".to_string()],
            permissions: vec!["read_activities".to_string(), "write_goals".to_string()],
            rate_limit_requests: 1000,
            rate_limit_window_seconds: 3600,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_a2a_client(&client, "test_secret", &api_key.id)
            .await
            .expect("Failed to create A2A client");
        (client, test_user_id)
    }

    #[tokio::test]
    async fn test_a2a_client_management() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let (client, user_id) = create_test_client(&db).await;

        // Get client
        let retrieved = db
            .get_a2a_client(&client.id)
            .await
            .expect("Failed to get A2A client")
            .expect("Client not found");

        assert_eq!(retrieved.id, client.id);
        assert_eq!(retrieved.name, client.name);
        assert_eq!(retrieved.permissions, client.permissions);

        // List clients - check that our client is in the list
        let clients = db
            .list_a2a_clients(&user_id)
            .await
            .expect("Failed to list A2A clients");

        // Find our client in the list
        let found_client = clients.iter().find(|c| c.id == client.id);
        assert!(
            found_client.is_some(),
            "Created client should be in the list"
        );
        assert_eq!(found_client.unwrap().id, client.id);
    }

    #[tokio::test]
    async fn test_a2a_session_management() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let (client, _user_id) = create_test_client(&db).await;

        // Create session (without user_id to avoid foreign key constraint)
        let session = A2ASession {
            id: format!("session_{}", Uuid::new_v4()),
            client_id: client.id.clone(),
            user_id: None, // No user association for this test
            granted_scopes: vec!["read".to_string(), "write".to_string()],
            expires_at: Utc::now() + chrono::Duration::hours(1),
            last_activity: Utc::now(),
            created_at: Utc::now(),
            requests_count: 0,
        };

        let session_token = db
            .create_a2a_session(
                &session.client_id,
                session.user_id.as_ref(),
                &session.granted_scopes,
                1,
            )
            .await
            .expect("Failed to create A2A session");

        // Get session
        let retrieved = db
            .get_a2a_session(&session_token)
            .await
            .expect("Failed to get A2A session")
            .expect("Session not found");

        assert_eq!(retrieved.id, session_token);
        assert_eq!(retrieved.client_id, session.client_id);
        assert_eq!(retrieved.granted_scopes, session.granted_scopes);

        // Update session activity
        db.update_a2a_session_activity(&session_token)
            .await
            .expect("Failed to update session activity");

        // Test getting active sessions for client
        let active_sessions = db
            .get_active_a2a_sessions(&client.id)
            .await
            .expect("Failed to get active sessions");

        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].id, session_token);
        assert_eq!(active_sessions[0].client_id, client.id);
    }

    #[tokio::test]
    async fn test_a2a_task_management() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let (client, _user_id) = create_test_client(&db).await;

        // Create task
        let task = A2ATask {
            id: format!("task_{}", Uuid::new_v4()),
            client_id: client.id.clone(),
            task_type: "analysis".to_string(),
            input_data: serde_json::json!({"data": "test"}),
            output_data: None,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        let task_id = db
            .create_a2a_task(&task.client_id, None, &task.task_type, &task.input_data)
            .await
            .expect("Failed to create A2A task");

        // Get task
        let retrieved = db
            .get_a2a_task(&task_id)
            .await
            .expect("Failed to get A2A task")
            .expect("Task not found");

        assert_eq!(retrieved.id, task_id);
        assert_eq!(retrieved.status, TaskStatus::Pending);

        // Update task status
        let output_data = serde_json::json!({"result": "success"});
        db.update_a2a_task_status(&task_id, &TaskStatus::Completed, Some(&output_data), None)
            .await
            .expect("Failed to update task status");

        // Verify update
        let updated = db
            .get_a2a_task(&task_id)
            .await
            .expect("Failed to get updated task")
            .expect("Task not found");

        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.output_data, Some(output_data));
        assert!(updated.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_a2a_usage_tracking() {
        let db = crate::database::tests::create_test_db()
            .await
            .expect("Failed to create test database");

        let (client, _user_id) = create_test_client(&db).await;

        // Record usage
        let usage = A2AUsage {
            id: None,
            client_id: client.id.clone(),
            session_token: None, // No session for this test
            timestamp: Utc::now(),
            tool_name: "analyze".to_string(),
            request_size_bytes: Some(256),
            response_size_bytes: Some(512),
            response_time_ms: Some(100),
            status_code: 200,
            error_message: None,
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            protocol_version: "1.0".to_string(),
            client_capabilities: vec!["analysis".to_string()],
            granted_scopes: vec!["read".to_string()],
        };

        db.record_a2a_usage(&usage)
            .await
            .expect("Failed to record A2A usage");

        // Check current usage
        let current_usage = db
            .get_a2a_client_current_usage(&client.id)
            .await
            .expect("Failed to get current usage");
        assert_eq!(current_usage, 1);

        // Get usage stats
        let stats = db
            .get_a2a_usage_stats(
                &client.id,
                Utc::now() - chrono::Duration::hours(1),
                Utc::now() + chrono::Duration::hours(1),
            )
            .await
            .expect("Failed to get usage stats");

        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 0);
    }
}
