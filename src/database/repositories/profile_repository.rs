// ABOUTME: Profile repository implementation
// ABOUTME: Handles user profiles, goals, and configuration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::ProfileRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `ProfileRepository`
pub struct ProfileRepositoryImpl {
    db: Database,
}

impl ProfileRepositoryImpl {
    /// Create a new `ProfileRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProfileRepository for ProfileRepositoryImpl {
    async fn upsert_profile(&self, user_id: Uuid, data: Value) -> Result<(), DatabaseError> {
        self.db
            .upsert_user_profile(user_id, data)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_profile(&self, user_id: Uuid) -> Result<Option<Value>, DatabaseError> {
        self.db
            .get_user_profile(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String, DatabaseError> {
        self.db
            .create_goal(user_id, goal_data)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_goals(&self, user_id: Uuid) -> Result<Vec<Value>, DatabaseError> {
        self.db
            .get_user_goals(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_goal_progress(
        &self,
        goal_id: &str,
        current_value: f64,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_goal_progress(goal_id, current_value)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_config(&self, user_id: &str) -> Result<Option<String>, DatabaseError> {
        self.db
            .get_user_configuration(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn save_config(&self, user_id: &str, config_json: &str) -> Result<(), DatabaseError> {
        self.db
            .save_user_configuration(user_id, config_json)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
