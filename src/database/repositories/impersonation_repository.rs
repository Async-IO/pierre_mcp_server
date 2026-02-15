// ABOUTME: Impersonation session repository implementation for audit trail
// ABOUTME: Delegates to DatabaseProvider for impersonation session lifecycle
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::ImpersonationRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use crate::permissions::impersonation::ImpersonationSession;
use async_trait::async_trait;
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `ImpersonationRepository`
pub struct ImpersonationRepositoryImpl {
    db: Database,
}

impl ImpersonationRepositoryImpl {
    /// Create a new `ImpersonationRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ImpersonationRepository for ImpersonationRepositoryImpl {
    async fn create_session(&self, session: &ImpersonationSession) -> Result<(), DatabaseError> {
        self.db
            .create_impersonation_session(session)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ImpersonationSession>, DatabaseError> {
        self.db
            .get_impersonation_session(session_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_active_session(
        &self,
        user_id: Uuid,
    ) -> Result<Option<ImpersonationSession>, DatabaseError> {
        self.db
            .get_active_impersonation_session(user_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn end_session(&self, session_id: &str) -> Result<(), DatabaseError> {
        self.db
            .end_impersonation_session(session_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn end_all_sessions(&self, impersonator_id: Uuid) -> Result<u64, DatabaseError> {
        self.db
            .end_all_impersonation_sessions(impersonator_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_sessions(
        &self,
        impersonator_id: Option<Uuid>,
        target_user_id: Option<Uuid>,
        active_only: bool,
        limit: u32,
    ) -> Result<Vec<ImpersonationSession>, DatabaseError> {
        self.db
            .list_impersonation_sessions(impersonator_id, target_user_id, active_only, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
