// ABOUTME: Structured error types for fitness provider operations using thiserror
// ABOUTME: Provides domain-specific errors with retry information and rate limit handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use thiserror::Error;

/// Provider operation errors with structured context
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Provider API is unavailable or returning errors
    #[error("Provider {provider} API error: {status_code} - {message}")]
    ApiError {
        /// Name of the fitness provider (e.g., "strava", "garmin")
        provider: String,
        /// HTTP status code from the provider
        status_code: u16,
        /// Error message from the provider
        message: String,
        /// Whether this error can be retried
        retryable: bool,
    },

    /// Rate limit exceeded with retry information
    #[error("Rate limit exceeded for {provider}: retry after {retry_after_secs} seconds")]
    RateLimitExceeded {
        /// Name of the fitness provider
        provider: String,
        /// Seconds to wait before retrying
        retry_after_secs: u64,
        /// Type of rate limit hit (e.g., "15-minute", "daily")
        limit_type: String,
    },

    /// Authentication failed or token expired
    #[error("Authentication failed for {provider}: {reason}")]
    AuthenticationFailed {
        /// Name of the fitness provider
        provider: String,
        /// Reason for authentication failure
        reason: String,
    },

    /// Token refresh failed
    #[error("Token refresh failed for {provider}: {details}")]
    TokenRefreshFailed {
        /// Name of the fitness provider
        provider: String,
        /// Details about the refresh failure
        details: String,
    },

    /// Resource not found
    #[error("{resource_type} '{resource_id}' not found in {provider}")]
    NotFound {
        /// Name of the fitness provider
        provider: String,
        /// Type of resource (e.g., "activity", "athlete")
        resource_type: String,
        /// ID of the resource that wasn't found
        resource_id: String,
    },

    /// Invalid data format from provider
    #[error("Invalid data format from {provider}: {field} - {reason}")]
    InvalidData {
        /// Name of the fitness provider
        provider: String,
        /// Field name with invalid data
        field: String,
        /// Reason why the data is invalid
        reason: String,
    },

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Configuration error
    #[error("Provider {provider} configuration error: {details}")]
    ConfigurationError {
        /// Name of the fitness provider
        provider: String,
        /// Details about the configuration issue
        details: String,
    },

    /// Feature not supported by provider
    #[error("Provider {provider} does not support {feature}")]
    UnsupportedFeature {
        /// Name of the fitness provider
        provider: String,
        /// Feature that is not supported
        feature: String,
    },

    /// HTTP client error
    #[error("HTTP client error for {provider}: {status}")]
    HttpError {
        /// Name of the fitness provider
        provider: String,
        /// HTTP status code
        status: u16,
        /// Response body
        body: String,
    },

    /// JSON parsing error
    #[error("Failed to parse {provider} response for field '{field}': {source}")]
    ParseError {
        /// Name of the fitness provider
        provider: String,
        /// Field that failed to parse
        field: &'static str,
        /// Underlying JSON error
        #[source]
        source: serde_json::Error,
    },

    /// Underlying reqwest error
    #[error("Network request failed for {provider}: {source}")]
    Reqwest {
        /// Name of the fitness provider
        provider: String,
        /// Underlying reqwest error
        #[source]
        source: reqwest::Error,
    },

    /// Timeout waiting for provider response
    #[error("Provider {provider} timeout: {operation} exceeded {timeout_secs}s")]
    Timeout {
        /// Name of the fitness provider
        provider: String,
        /// Operation that timed out
        operation: &'static str,
        /// Timeout duration in seconds
        timeout_secs: u64,
    },

    /// Provider-specific quota exceeded
    #[error("Provider {provider} quota exceeded: {quota_type}")]
    QuotaExceeded {
        /// Name of the fitness provider
        provider: String,
        /// Type of quota exceeded
        quota_type: String,
    },
}

impl ProviderError {
    /// Check if error is retryable
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::ApiError { retryable, .. } => *retryable,
            Self::RateLimitExceeded { .. }
            | Self::NetworkError(_)
            | Self::Timeout { .. }
            | Self::Reqwest { .. }
            | Self::HttpError { .. } => true,
            Self::AuthenticationFailed { .. }
            | Self::TokenRefreshFailed { .. }
            | Self::NotFound { .. }
            | Self::InvalidData { .. }
            | Self::ConfigurationError { .. }
            | Self::UnsupportedFeature { .. }
            | Self::ParseError { .. }
            | Self::QuotaExceeded { .. } => false,
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
