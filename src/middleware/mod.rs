// ABOUTME: HTTP middleware for request tracing, authentication, and context propagation
// ABOUTME: Provides request ID generation, span creation, and tenant context for structured logging

pub mod auth;
pub mod cors;
pub mod rate_limiting;
pub mod tracing;

pub use auth::McpAuthMiddleware;
pub use cors::setup_cors;
pub use rate_limiting::*;
pub use tracing::*;
