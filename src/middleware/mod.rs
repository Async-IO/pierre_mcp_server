// ABOUTME: HTTP middleware for request tracing, authentication, and context propagation
// ABOUTME: Provides request ID generation, span creation, and tenant context for structured logging
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Authentication middleware for MCP and API requests
pub mod auth;
/// CORS (Cross-Origin Resource Sharing) configuration
pub mod cors;
/// Rate limiting middleware and utilities
pub mod rate_limiting;
/// PII redaction and sensitive data masking
pub mod redaction;
/// Request tracing and context propagation
pub mod tracing;

// Authentication middleware

/// MCP authentication middleware
pub use auth::McpAuthMiddleware;

// CORS configuration

/// Setup CORS middleware
pub use cors::setup_cors;

// Rate limiting middleware and utilities

/// Check rate limit and send error response
pub use rate_limiting::check_rate_limit_and_respond;
/// Create rate limit error
pub use rate_limiting::create_rate_limit_error;
/// Create rate limit error as JSON
pub use rate_limiting::create_rate_limit_error_json;
/// Create rate limit headers
pub use rate_limiting::create_rate_limit_headers;
/// Rate limit headers module
pub use rate_limiting::headers;

// PII-safe logging and redaction

/// Mask email addresses for logging
pub use redaction::mask_email;
/// Redact sensitive HTTP headers
pub use redaction::redact_headers;
/// Redact JSON fields by pattern
pub use redaction::redact_json_fields;
/// Redact token patterns from strings
pub use redaction::redact_token_patterns;
/// Bounded tenant label for tracing
pub use redaction::BoundedTenantLabel;
/// Bounded user label for tracing
pub use redaction::BoundedUserLabel;
/// Redaction configuration
pub use redaction::RedactionConfig;
/// Redaction features toggle
pub use redaction::RedactionFeatures;

// Request tracing and context management

/// Create database operation span
pub use tracing::create_database_span;
/// Create MCP operation span
pub use tracing::create_mcp_span;
/// Create HTTP request span
pub use tracing::create_request_span;
/// Add request tracing middleware
pub use tracing::with_request_tracing;
/// Request context for tracing
pub use tracing::RequestContext;
