// ABOUTME: Structured error types for database operations using thiserror
// ABOUTME: Provides domain-specific errors with context for better error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use thiserror::Error;

/// Database operation errors with structured context
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Entity not found in database
    #[error("Entity not found: {entity_type} with id '{entity_id}'")]
    NotFound {
        /// Type of entity that was not found
        entity_type: &'static str,
        /// ID of the entity that was not found
        entity_id: String,
    },

    /// Cross-tenant access attempt detected
    #[error("Tenant isolation violation: attempted to access {entity_type} '{entity_id}' from tenant '{requested_tenant}' but it belongs to tenant '{actual_tenant}'")]
    TenantIsolationViolation {
        /// Type of entity being accessed
        entity_type: &'static str,
        /// ID of the entity being accessed
        entity_id: String,
        /// Tenant that attempted the access
        requested_tenant: String,
        /// Tenant that actually owns the entity
        actual_tenant: String,
    },

    /// Encryption operation failed
    #[error("Encryption failed: {context}")]
    EncryptionFailed {
        /// Context describing the encryption failure
        context: String,
    },

    /// Decryption operation failed
    #[error("Decryption failed: {context}")]
    DecryptionFailed {
        /// Context describing the decryption failure
        context: String,
    },

    /// Database constraint violation
    #[error("Constraint violation: {constraint} - {details}")]
    ConstraintViolation {
        /// Name of the constraint that was violated
        constraint: String,
        /// Details about the constraint violation
        details: String,
    },

    /// Database connection error
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// Database query error
    #[error("Query execution error: {context}")]
    QueryError {
        /// Context describing the query error
        context: String,
    },

    /// Migration error
    #[error("Migration failed: {version} - {details}")]
    MigrationError {
        /// Migration version that failed
        version: String,
        /// Details about the migration failure
        details: String,
    },

    /// Invalid data format
    #[error("Invalid data format for {field}: {reason}")]
    InvalidData {
        /// Field that has invalid data
        field: String,
        /// Reason why the data is invalid
        reason: String,
    },

    /// Underlying `SQLx` error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// UUID parsing error
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    /// Connection pool exhausted
    #[error(
        "Connection pool exhausted: {max_connections} connections in use, waited {wait_time_ms}ms"
    )]
    PoolExhausted {
        /// Maximum number of connections allowed
        max_connections: u32,
        /// Time waited for a connection in milliseconds
        wait_time_ms: u64,
    },

    /// Transaction rolled back
    #[error("Transaction rolled back: {reason}")]
    TransactionRollback {
        /// Reason for the rollback
        reason: &'static str,
    },

    /// Schema version mismatch
    #[error("Schema version mismatch: expected {expected}, found {actual}")]
    SchemaMismatch {
        /// Expected schema version
        expected: String,
        /// Actual schema version found
        actual: String,
    },

    /// Timeout waiting for database operation
    #[error("Database operation timeout: {operation} exceeded {timeout_secs}s")]
    Timeout {
        /// Operation that timed out
        operation: &'static str,
        /// Timeout duration in seconds
        timeout_secs: u64,
    },

    /// Database transaction conflict
    #[error("Transaction conflict: {details}")]
    TransactionConflict {
        /// Details about the conflict
        details: String,
    },
}

/// Result type for database operations
pub type DatabaseResult<T> = Result<T, DatabaseError>;
