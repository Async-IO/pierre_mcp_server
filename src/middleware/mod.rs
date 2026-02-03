// ABOUTME: HTTP middleware for request tracing, authentication, and context propagation
// ABOUTME: Provides request ID generation, span creation, and tenant context for structured logging
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Authentication middleware for MCP and API requests
pub mod auth;
/// CORS middleware configuration
pub mod cors;
/// CSRF validation middleware
pub mod csrf;
/// Rate limiting middleware and utilities
pub mod rate_limiting;
/// PII redaction and sensitive data masking
pub mod redaction;
/// Request ID generation and propagation
pub mod request_id;
/// Tenant context extraction middleware
pub mod tenant;
/// Request tracing and context propagation
pub mod tracing;

// Authentication middleware

/// MCP authentication middleware
pub use auth::McpAuthMiddleware;
/// CSRF validation middleware
pub use csrf::CsrfMiddleware;

// CORS middleware

/// Setup CORS layer for HTTP endpoints
pub use cors::setup_cors;

// Rate limiting middleware and utilities

/// Check rate limit and send error response
pub use rate_limiting::check_rate_limit_and_respond;
/// Create rate limit error
pub use rate_limiting::create_rate_limit_error;
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

// Request ID middleware

/// Request ID middleware function
pub use request_id::request_id_middleware;
/// Request ID extractor
pub use request_id::RequestId;

// Request tracing and context management

/// Create database operation span
pub use tracing::create_database_span;
/// Create MCP operation span
pub use tracing::create_mcp_span;
/// Create HTTP request span
pub use tracing::create_request_span;
/// Request context for tracing
pub use tracing::RequestContext;

// Tenant context middleware

/// Require tenant context helper function
pub use tenant::require_tenant_context;
/// Tenant context extraction middleware
pub use tenant::tenant_context_middleware;
/// Extracted tenant context wrapper for request extensions
pub use tenant::ExtractedTenantContext;
