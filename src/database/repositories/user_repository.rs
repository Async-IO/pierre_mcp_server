// ABOUTME: User account management repository implementation
// ABOUTME: Handles user creation, retrieval, status updates, and pagination
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::UserRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::models::{User, UserStatus};
use crate::pagination::{CursorPage, PaginationParams};
use async_trait::async_trait;
use pierre_core::models::TenantId;
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

    async fn get_by_id(&self, id: Uuid, tenant_id: TenantId) -> Result<Option<User>, DatabaseError> {
        self.db
            .get_user(id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_by_id_global(&self, id: Uuid) -> Result<Option<User>, DatabaseError> {
        self.db
            .get_user_global(id)
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

    async fn get_by_firebase_uid(&self, firebase_uid: &str) -> Result<Option<User>, DatabaseError> {
        self.db
            .get_user_by_firebase_uid(firebase_uid)
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

    async fn list_by_status(
        &self,
        status: &str,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<User>, DatabaseError> {
        self.db
            .get_users_by_status(status, tenant_id)
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
        approved_by: Option<Uuid>,
    ) -> Result<User, DatabaseError> {
        self.db
            .update_user_status(id, new_status, approved_by)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_tenant_id(&self, id: Uuid, tenant_id: TenantId) -> Result<(), DatabaseError> {
        self.db
            .update_user_tenant_id(id, tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<(), DatabaseError> {
        self.db
            .update_user_password(id, password_hash)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_display_name(&self, id: Uuid, display_name: &str) -> Result<User, DatabaseError> {
        self.db
            .update_user_display_name(id, display_name)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.db
            .delete_user(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_first_admin(&self) -> Result<Option<User>, DatabaseError> {
        self.db
            .get_first_admin_user()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
