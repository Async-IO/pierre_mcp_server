// ABOUTME: HTTP middleware for request tracing, authentication, and context propagation
// ABOUTME: Provides request ID generation, span creation, and tenant context for structured logging

pub mod auth;
pub mod cors;
pub mod rate_limiting;
pub mod tracing;

// Authentication middleware
pub use auth::McpAuthMiddleware;

// CORS configuration
pub use cors::setup_cors;

// Rate limiting middleware and utilities
pub use rate_limiting::{
    check_rate_limit_and_respond, create_rate_limit_error, create_rate_limit_error_json,
    create_rate_limit_headers, headers,
};

// Request tracing and context management
pub use tracing::{
    create_database_span, create_mcp_span, create_request_span, with_request_tracing,
    RequestContext,
};
