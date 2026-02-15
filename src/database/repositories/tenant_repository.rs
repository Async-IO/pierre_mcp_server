// ABOUTME: Tenant management repository implementation
// ABOUTME: Handles multi-tenant support, OAuth credentials, and OAuth apps
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::TenantRepository;
use pierre_core::models::TenantId;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `TenantRepository`
pub struct TenantRepositoryImpl {
    db: Database,
}

impl TenantRepositoryImpl {
    /// Create a new `TenantRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TenantRepository for TenantRepositoryImpl {
    async fn create(&self, tenant: &crate::models::Tenant) -> Result<(), DatabaseError> {
        self.db
            .create_tenant(tenant)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_id(&self, id: TenantId) -> Result<crate::models::Tenant, DatabaseError> {
        self.db
            .get_tenant_by_id(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_slug(&self, slug: &str) -> Result<crate::models::Tenant, DatabaseError> {
        self.db
            .get_tenant_by_slug(slug)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Tenant>, DatabaseError> {
        self.db
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_all(&self) -> Result<Vec<crate::models::Tenant>, DatabaseError> {
        self.db
            .get_all_tenants()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_tenant_oauth_credentials(credentials)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_oauth_providers(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>, DatabaseError> {
        self.db
            .get_tenant_oauth_providers(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_oauth_credentials(
        &self,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>, DatabaseError> {
        self.db
            .get_tenant_oauth_credentials(tenant_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_user_role(
        &self,
        user_id: &str,
        tenant_id: TenantId,
    ) -> Result<Option<String>, DatabaseError> {
        let user_uuid = Uuid::parse_str(user_id)?;

        self.db
            .get_user_tenant_role(user_uuid, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<(), DatabaseError> {
        self.db
            .create_oauth_app(app)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_oauth_app_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<crate::models::OAuthApp, DatabaseError> {
        self.db
            .get_oauth_app_by_client_id(client_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>, DatabaseError> {
        self.db
            .list_oauth_apps_for_user(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
