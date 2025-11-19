// ABOUTME: Insight repository implementation
// ABOUTME: Handles AI-generated insights storage and retrieval
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::InsightRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `InsightRepository`
pub struct InsightRepositoryImpl {
    db: Database,
}

impl InsightRepositoryImpl {
    /// Create a new `InsightRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl InsightRepository for InsightRepositoryImpl {
    async fn store(&self, user_id: Uuid, insight_data: Value) -> Result<String, DatabaseError> {
        self.db
            .store_insight(user_id, insight_data)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>, DatabaseError> {
        self.db
            .get_user_insights(user_id, insight_type, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
