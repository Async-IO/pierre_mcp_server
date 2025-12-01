// ABOUTME: API key management repository implementation
// ABOUTME: Handles API key creation, retrieval, deactivation, and cleanup
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::ApiKeyRepository;
use crate::api_keys::ApiKey;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `ApiKeyRepository`
pub struct ApiKeyRepositoryImpl {
    db: Database,
}

impl ApiKeyRepositoryImpl {
    /// Create a new `ApiKeyRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ApiKeyRepository for ApiKeyRepositoryImpl {
    async fn create(&self, key: &ApiKey) -> Result<(), DatabaseError> {
        self.db
            .create_api_key(key)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_prefix(
        &self,
        prefix: &str,
        hash: &str,
    ) -> Result<Option<ApiKey>, DatabaseError> {
        self.db
            .get_api_key_by_prefix(prefix, hash)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<ApiKey>, DatabaseError> {
        self.db
            .get_api_key_by_id(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<ApiKey>, DatabaseError> {
        self.db
            .get_user_api_keys(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_filtered(
        &self,
        user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>, DatabaseError> {
        self.db
            .get_api_keys_filtered(user_email, active_only, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_last_used(&self, id: &str) -> Result<(), DatabaseError> {
        self.db
            .update_api_key_last_used(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn deactivate(&self, id: &str, user_id: Uuid) -> Result<(), DatabaseError> {
        self.db
            .deactivate_api_key(id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn cleanup_expired(&self) -> Result<u64, DatabaseError> {
        self.db
            .cleanup_expired_api_keys()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_expired(&self) -> Result<Vec<ApiKey>, DatabaseError> {
        self.db
            .get_expired_api_keys()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
