// ABOUTME: Standardized error handling utilities for consistent error management
// ABOUTME: Provides helper functions and patterns for creating and handling errors consistently
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::Result;

use crate::errors::{AppError, ErrorCode};

/// Create a validation error with context
#[must_use]
pub fn validation_error(message: &str) -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        format!("Validation failed: {message}"),
    )
}

/// Create an authentication error with context
#[must_use]
pub fn auth_error(message: &str) -> AppError {
    AppError::new(
        ErrorCode::AuthInvalid,
        format!("Authentication failed: {message}"),
    )
}

/// Create a user state error with context
#[must_use]
pub fn user_state_error(message: &str) -> AppError {
    AppError::new(
        ErrorCode::PermissionDenied,
        format!("User state error: {message}"),
    )
}

/// Create a generic operation error with context
#[must_use]
pub fn operation_error(operation: &str, message: &str) -> AppError {
    AppError::new(
        ErrorCode::InternalError,
        format!("{operation} failed: {message}"),
    )
}

/// Helper to ensure we use consistent error patterns
pub trait ErrorContext<T> {
    /// Convert to validation error with context
    /// # Errors
    /// Returns validation error with provided context message
    fn with_validation_context(self, message: &str) -> Result<T, AppError>;
    /// Convert to authentication error with context
    /// # Errors
    /// Returns auth error with provided context message
    fn with_auth_context(self, message: &str) -> Result<T, AppError>;
    /// Convert to operation error with context
    /// # Errors
    /// Returns operation error with provided context message
    fn with_operation_context(self, operation: &str) -> Result<T, AppError>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    fn with_validation_context(self, message: &str) -> Result<T, AppError> {
        self.map_err(|e| validation_error(&format!("{message}: {e}")))
    }

    fn with_auth_context(self, message: &str) -> Result<T, AppError> {
        self.map_err(|e| auth_error(&format!("{message}: {e}")))
    }

    fn with_operation_context(self, operation: &str) -> Result<T, AppError> {
        self.map_err(|e| operation_error(operation, &format!("{e}")))
    }
}
