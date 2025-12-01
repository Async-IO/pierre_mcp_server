// ABOUTME: Usage tracking and analytics repository implementation
// ABOUTME: Handles API key usage, JWT usage, request logs, and system stats
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::UsageRepository;
use crate::api_keys::{ApiKeyUsage, ApiKeyUsageStats};
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::rate_limiting::JwtUsage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `UsageRepository`
pub struct UsageRepositoryImpl {
    db: Database,
}

impl UsageRepositoryImpl {
    /// Create a new `UsageRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UsageRepository for UsageRepositoryImpl {
    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<(), DatabaseError> {
        self.db
            .record_api_key_usage(usage)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32, DatabaseError> {
        self.db
            .get_api_key_current_usage(api_key_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats, DatabaseError> {
        self.db
            .get_api_key_usage_stats(api_key_id, start, end)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<(), DatabaseError> {
        self.db
            .record_jwt_usage(usage)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32, DatabaseError> {
        self.db
            .get_jwt_current_usage(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_request_logs(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        status_filter: Option<&str>,
        tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>, DatabaseError> {
        self.db
            .get_request_logs(api_key_id, start_time, end_time, status_filter, tool_filter)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_system_stats(&self) -> Result<(u64, u64), DatabaseError> {
        self.db
            .get_system_stats()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>, DatabaseError> {
        self.db
            .get_top_tools_analysis(user_id, start_time, end_time)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
