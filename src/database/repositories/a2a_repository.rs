// ABOUTME: A2A (Agent-to-Agent) repository implementation
// ABOUTME: Handles A2A clients, sessions, tasks, and usage tracking
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::A2ARepository;
use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::database::{A2AUsage, A2AUsageStats, DatabaseError};
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `A2ARepository`
pub struct A2ARepositoryImpl {
    db: Database,
}

impl A2ARepositoryImpl {
    /// Create a new `A2ARepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl A2ARepository for A2ARepositoryImpl {
    async fn create_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String, DatabaseError> {
        self.db
            .create_a2a_client(client, client_secret, api_key_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client(&self, id: &str) -> Result<Option<A2AClient>, DatabaseError> {
        self.db
            .get_a2a_client(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client_by_api_key(
        &self,
        api_key_id: &str,
    ) -> Result<Option<A2AClient>, DatabaseError> {
        self.db
            .get_a2a_client_by_api_key_id(api_key_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client_by_name(&self, name: &str) -> Result<Option<A2AClient>, DatabaseError> {
        self.db
            .get_a2a_client_by_name(name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>, DatabaseError> {
        self.db
            .list_a2a_clients(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn deactivate_client(&self, id: &str) -> Result<(), DatabaseError> {
        self.db
            .deactivate_a2a_client(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client_credentials(
        &self,
        id: &str,
    ) -> Result<Option<(String, String)>, DatabaseError> {
        self.db
            .get_a2a_client_credentials(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn invalidate_client_sessions(&self, client_id: &str) -> Result<(), DatabaseError> {
        self.db
            .invalidate_a2a_client_sessions(client_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<(), DatabaseError> {
        self.db
            .deactivate_client_api_keys(client_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn create_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String, DatabaseError> {
        self.db
            .create_a2a_session(client_id, user_id, granted_scopes, expires_in_hours)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_session(&self, token: &str) -> Result<Option<A2ASession>, DatabaseError> {
        self.db
            .get_a2a_session(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_session_activity(&self, token: &str) -> Result<(), DatabaseError> {
        self.db
            .update_a2a_session_activity(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_active_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>, DatabaseError> {
        self.db
            .get_active_a2a_sessions(client_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn create_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String, DatabaseError> {
        self.db
            .create_a2a_task(client_id, session_id, task_type, input_data)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_task(&self, id: &str) -> Result<Option<A2ATask>, DatabaseError> {
        self.db
            .get_a2a_task(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>, DatabaseError> {
        self.db
            .list_a2a_tasks(client_id, status_filter, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_task_status(
        &self,
        id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_a2a_task_status(id, status, result, error)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn record_usage(&self, usage: &A2AUsage) -> Result<(), DatabaseError> {
        self.db
            .record_a2a_usage(usage)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client_current_usage(&self, client_id: &str) -> Result<u32, DatabaseError> {
        self.db
            .get_a2a_client_current_usage(client_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_usage_stats(
        &self,
        client_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<A2AUsageStats, DatabaseError> {
        self.db
            .get_a2a_usage_stats(client_id, start, end)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>, DatabaseError> {
        self.db
            .get_a2a_client_usage_history(client_id, days)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
