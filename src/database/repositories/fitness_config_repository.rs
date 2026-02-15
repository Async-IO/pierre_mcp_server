// ABOUTME: Fitness configuration repository implementation
// ABOUTME: Handles tenant and user-specific fitness configuration storage
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::FitnessConfigRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use pierre_core::models::TenantId;

/// SQLite/PostgreSQL implementation of `FitnessConfigRepository`
pub struct FitnessConfigRepositoryImpl {
    db: Database,
}

impl FitnessConfigRepositoryImpl {
    /// Create a new `FitnessConfigRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl FitnessConfigRepository for FitnessConfigRepositoryImpl {
    async fn save_tenant_config(
        &self,
        tenant_id: TenantId,
        configuration_name: &str,
        config: &crate::config::fitness::FitnessConfig,
    ) -> Result<String, DatabaseError> {
        self.db
            .save_tenant_fitness_config(tenant_id, configuration_name, config)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn save_user_config(
        &self,
        tenant_id: TenantId,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness::FitnessConfig,
    ) -> Result<String, DatabaseError> {
        self.db
            .save_user_fitness_config(tenant_id, user_id, configuration_name, config)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_tenant_config(
        &self,
        tenant_id: TenantId,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness::FitnessConfig>, DatabaseError> {
        self.db
            .get_tenant_fitness_config(tenant_id, configuration_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_user_config(
        &self,
        tenant_id: TenantId,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness::FitnessConfig>, DatabaseError> {
        self.db
            .get_user_fitness_config(tenant_id, user_id, configuration_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_tenant_configs(&self, tenant_id: TenantId) -> Result<Vec<String>, DatabaseError> {
        self.db
            .list_tenant_fitness_configurations(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_user_configs(
        &self,
        tenant_id: TenantId,
        user_id: &str,
    ) -> Result<Vec<String>, DatabaseError> {
        self.db
            .list_user_fitness_configurations(tenant_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_config(
        &self,
        tenant_id: TenantId,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool, DatabaseError> {
        self.db
            .delete_fitness_config(tenant_id, user_id, configuration_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
