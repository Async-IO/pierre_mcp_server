// ABOUTME: Security repository implementation
// ABOUTME: Handles RSA keypairs, key rotation, audit events, and system secrets
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::SecurityRepository;
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// SQLite/PostgreSQL implementation of `SecurityRepository`
pub struct SecurityRepositoryImpl {
    db: Database,
}

impl SecurityRepositoryImpl {
    /// Create a new `SecurityRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SecurityRepository for SecurityRepositoryImpl {
    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: usize,
    ) -> Result<(), DatabaseError> {
        self.db
            .save_rsa_keypair(
                kid,
                private_key_pem,
                public_key_pem,
                created_at,
                is_active,
                key_size_bits.try_into().unwrap_or(2048),
            )
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>, DatabaseError> {
        self.db
            .load_rsa_keypairs()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_rsa_keypair_status(
        &self,
        kid: &str,
        is_active: bool,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_rsa_keypair_active_status(kid, is_active)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_key_version(
        &self,
        tenant_id: Option<Uuid>,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_key_version(tenant_id, version)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>, DatabaseError> {
        self.db
            .get_key_versions(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>, DatabaseError> {
        self.db
            .get_current_key_version(tenant_id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_key_version_status(tenant_id, version, is_active)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64, DatabaseError> {
        self.db
            .delete_old_key_versions(tenant_id, keep_count)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn store_audit_event(
        &self,
        tenant_id: Option<Uuid>,
        event: &crate::security::audit::AuditEvent,
    ) -> Result<(), DatabaseError> {
        self.db
            .store_audit_event(tenant_id, event)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>, DatabaseError> {
        self.db
            .get_audit_events(tenant_id, event_type, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_or_create_system_secret(
        &self,
        secret_type: &str,
    ) -> Result<String, DatabaseError> {
        self.db
            .get_or_create_system_secret(secret_type)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_system_secret(&self, secret_type: &str) -> Result<String, DatabaseError> {
        self.db
            .get_system_secret(secret_type)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn update_system_secret(
        &self,
        secret_type: &str,
        new_value: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .update_system_secret(secret_type, new_value)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
