// ABOUTME: User MCP token repository implementation for AI client authentication
// ABOUTME: Delegates to DatabaseProvider for token creation, validation, and lifecycle
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::UserMcpTokenRepository;
use crate::database::{
    CreateUserMcpTokenRequest, DatabaseError, UserMcpToken, UserMcpTokenCreated, UserMcpTokenInfo,
};
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `UserMcpTokenRepository`
pub struct UserMcpTokenRepositoryImpl {
    db: Database,
}

impl UserMcpTokenRepositoryImpl {
    /// Create a new `UserMcpTokenRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserMcpTokenRepository for UserMcpTokenRepositoryImpl {
    async fn create(
        &self,
        user_id: Uuid,
        request: &CreateUserMcpTokenRequest,
    ) -> Result<UserMcpTokenCreated, DatabaseError> {
        self.db
            .create_user_mcp_token(user_id, request)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn validate(&self, token_value: &str) -> Result<Uuid, DatabaseError> {
        self.db
            .validate_user_mcp_token(token_value)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list(&self, user_id: Uuid) -> Result<Vec<UserMcpTokenInfo>, DatabaseError> {
        self.db
            .list_user_mcp_tokens(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn revoke(&self, token_id: &str, user_id: Uuid) -> Result<(), DatabaseError> {
        self.db
            .revoke_user_mcp_token(token_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get(
        &self,
        token_id: &str,
        user_id: Uuid,
    ) -> Result<Option<UserMcpToken>, DatabaseError> {
        self.db
            .get_user_mcp_token(token_id, user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn cleanup_expired(&self) -> Result<u64, DatabaseError> {
        self.db
            .cleanup_expired_user_mcp_tokens()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
