// ABOUTME: Security context for dependency injection of security-related services
// ABOUTME: Contains CSRF protection, PII redaction, and rate limiting for secure operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::middleware::redaction::RedactionConfig;
use crate::middleware::CsrfMiddleware;
use crate::oauth2_server::rate_limiting::OAuth2RateLimiter;
use crate::security::csrf::CsrfTokenManager;
use std::sync::Arc;

/// Security context containing security-related dependencies
///
/// This context provides all security-related dependencies needed for
/// request protection, PII handling, and rate limiting.
///
/// # Dependencies
/// - `redaction_config`: Configuration for PII redaction in logs and responses
/// - `oauth2_rate_limiter`: Rate limiter for `OAuth2` endpoints
/// - `csrf_manager`: CSRF token manager for request forgery protection
/// - `csrf_middleware`: CSRF validation middleware
#[derive(Clone)]
pub struct SecurityContext {
    redaction_config: Arc<RedactionConfig>,
    oauth2_rate_limiter: Arc<OAuth2RateLimiter>,
    csrf_manager: Arc<CsrfTokenManager>,
    csrf_middleware: Arc<CsrfMiddleware>,
}

impl SecurityContext {
    /// Create new security context
    #[must_use]
    pub const fn new(
        redaction_config: Arc<RedactionConfig>,
        oauth2_rate_limiter: Arc<OAuth2RateLimiter>,
        csrf_manager: Arc<CsrfTokenManager>,
        csrf_middleware: Arc<CsrfMiddleware>,
    ) -> Self {
        Self {
            redaction_config,
            oauth2_rate_limiter,
            csrf_manager,
            csrf_middleware,
        }
    }

    /// Get redaction configuration for PII handling
    #[must_use]
    pub const fn redaction_config(&self) -> &Arc<RedactionConfig> {
        &self.redaction_config
    }

    /// Get `OAuth2` rate limiter for endpoint protection
    #[must_use]
    pub const fn oauth2_rate_limiter(&self) -> &Arc<OAuth2RateLimiter> {
        &self.oauth2_rate_limiter
    }

    /// Get CSRF token manager for request forgery protection
    #[must_use]
    pub const fn csrf_manager(&self) -> &Arc<CsrfTokenManager> {
        &self.csrf_manager
    }

    /// Get CSRF validation middleware
    #[must_use]
    pub const fn csrf_middleware(&self) -> &Arc<CsrfMiddleware> {
        &self.csrf_middleware
    }
}
