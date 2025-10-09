// ABOUTME: Authentication context for dependency injection of auth-related services
// ABOUTME: Contains auth manager, middleware, and JWT secret for authentication operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::auth::AuthManager;
use crate::middleware::McpAuthMiddleware;
use std::sync::Arc;

/// Authentication context containing auth-related dependencies
///
/// This context provides all authentication and authorization dependencies
/// needed for request processing, token validation, and user authentication.
///
/// # Dependencies
/// - `auth_manager`: Core authentication and token management
/// - `auth_middleware`: MCP-specific authentication middleware
/// - `admin_jwt_secret`: Secret for admin JWT token validation
#[derive(Clone)]
pub struct AuthContext {
    auth_manager: Arc<AuthManager>,
    auth_middleware: Arc<McpAuthMiddleware>,
    admin_jwt_secret: Arc<str>,
}

impl AuthContext {
    /// Create new authentication context
    #[must_use]
    pub const fn new(
        auth_manager: Arc<AuthManager>,
        auth_middleware: Arc<McpAuthMiddleware>,
        admin_jwt_secret: Arc<str>,
    ) -> Self {
        Self {
            auth_manager,
            auth_middleware,
            admin_jwt_secret,
        }
    }

    /// Get auth manager for token operations
    #[must_use]
    pub const fn auth_manager(&self) -> &Arc<AuthManager> {
        &self.auth_manager
    }

    /// Get auth middleware for request processing
    #[must_use]
    pub const fn auth_middleware(&self) -> &Arc<McpAuthMiddleware> {
        &self.auth_middleware
    }

    /// Get admin JWT secret for token validation
    #[must_use]
    pub const fn admin_jwt_secret(&self) -> &Arc<str> {
        &self.admin_jwt_secret
    }
}
