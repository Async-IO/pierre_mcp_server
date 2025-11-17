// ABOUTME: A2A (Agent-to-Agent) database operations
// ABOUTME: Manages agent client registration and authentication for enterprise APIs
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::Database;
use crate::a2a::{
    auth::A2AClient,
    client::A2ASession,
    protocol::{A2ATask, TaskStatus},
};
use crate::database_plugins::shared;
use crate::errors::AppError;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

/// Records of A2A protocol usage for analytics and billing
#[derive(Debug, Serialize, Deserialize)]
pub struct A2AUsage {
    /// Database record ID (None for new records)
    pub id: Option<i64>,
    /// A2A client identifier
    pub client_id: String,
    /// Optional session token for this request
    pub session_token: Option<String>,
    /// When the request was made
    pub timestamp: DateTime<Utc>,
    /// Name of the tool/endpoint called
    pub tool_name: String,
    /// Response time in milliseconds
    pub response_time_ms: Option<u32>,
    /// HTTP status code returned
    pub status_code: u16,
    /// Error message if request failed
    pub error_message: Option<String>,
    /// Request payload size in bytes
    pub request_size_bytes: Option<u32>,
    /// Response payload size in bytes
    pub response_size_bytes: Option<u32>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent string
    pub user_agent: Option<String>,
    /// A2A protocol version used
    pub protocol_version: String,
    /// List of capabilities advertised by client
    pub client_capabilities: Vec<String>,
    /// OAuth scopes granted for this request
    pub granted_scopes: Vec<String>,
}

/// Aggregated statistics for A2A usage over a time period
#[derive(Debug, Serialize, Deserialize)]
pub struct A2AUsageStats {
    /// A2A client identifier
    pub client_id: String,
    /// Start of the statistics period
    pub period_start: DateTime<Utc>,
    /// End of the statistics period
    pub period_end: DateTime<Utc>,
    /// Total number of requests in period
    pub total_requests: u32,
    /// Number of successful requests (2xx status)
    pub successful_requests: u32,
    /// Number of failed requests (4xx/5xx status)
    pub failed_requests: u32,
    /// Average response time across all requests (ms)
    pub avg_response_time_ms: Option<u32>,
    /// Total bytes sent in requests
    pub total_request_bytes: Option<u64>,
    /// Total bytes sent in responses
    pub total_response_bytes: Option<u64>,
}

/// Helper functions for safe type conversions
fn safe_u32_to_i32(value: u32) -> Result<i32> {
    Ok(i32::try_from(value).map_err(|e| {
        tracing::warn!(
            value = value,
            max_i32 = i32::MAX,
            error = %e,
            "Type conversion failed: u32 to i32"
        );
        AppError::invalid_input(format!("Value {value} too large to convert to i32: {e}"))
    })?)
}

/// Safely convert i32 to u32, returning an error if negative
fn safe_i32_to_u32(value: i32) -> Result<u32> {
    Ok(u32::try_from(value).map_err(|e| {
        tracing::warn!(
            value = value,
            error = %e,
            "Type conversion failed: negative i32 cannot convert to u32"
        );
        AppError::invalid_input(format!("Cannot convert negative value {value} to u32: {e}"))
    })?)
}

/// Safely convert i32 to u64, returning an error if negative
fn safe_i32_to_u64(value: i32) -> Result<u64> {
    Ok(u64::try_from(value).map_err(|_| {
        AppError::invalid_input(format!("Cannot convert negative value {value} to u64"))
    })?)
}

/// Safely convert i64 to u64, returning an error if negative
fn safe_i64_to_u64(value: i64) -> Result<u64> {
    Ok(u64::try_from(value).map_err(|_| {
        AppError::invalid_input(format!("Cannot convert negative value {value} to u64"))
    })?)
}

/// Safely convert f64 to u32, clamping to u32 range
fn safe_f64_to_u32(value: f64) -> u32 {
    if value.is_nan() || value < 0.0 {
        0
    } else if value > f64::from(u32::MAX) {
        u32::MAX
    } else {
        // Safe: value range checked above to be within u32 bounds
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            value as u32
        }
    }
}

impl Database {
    /// Create A2A tables and indexes
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub(super) async fn migrate_a2a(&self) -> Result<()> {
        self.create_a2a_clients_table().await?;
        self.create_a2a_sessions_table().await?;
        self.create_a2a_tasks_table().await?;
        self.create_a2a_usage_table().await?;
        self.create_a2a_client_api_keys_table().await?;
        self.create_a2a_indexes().await?;
        Ok(())
    }

