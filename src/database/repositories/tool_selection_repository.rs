// ABOUTME: Tool selection repository implementation for per-tenant MCP tool configuration
// ABOUTME: Handles CRUD operations for tool catalog and tenant tool overrides
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::ToolSelectionRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::models::{TenantPlan, TenantToolOverride, ToolCatalogEntry, ToolCategory};
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `ToolSelectionRepository`
pub struct ToolSelectionRepositoryImpl {
    db: Database,
}

impl ToolSelectionRepositoryImpl {
    /// Create a new `ToolSelectionRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ToolSelectionRepository for ToolSelectionRepositoryImpl {
    async fn get_tool_catalog(&self) -> Result<Vec<ToolCatalogEntry>, DatabaseError> {
        self.db
            .get_tool_catalog()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_tool_catalog_entry(
        &self,
        tool_name: &str,
    ) -> Result<Option<ToolCatalogEntry>, DatabaseError> {
        self.db
            .get_tool_catalog_entry(tool_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_tools_by_category(
        &self,
        category: ToolCategory,
    ) -> Result<Vec<ToolCatalogEntry>, DatabaseError> {
        self.db
            .get_tools_by_category(category)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_tools_by_min_plan(
        &self,
        plan: TenantPlan,
    ) -> Result<Vec<ToolCatalogEntry>, DatabaseError> {
        self.db
            .get_tools_by_min_plan(plan)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_tenant_tool_overrides(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<TenantToolOverride>, DatabaseError> {
        self.db
            .get_tenant_tool_overrides(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
    ) -> Result<Option<TenantToolOverride>, DatabaseError> {
        self.db
            .get_tenant_tool_override(tenant_id, tool_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn upsert_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
        is_enabled: bool,
        enabled_by_user_id: Option<Uuid>,
        reason: Option<String>,
    ) -> Result<TenantToolOverride, DatabaseError> {
        self.db
            .upsert_tenant_tool_override(tenant_id, tool_name, is_enabled, enabled_by_user_id, reason)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_tenant_tool_override(
        &self,
        tenant_id: Uuid,
        tool_name: &str,
    ) -> Result<bool, DatabaseError> {
        self.db
            .delete_tenant_tool_override(tenant_id, tool_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn count_enabled_tools(&self, tenant_id: Uuid) -> Result<usize, DatabaseError> {
        self.db
            .count_enabled_tools(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
