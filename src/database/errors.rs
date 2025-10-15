// ABOUTME: Structured error types for database operations using thiserror
// ABOUTME: Provides domain-specific errors with context for better error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use thiserror::Error;

/// Database operation errors with structured context
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Entity not found in database
    #[error("Entity not found: {entity_type} with id '{entity_id}'")]
    NotFound {
        entity_type: &'static str,
        entity_id: String,
    },

    /// Cross-tenant access attempt detected
    #[error("Tenant isolation violation: attempted to access {entity_type} '{entity_id}' from tenant '{requested_tenant}' but it belongs to tenant '{actual_tenant}'")]
    TenantIsolationViolation {
        entity_type: &'static str,
        entity_id: String,
        requested_tenant: String,
        actual_tenant: String,
    },

    /// Encryption operation failed
    #[error("Encryption failed: {context}")]
    EncryptionFailed { context: String },

    /// Decryption operation failed
    #[error("Decryption failed: {context}")]
    DecryptionFailed { context: String },

    /// Database constraint violation
    #[error("Constraint violation: {constraint} - {details}")]
    ConstraintViolation { constraint: String, details: String },

    /// Database connection error
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// Database query error
    #[error("Query execution error: {context}")]
    QueryError { context: String },

    /// Migration error
    #[error("Migration failed: {version} - {details}")]
    MigrationError { version: String, details: String },

    /// Invalid data format
    #[error("Invalid data format for {field}: {reason}")]
    InvalidData { field: String, reason: String },

    /// Underlying `SQLx` error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// UUID parsing error
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    /// Generic error for conversion from anyhow
    #[error("Database operation failed: {0}")]
    Other(String),
}

impl From<anyhow::Error> for DatabaseError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}

/// Result type for database operations
pub type DatabaseResult<T> = Result<T, DatabaseError>;
