// ABOUTME: OAuth token storage repository implementation
// ABOUTME: Handles OAuth tokens, app credentials, and provider sync tracking
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::OAuthTokenRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::models::{UserOAuthApp, UserOAuthToken};
use async_trait::async_trait;
use pierre_core::models::TenantId;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `OAuthTokenRepository`
pub struct OAuthTokenRepositoryImpl {
    db: Database,
}

impl OAuthTokenRepositoryImpl {
    /// Create a new `OAuthTokenRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OAuthTokenRepository for OAuthTokenRepositoryImpl {
    async fn upsert(&self, token: &UserOAuthToken) -> Result<(), DatabaseError> {
        self.db
            .upsert_user_oauth_token(token)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>, DatabaseError> {
        self.db
            .get_user_oauth_token(user_id, tenant_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_by_user(
        &self,
        user_id: Uuid,
        tenant_id: Option<TenantId>,
    ) -> Result<Vec<UserOAuthToken>, DatabaseError> {
        self.db
            .get_user_oauth_tokens(user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_by_tenant_provider(
        &self,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>, DatabaseError> {
        self.db
            .get_tenant_provider_tokens(tenant_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .delete_user_oauth_token(user_id, tenant_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_all_for_user(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<(), DatabaseError> {
        self.db
            .delete_user_oauth_tokens(user_id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn refresh(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), DatabaseError> {
        self.db
            .refresh_user_oauth_token(
                user_id,
                tenant_id,
                provider,
                access_token,
                refresh_token,
                expires_at,
            )
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_user_oauth_app(user_id, provider, client_id, client_secret, redirect_uri)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>, DatabaseError> {
        self.db
            .get_user_oauth_app(user_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>, DatabaseError> {
        self.db
            .list_user_oauth_apps(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn remove_app(&self, user_id: Uuid, provider: &str) -> Result<(), DatabaseError> {
        self.db
            .remove_user_oauth_app(user_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>, DatabaseError> {
        self.db
            .get_provider_last_sync(user_id, tenant_id, provider)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_last_sync(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_provider_last_sync(user_id, tenant_id, provider, sync_time)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
