// ABOUTME: Standardized error handling utilities for consistent error management
// ABOUTME: Provides helper functions and patterns for creating and handling errors consistently
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt::{Debug, Display};
use std::result::Result;

use crate::errors::{AppError, AppResult, ErrorCode};

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
    fn with_validation_context(self, message: &str) -> AppResult<T>;
    /// Convert to authentication error with context
    /// # Errors
    /// Returns auth error with provided context message
    fn with_auth_context(self, message: &str) -> AppResult<T>;
    /// Convert to operation error with context
    /// # Errors
    /// Returns operation error with provided context message
    fn with_operation_context(self, operation: &str) -> AppResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Display + Debug + Send + Sync + 'static,
{
    fn with_validation_context(self, message: &str) -> AppResult<T> {
        self.map_err(|e| validation_error(&format!("{message}: {e}")))
    }

    fn with_auth_context(self, message: &str) -> AppResult<T> {
        self.map_err(|e| auth_error(&format!("{message}: {e}")))
    }

    fn with_operation_context(self, operation: &str) -> AppResult<T> {
        self.map_err(|e| operation_error(operation, &format!("{e}")))
    }
}
