// ABOUTME: Structured error types for fitness provider operations using thiserror
// ABOUTME: Provides domain-specific errors with retry information and rate limit handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use thiserror::Error;

/// Provider operation errors with structured context
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Provider API is unavailable or returning errors
    #[error("Provider {provider} API error: {status_code} - {message}")]
    ApiError {
        provider: String,
        status_code: u16,
        message: String,
        retryable: bool,
    },

    /// Rate limit exceeded with retry information
    #[error("Rate limit exceeded for {provider}: retry after {retry_after_secs} seconds")]
    RateLimitExceeded {
        provider: String,
        retry_after_secs: u64,
        limit_type: String,
    },

    /// Authentication failed or token expired
    #[error("Authentication failed for {provider}: {reason}")]
    AuthenticationFailed { provider: String, reason: String },

    /// Token refresh failed
    #[error("Token refresh failed for {provider}: {details}")]
    TokenRefreshFailed { provider: String, details: String },

    /// Resource not found
    #[error("{resource_type} '{resource_id}' not found in {provider}")]
    NotFound {
        provider: String,
        resource_type: String,
        resource_id: String,
    },

    /// Invalid data format from provider
    #[error("Invalid data format from {provider}: {field} - {reason}")]
    InvalidData {
        provider: String,
        field: String,
        reason: String,
    },

    /// Network error
    #[error("Network error communicating with {provider}: {0}")]
    NetworkError(String),

    /// Configuration error
    #[error("Provider {provider} configuration error: {details}")]
    ConfigurationError { provider: String, details: String },

    /// Generic provider error
    #[error("Provider operation failed: {0}")]
    Other(String),
}

impl ProviderError {
    /// Check if error is retryable
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::ApiError { retryable, .. } => *retryable,
            Self::RateLimitExceeded { .. } | Self::NetworkError(_) => true,
            Self::AuthenticationFailed { .. }
            | Self::TokenRefreshFailed { .. }
            | Self::NotFound { .. }
            | Self::InvalidData { .. }
            | Self::ConfigurationError { .. }
            | Self::Other(_) => false,
        }
    }

    /// Get retry delay in seconds if applicable
    #[must_use]
    pub const fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::RateLimitExceeded {
                retry_after_secs, ..
            } => Some(*retry_after_secs),
            _ => None,
        }
    }
}

/// Result type for provider operations
pub type ProviderResult<T> = Result<T, ProviderError>;
