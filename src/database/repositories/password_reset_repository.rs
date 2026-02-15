// ABOUTME: Password reset token repository for secure password recovery
// ABOUTME: Delegates to DatabaseProvider for hashed token storage and atomic consumption
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::PasswordResetRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `PasswordResetRepository`
pub struct PasswordResetRepositoryImpl {
    db: Database,
}

impl PasswordResetRepositoryImpl {
    /// Create a new `PasswordResetRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PasswordResetRepository for PasswordResetRepositoryImpl {
    async fn store_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        created_by: &str,
    ) -> Result<Uuid, DatabaseError> {
        self.db
            .store_password_reset_token(user_id, token_hash, created_by)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn consume_token(&self, token_hash: &str) -> Result<Uuid, DatabaseError> {
        self.db
            .consume_password_reset_token(token_hash)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn invalidate_user_tokens(&self, user_id: Uuid) -> Result<(), DatabaseError> {
        self.db
            .invalidate_user_reset_tokens(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
