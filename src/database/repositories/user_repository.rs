// ABOUTME: User account management repository implementation
// ABOUTME: Handles user creation, retrieval, status updates, and pagination
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::UserRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::models::{User, UserStatus};
use crate::pagination::{CursorPage, PaginationParams};
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `UserRepository`
pub struct UserRepositoryImpl {
    db: Database,
}

impl UserRepositoryImpl {
    /// Create a new `UserRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn create(&self, user: &User) -> Result<Uuid, DatabaseError> {
        self.db
            .create_user(user)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, DatabaseError> {
        self.db
            .get_user(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError> {
        self.db
            .get_user_by_email(email)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_email_required(&self, email: &str) -> Result<User, DatabaseError> {
        self.db
            .get_user_by_email_required(email)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_last_active(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.db
            .update_last_active(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_count(&self) -> Result<i64, DatabaseError> {
        self.db
            .get_user_count()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_by_status(&self, status: &str) -> Result<Vec<User>, DatabaseError> {
        self.db
            .get_users_by_status(status)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_by_status_paginated(
        &self,
        status: &str,
        pagination: &PaginationParams,
    ) -> Result<CursorPage<User>, DatabaseError> {
        self.db
            .get_users_by_status_cursor(status, pagination)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_status(
        &self,
        id: Uuid,
        new_status: UserStatus,
        admin_token_id: &str,
    ) -> Result<User, DatabaseError> {
        self.db
            .update_user_status(id, new_status, admin_token_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_tenant_id(&self, id: Uuid, tenant_id: &str) -> Result<(), DatabaseError> {
        self.db
            .update_user_tenant_id(id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
