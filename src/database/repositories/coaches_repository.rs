// ABOUTME: Coaches repository implementation for custom AI personas
// ABOUTME: Provides trait-based abstraction for coaches CRUD operations with tenant isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::CoachesRepository;
use crate::database::coaches::{Coach, CoachesManager, CreateCoachRequest, ListCoachesFilter, UpdateCoachRequest};
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use pierre_core::models::TenantId;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `CoachesRepository`
pub struct CoachesRepositoryImpl {
    db: Database,
}

impl CoachesRepositoryImpl {
    /// Create a new `CoachesRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }

    fn get_manager(&self) -> Option<CoachesManager> {
        self.db
            .sqlite_pool()
            .map(|pool| CoachesManager::new(pool.clone()))
    }
}

#[async_trait]
impl CoachesRepository for CoachesRepositoryImpl {
    async fn create(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &CreateCoachRequest,
    ) -> Result<Coach, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .create(user_id, tenant_id, request)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_id(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<Coach>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .get(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        filter: &ListCoachesFilter,
    ) -> Result<Vec<Coach>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .list(user_id, tenant_id, filter)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
        request: &UpdateCoachRequest,
    ) -> Result<Option<Coach>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .update(coach_id, user_id, tenant_id, request)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .delete(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn record_usage(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<bool, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .record_usage(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn toggle_favorite(
        &self,
        coach_id: &str,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Option<bool>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .toggle_favorite(coach_id, user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn search(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        query: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Coach>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .search(user_id, tenant_id, query, limit, offset)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn count(&self, user_id: Uuid, tenant_id: TenantId) -> Result<u32, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Coaches operations require SQLite backend".to_string(),
        })?;

        manager
            .count(user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