    /// Create `a2a_clients` table
    async fn create_a2a_clients_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_clients (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                public_key TEXT NOT NULL,
                client_secret TEXT NOT NULL,
                permissions TEXT NOT NULL,
                capabilities TEXT NOT NULL DEFAULT '[]',
                redirect_uris TEXT NOT NULL DEFAULT '[]',
                rate_limit_requests INTEGER NOT NULL DEFAULT 1000,
                rate_limit_window_seconds INTEGER NOT NULL DEFAULT 3600,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(public_key)
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create `a2a_sessions` table
    async fn create_a2a_sessions_table(&self) -> Result<()> {
        sqlx::query(
            r"
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
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create `a2a_tasks` table
    async fn create_a2a_tasks_table(&self) -> Result<()> {
        sqlx::query(
            r"
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
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create `a2a_usage` table
    async fn create_a2a_usage_table(&self) -> Result<()> {
        sqlx::query(
            r"
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
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create `a2a_client_api_keys` junction table
    async fn create_a2a_client_api_keys_table(&self) -> Result<()> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS a2a_client_api_keys (
                client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
                api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (client_id, api_key_id)
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create indexes for A2A tables
    async fn create_a2a_indexes(&self) -> Result<()> {
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

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_a2a_client_api_keys_api_key_id ON a2a_client_api_keys(api_key_id)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a new A2A client
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON serialization fails
    pub async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        sqlx::query(
            r"
            INSERT INTO a2a_clients (
                id, user_id, name, description, public_key, client_secret, permissions,
                capabilities, redirect_uris,
                rate_limit_requests, rate_limit_window_seconds, is_active,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ",
        )
        .bind(&client.id)
        .bind(client.user_id.to_string())
        .bind(&client.name)
        .bind(&client.description)
        .bind(&client.public_key)
        .bind(client_secret)
        .bind(serde_json::to_string(&client.permissions)?)
        .bind(serde_json::to_string(&client.capabilities)?)
        .bind(serde_json::to_string(&client.redirect_uris)?)
        .bind(safe_u32_to_i32(client.rate_limit_requests)?)
        .bind(safe_u32_to_i32(client.rate_limit_window_seconds)?)
        .bind(client.is_active)
        .bind(client.created_at)
        .bind(client.updated_at)
        .execute(&self.pool)
        .await?;

        // Associate A2A client with API key
        sqlx::query(
            r"
            INSERT INTO a2a_client_api_keys (client_id, api_key_id, created_at)
            VALUES ($1, $2, $3)
            ",
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

        Ok(client.id.clone()) // Safe: String ownership needed for return value
    }

    /// Get an A2A client by ID
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON deserialization fails
    pub async fn get_a2a_client_impl(&self, client_id: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, description, public_key, permissions, capabilities, redirect_uris,
                   rate_limit_requests, rate_limit_window_seconds, is_active,
                   created_at, updated_at
            FROM a2a_clients
            WHERE id = $1
            ",
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            let capabilities_json: String = row.get("capabilities");
            let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_id = ?row.get::<String, _>("id"),
                    error = %e,
                    operation = "get_a2a_client",
                    "A2A client capabilities JSON parsing failed, using empty array"
                );
                vec![]
            });

            let redirect_uris_json: String = row.get("redirect_uris");
            let redirect_uris = serde_json::from_str(&redirect_uris_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_id = ?row.get::<String, _>("id"),
                    error = %e,
                    operation = "get_a2a_client",
                    "A2A client redirect_uris JSON parsing failed, using empty array"
                );
                vec![]
            });

            Ok(Some(A2AClient {
                id: row.get("id"),
                user_id: uuid::Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities,
                redirect_uris,
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: safe_i32_to_u32(row.get::<i32, _>("rate_limit_requests"))?,
                rate_limit_window_seconds: safe_i32_to_u32(
                    row.get::<i32, _>("rate_limit_window_seconds"),
                )?,
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get A2A client by API key ID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_a2a_client_by_api_key_id_impl(
        &self,
        api_key_id: &str,
    ) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r"
            SELECT c.id, c.user_id, c.name, c.description, c.public_key, c.permissions, c.capabilities,
                   c.redirect_uris, c.rate_limit_requests, c.rate_limit_window_seconds, c.is_active,
                   c.created_at, c.updated_at
            FROM a2a_clients c
            INNER JOIN a2a_client_api_keys k ON c.id = k.client_id
            WHERE k.api_key_id = $1 AND c.is_active = 1
            ",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            let capabilities_json: String = row.get("capabilities");
            let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_id = ?row.get::<String, _>("id"),
                    api_key_id = api_key_id,
                    error = %e,
                    operation = "get_a2a_client_by_api_key_id",
                    "A2A client capabilities JSON parsing failed, using empty array"
                );
                vec![]
            });

            let redirect_uris_json: String = row.get("redirect_uris");
            let redirect_uris = serde_json::from_str(&redirect_uris_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_id = ?row.get::<String, _>("id"),
                    api_key_id = api_key_id,
                    error = %e,
                    operation = "get_a2a_client_by_api_key_id",
                    "A2A client redirect_uris JSON parsing failed, using empty array"
                );
                vec![]
            });

            Ok(Some(A2AClient {
                id: row.get("id"),
                user_id: uuid::Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities,
                redirect_uris,
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: safe_i32_to_u32(row.get::<i32, _>("rate_limit_requests"))?,
                rate_limit_window_seconds: safe_i32_to_u32(
                    row.get::<i32, _>("rate_limit_window_seconds"),
                )?,
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// List all A2A clients for a user (or all clients if `user_id` is nil)
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON deserialization fails
    pub async fn list_a2a_clients_impl(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        let rows = if user_id == &Uuid::nil() {
            // Admin/system-wide query - list all active A2A clients
            let query = r"
                SELECT c.id, c.user_id, c.name, c.description, c.public_key, c.permissions, c.capabilities, c.redirect_uris,
                       c.rate_limit_requests, c.rate_limit_window_seconds, c.is_active,
                       c.created_at, c.updated_at
                FROM a2a_clients c 
                WHERE c.is_active = 1
                ORDER BY c.created_at DESC
            ";

            sqlx::query(query).fetch_all(&self.pool).await?
        } else {
            // User-specific query - filter by user_id through their associated API keys
            let query = r"
                SELECT DISTINCT c.id, c.user_id, c.name, c.description, c.public_key, c.permissions, c.capabilities, c.redirect_uris,
                       c.rate_limit_requests, c.rate_limit_window_seconds, c.is_active,
                       c.created_at, c.updated_at
                FROM a2a_clients c 
                INNER JOIN a2a_client_api_keys cak ON c.id = cak.client_id
                INNER JOIN api_keys k ON cak.api_key_id = k.id 
                WHERE c.is_active = 1 AND k.user_id = ? AND k.is_active = 1
                ORDER BY c.created_at DESC
            ";

            sqlx::query(query)
                .bind(user_id.to_string())
                .fetch_all(&self.pool)
                .await?
        };

        let mut clients = Vec::new();
        for row in rows {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            let capabilities_json: String = row.get("capabilities");
            let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_id = ?row.get::<String, _>("id"),
                    error = %e,
                    operation = "list_a2a_clients",
                    "A2A client capabilities JSON parsing failed, using empty array"
                );
                vec![]
            });

            let redirect_uris_json: String = row.get("redirect_uris");
            let redirect_uris = serde_json::from_str(&redirect_uris_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_id = ?row.get::<String, _>("id"),
                    error = %e,
                    operation = "list_a2a_clients",
                    "A2A client redirect_uris JSON parsing failed, using empty array"
                );
                vec![]
            });

            clients.push(A2AClient {
                id: row.get("id"),
                user_id: uuid::Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities,
                redirect_uris,
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: safe_i32_to_u32(row.get::<i32, _>("rate_limit_requests"))?,
                rate_limit_window_seconds: safe_i32_to_u32(
                    row.get::<i32, _>("rate_limit_window_seconds"),
                )?,
                updated_at: row.get("updated_at"),
            });
        }

        Ok(clients)
    }

    /// Deactivate an A2A client
    ///
    /// # Errors
    /// Returns an error if database operations fail or client not found
    pub async fn deactivate_a2a_client_impl(&self, client_id: &str) -> Result<()> {
        let query = "UPDATE a2a_clients SET is_active = 0, updated_at = ? WHERE id = ?";
        let now = chrono::Utc::now();

        let result = sqlx::query(query)
            .bind(now)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("A2A client: {client_id}")).into());
        }

