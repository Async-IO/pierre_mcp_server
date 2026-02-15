// ABOUTME: LLM credential repository implementation for tenant-scoped API key management
// ABOUTME: Delegates to DatabaseProvider for encrypted LLM credential storage and retrieval
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::LlmCredentialRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::tenant::llm_manager::{LlmCredentialRecord, LlmCredentialSummary};
use async_trait::async_trait;
use pierre_core::models::TenantId;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `LlmCredentialRepository`
pub struct LlmCredentialRepositoryImpl {
    db: Database,
}

impl LlmCredentialRepositoryImpl {
    /// Create a new `LlmCredentialRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LlmCredentialRepository for LlmCredentialRepositoryImpl {
    async fn store(&self, record: &LlmCredentialRecord) -> Result<(), DatabaseError> {
        self.db
            .store_llm_credentials(record)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get(
        &self,
        tenant_id: TenantId,
        user_id: Option<Uuid>,
        provider: &str,
    ) -> Result<Option<LlmCredentialRecord>, DatabaseError> {
        self.db
            .get_llm_credentials(tenant_id, user_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<LlmCredentialSummary>, DatabaseError> {
        self.db
            .list_llm_credentials(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete(
        &self,
        tenant_id: TenantId,
        user_id: Option<Uuid>,
        provider: &str,
    ) -> Result<bool, DatabaseError> {
        self.db
            .delete_llm_credentials(tenant_id, user_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_admin_config_override(
        &self,
        config_key: &str,
        tenant_id: Option<TenantId>,
    ) -> Result<Option<String>, DatabaseError> {
        self.db
            .get_admin_config_override(config_key, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
