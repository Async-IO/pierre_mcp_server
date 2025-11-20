// ABOUTME: Admin token management repository implementation
// ABOUTME: Handles admin token creation, usage tracking, and provisioned keys
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::AdminRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// SQLite/PostgreSQL implementation of `AdminRepository`
pub struct AdminRepositoryImpl {
    db: Database,
}

impl AdminRepositoryImpl {
    /// Create a new `AdminRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AdminRepository for AdminRepositoryImpl {
    async fn create_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken, DatabaseError> {
        self.db
            .create_admin_token(request, admin_jwt_secret, jwks_manager)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>, DatabaseError> {
        self.db
            .get_admin_token_by_id(token_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_token_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>, DatabaseError> {
        self.db
            .get_admin_token_by_prefix(prefix)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>, DatabaseError> {
        self.db
            .list_admin_tokens(include_inactive)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn deactivate_token(&self, token_id: &str) -> Result<(), DatabaseError> {
        self.db
            .deactivate_admin_token(token_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_admin_token_last_used(token_id, ip_address)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn record_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<(), DatabaseError> {
        self.db
            .record_admin_token_usage(usage)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_usage_history(
        &self,
        token_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>, DatabaseError> {
        self.db
            .get_admin_token_usage_history(token_id, start, end)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn record_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .record_admin_provisioned_key(
                admin_token_id,
                api_key_id,
                user_email,
                tier,
                rate_limit_requests,
                rate_limit_period,
            )
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Value>, DatabaseError> {
        self.db
            .get_admin_provisioned_keys(admin_token_id, start, end)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