        Ok(())
    }

    /// Get client credentials for authentication
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        let query = "SELECT id, client_secret FROM a2a_clients WHERE id = ? AND is_active = 1";

        let row = sqlx::query(query)
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map_or_else(
            || None,
            |row| {
                let id: String = row.get("id");
                let secret: String = row.get("client_secret");
                Some((id, secret))
            },
        ))
    }

    /// Invalidate all active sessions for a client
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub async fn invalidate_a2a_client_sessions_impl(&self, client_id: &str) -> Result<()> {
        let query =
            "UPDATE a2a_sessions SET expires_at = datetime('now', '-1 hour') WHERE client_id = ?";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Deactivate all API keys associated with a client
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub async fn deactivate_client_api_keys_impl(&self, client_id: &str) -> Result<()> {
        // Get API keys associated with the client through the a2a_clients table
        let query = "UPDATE api_keys SET is_active = 0 WHERE id IN (SELECT api_key_id FROM a2a_client_api_keys WHERE client_id = ?)";

        sqlx::query(query)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get A2A client by name
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON deserialization fails
    pub async fn get_a2a_client_by_name_impl(&self, name: &str) -> Result<Option<A2AClient>> {
        let row = sqlx::query(
            r"
            SELECT id, name, description, public_key, permissions, capabilities, redirect_uris,
                   rate_limit_requests, rate_limit_window_seconds, is_active,
                   created_at, updated_at
            FROM a2a_clients
            WHERE name = $1
            ",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.get("permissions");
            let permissions = serde_json::from_str(&permissions_json)?;

            let capabilities_json: String = row.get("capabilities");
            let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_name = name,
                    error = %e,
                    operation = "get_a2a_client_by_name",
                    "A2A client capabilities JSON parsing failed, using empty array"
                );
                vec![]
            });

            let redirect_uris_json: String = row.get("redirect_uris");
            let redirect_uris = serde_json::from_str(&redirect_uris_json).unwrap_or_else(|e| {
                tracing::warn!(
                    client_name = name,
                    error = %e,
                    operation = "get_a2a_client_by_name",
                    "A2A client redirect_uris JSON parsing failed, using empty array"
                );
                vec![]
            });

            Ok(Some(A2AClient {
                id: row.get("id"),
                user_id: uuid::Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                name: row.get("name"),
                description: row.get("description"),
                public_key: row.get("public_key"),
                capabilities,
                redirect_uris,
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                permissions,
                rate_limit_requests: safe_i32_to_u32(row.get::<i32, _>("rate_limit_requests"))?,
                rate_limit_window_seconds: safe_i32_to_u32(
                    row.get::<i32, _>("rate_limit_window_seconds"),
                )?,
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a new A2A session
    ///
    /// # Errors
    /// Returns an error if database operations fail
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
            r"
            INSERT INTO a2a_sessions (
                session_token, client_id, user_id, granted_scopes,
                expires_at, last_activity, created_at, requests_count
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ",
        )
        .bind(&session_token)
        .bind(client_id)
        .bind(user_id.map(ToString::to_string))
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
    ///
    /// # Errors
    /// Returns an error if database operations fail or UUID parsing fails
    pub async fn get_a2a_session_impl(&self, session_token: &str) -> Result<Option<A2ASession>> {
        let row = sqlx::query(
            r"
            SELECT session_token, client_id, user_id, granted_scopes,
                   expires_at, last_activity, created_at, requests_count
            FROM a2a_sessions
            WHERE session_token = $1 AND expires_at > CURRENT_TIMESTAMP
            ",
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
                .map(ToString::to_string)
                .collect();

            Ok(Some(A2ASession {
                id: row.get("session_token"),
                client_id: row.get("client_id"),
                user_id,
                granted_scopes,
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_activity: row.get("last_activity"),
                requests_count: safe_i32_to_u64(row.get::<i32, _>("requests_count"))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update A2A session activity timestamp
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub async fn update_a2a_session_activity_impl(&self, session_token: &str) -> Result<()> {
        sqlx::query(
            r"
            UPDATE a2a_sessions 
            SET last_activity = datetime('now'), requests_count = requests_count + 1
            WHERE session_token = $1
            ",
        )
        .bind(session_token)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get active sessions for a specific client
    ///
    /// # Errors
    /// Returns an error if database operations fail or UUID parsing fails
    pub async fn get_active_a2a_sessions_impl(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        let rows = sqlx::query(
            r"
            SELECT session_token, client_id, user_id, granted_scopes,
                   expires_at, last_activity, created_at, requests_count
            FROM a2a_sessions
            WHERE client_id = $1 AND expires_at > CURRENT_TIMESTAMP
            ORDER BY last_activity DESC
            ",
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
                .map(ToString::to_string)
                .collect();

            sessions.push(A2ASession {
                id: row.get("session_token"),
                client_id: row.get("client_id"),
                user_id,
                granted_scopes,
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_activity: row.get("last_activity"),
                requests_count: safe_i32_to_u64(row.get::<i32, _>("requests_count"))?,
            });
        }

        Ok(sessions)
    }

    /// Create a new A2A task
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON serialization fails
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
            r"
            INSERT INTO a2a_tasks (
                id, client_id, task_type, input_data, output_data,
                status, error_message, created_at, updated_at, completed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ",
        )
        .bind(&task_id)
        .bind(client_id)
        .bind(task_type)
        .bind(serde_json::to_string(input_data)?)
        .bind(None::<String>) // output_data
        .bind(shared::enums::task_status_to_str(&TaskStatus::Pending))
        .bind(None::<String>) // error_message
        .bind(now)
        .bind(now)
        .bind(None::<DateTime<Utc>>) // completed_at
        .execute(&self.pool)
        .await?;

        Ok(task_id)
    }

    /// List A2A tasks with optional filtering
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON deserialization fails
    pub async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        use std::fmt::Write;
        let mut query = String::from(
            r"
            SELECT id, client_id, task_type, input_data, output_data,
                   status, error_message, created_at, updated_at, completed_at
            FROM a2a_tasks
            ",
        );

        let mut conditions = Vec::new();
        let mut bind_count = 0;

        if client_id.is_some() {
            bind_count += 1;
            conditions.push(format!("client_id = ${bind_count}"));
        }

        if status_filter.is_some() {
            bind_count += 1;
            conditions.push(format!("status = ${bind_count}"));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY created_at DESC");

        if limit.is_some() {
            bind_count += 1;
            if write!(query, " LIMIT ${bind_count}").is_err() {
                return Err(AppError::internal("Failed to write LIMIT clause to query").into());
            }
        }

        if offset.is_some() {
            bind_count += 1;
            if write!(query, " OFFSET ${bind_count}").is_err() {
                return Err(AppError::internal("Failed to write OFFSET clause to query").into());
            }
        }

        let mut sql_query = sqlx::query(&query);

        if let Some(client_id_val) = client_id {
            sql_query = sql_query.bind(client_id_val);
        }

        if let Some(status_val) = status_filter {
            sql_query = sql_query.bind(status_val.to_string());
        }

        if let Some(limit_val) = limit {
            sql_query = sql_query.bind(safe_u32_to_i32(limit_val)?);
        }

        if let Some(offset_val) = offset {
            sql_query = sql_query.bind(safe_u32_to_i32(offset_val)?);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let tasks: Vec<A2ATask> = rows
            .iter()
            .map(shared::mappers::parse_a2a_task_from_row)
            .collect::<Result<Vec<_>>>()?;

        Ok(tasks)
    }

    /// Get an A2A task by ID
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON deserialization fails
    pub async fn get_a2a_task_impl(&self, task_id: &str) -> Result<Option<A2ATask>> {
        let row = sqlx::query(
            r"
            SELECT id, client_id, task_type, input_data, output_data,
                   status, error_message, created_at, updated_at, completed_at
            FROM a2a_tasks
            WHERE id = $1
            ",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let task = shared::mappers::parse_a2a_task_from_row(&row)?;
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Update A2A task status
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON serialization fails
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
            r"
            UPDATE a2a_tasks 
            SET status = $2, output_data = $3, error_message = $4,
                updated_at = datetime('now'), completed_at = $5
            WHERE id = $1
            ",
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
    ///
    /// # Errors
    /// Returns an error if database operations fail or JSON serialization fails
    pub async fn record_a2a_usage_impl(&self, usage: &A2AUsage) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO a2a_usage (
                client_id, session_token, timestamp, tool_name, response_time_ms,
                status_code, error_message, request_size_bytes, response_size_bytes,
                ip_address, user_agent, protocol_version, client_capabilities, granted_scopes
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ",
        )
        .bind(&usage.client_id)
        .bind(&usage.session_token)
        .bind(usage.timestamp)
        .bind(&usage.tool_name)
        .bind(usage.response_time_ms.map(safe_u32_to_i32).transpose()?)
        .bind(i32::from(usage.status_code))
        .bind(&usage.error_message)
        .bind(usage.request_size_bytes.map(safe_u32_to_i32).transpose()?)
        .bind(usage.response_size_bytes.map(safe_u32_to_i32).transpose()?)
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
    ///
    /// # Errors
    /// Returns an error if database operations fail or client not found
    pub async fn get_a2a_client_current_usage_impl(&self, client_id: &str) -> Result<u32> {
        // Get the client to determine its rate limit window
        let client = self
            .get_a2a_client(client_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("A2A client: {client_id}")))?;

        let window_start =
            Utc::now() - chrono::Duration::seconds(i64::from(client.rate_limit_window_seconds));

        let count: i32 = sqlx::query_scalar(
            r"
            SELECT COUNT(*) FROM a2a_usage
            WHERE client_id = $1 AND timestamp > $2
            ",
        )
        .bind(client_id)
        .bind(window_start)
        .fetch_one(&self.pool)
        .await?;

        safe_i32_to_u32(count)
    }

    /// Get A2A usage statistics for a client
    ///
    /// # Errors
    /// Returns an error if database operations fail
    pub async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<A2AUsageStats> {
        let stats = sqlx::query(
            r"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as failed_requests,
                AVG(response_time_ms) as avg_response_time,
                SUM(request_size_bytes) as total_request_bytes,
                SUM(response_size_bytes) as total_response_bytes
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp >= $2 AND timestamp <= $3
            ",
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
            client_id: client_id.to_owned(),
            period_start: start_date,
            period_end: end_date,
            total_requests: safe_i32_to_u32(total_requests)?,
            successful_requests: safe_i32_to_u32(successful_requests)?,
            failed_requests: safe_i32_to_u32(failed_requests)?,
            avg_response_time_ms: avg_response_time.map(safe_f64_to_u32),
            total_request_bytes: total_request_bytes.map(safe_i64_to_u64).transpose()?,
            total_response_bytes: total_response_bytes.map(safe_i64_to_u64).transpose()?,
        })
    }

    /// Get A2A client usage history (daily aggregates with success/error counts)
    ///
    /// # Errors
    /// Returns an error if database operations fail or date parsing fails
    ///
    /// # Panics
    /// Panics if the date string from database is not in expected YYYY-MM-DD format
    pub async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        let start_date = Utc::now() - chrono::Duration::days(i64::from(days));

        let rows = sqlx::query(
            r"
            SELECT 
                date(timestamp) as usage_date,
                COUNT(CASE WHEN status_code >= 200 AND status_code < 400 THEN 1 END) as success_count,
                COUNT(CASE WHEN status_code >= 400 THEN 1 END) as error_count
            FROM a2a_usage
            WHERE client_id = $1 AND timestamp >= $2
            GROUP BY date(timestamp)
            ORDER BY usage_date DESC
            ",
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
                .ok_or_else(|| {
                    AppError::invalid_input(format!(
                        "Failed to create datetime from date {date_str}"
                    ))
                })?
                .and_utc();

            history.push((
                date,
                safe_i32_to_u32(success_count)?,
                safe_i32_to_u32(error_count)?,
            ));
        }

        Ok(history)
    }
    // Public wrapper methods (delegate to _impl versions)

    /// Deactivate A2A client (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        self.deactivate_a2a_client_impl(client_id).await
    }

    /// Deactivate client API keys (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        self.deactivate_client_api_keys_impl(client_id).await
    }

    /// Get A2A client by ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        self.get_a2a_client_impl(client_id).await
    }

    /// Get A2A client by API key ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_a2a_client_by_api_key_id(
        &self,
        api_key_id: &str,
    ) -> Result<Option<A2AClient>> {
        self.get_a2a_client_by_api_key_id_impl(api_key_id).await
    }

    /// Get A2A client by name (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        self.get_a2a_client_by_name_impl(name).await
    }

    /// Get A2A client current usage (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        self.get_a2a_client_current_usage_impl(client_id).await
    }

    /// Get A2A session by token (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        self.get_a2a_session_impl(session_token).await
    }

    /// Get A2A task by ID (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        self.get_a2a_task_impl(task_id).await
    }

    /// Get active A2A sessions (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        self.get_active_a2a_sessions_impl(client_id).await
    }

    /// Invalidate A2A client sessions (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        self.invalidate_a2a_client_sessions_impl(client_id).await
    }

    /// List A2A clients for user (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        self.list_a2a_clients_impl(user_id).await
    }

    /// Record A2A usage (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn record_a2a_usage(&self, usage: &crate::database::A2AUsage) -> Result<()> {
        self.record_a2a_usage_impl(usage).await
    }

    /// Update A2A session activity (public API)
    ///
    /// # Errors
    /// Returns error if database operation fails
    pub async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        self.update_a2a_session_activity_impl(session_token).await
    }
}
