// ABOUTME: Provider connection repository for tracking external fitness provider links
// ABOUTME: Delegates to DatabaseProvider for tenant-scoped provider connection management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::ProviderConnectionRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::models::{ConnectionType, ProviderConnection};
use async_trait::async_trait;
use pierre_core::models::TenantId;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `ProviderConnectionRepository`
pub struct ProviderConnectionRepositoryImpl {
    db: Database,
}

impl ProviderConnectionRepositoryImpl {
    /// Create a new `ProviderConnectionRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProviderConnectionRepository for ProviderConnectionRepositoryImpl {
    async fn register(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        connection_type: &ConnectionType,
        metadata: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.db
            .register_provider_connection(user_id, tenant_id, provider, connection_type, metadata)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn remove(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .remove_provider_connection(user_id, tenant_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_user_connections(
        &self,
        user_id: Uuid,
        tenant_id: Option<TenantId>,
    ) -> Result<Vec<ProviderConnection>, DatabaseError> {
        self.db
            .get_user_provider_connections(user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn is_connected(&self, user_id: Uuid, provider: &str) -> Result<bool, DatabaseError> {
        self.db
            .is_provider_connected(user_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
