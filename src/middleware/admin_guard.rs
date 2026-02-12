// ABOUTME: Central admin authorization guard for routes requiring admin privileges
// ABOUTME: Verifies user has admin role and returns 403 Forbidden if not authorized
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Admin Authorization Guard
//!
//! This module provides centralized admin authorization checking for route handlers.
//! Instead of each handler performing inline `user.role.is_admin_or_higher()` checks,
//! handlers can use the `require_admin` helper function.
//!
//! # Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::auth::AuthResult;
//! use pierre_mcp_server::database_plugins::factory::Database;
//! use pierre_mcp_server::middleware::admin_guard::require_admin;
//! use std::sync::Arc;
//!
//! async fn admin_handler(
//!     auth: AuthResult,
//!     database: Arc<Database>,
//! ) -> Result<String, pierre_mcp_server::errors::AppError> {
//!     // This verifies admin role and returns the User if authorized
//!     let admin_user = require_admin(auth.user_id, &database).await?;
//!     Ok(format!("Welcome admin: {}", admin_user.email))
//! }
//! ```

use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, ErrorCode};
use pierre_core::models::User;
use std::sync::Arc;
use uuid::Uuid;

/// Require admin privileges for a user
///
/// Verifies that the authenticated user has admin role (admin or `super_admin`).
/// Returns the User record if authorized, or 403 Forbidden if not.
///
/// # Arguments
///
/// * `user_id` - The authenticated user's ID (from `AuthResult.user_id`)
/// * `database` - Database connection for user lookup
///
/// # Errors
///
/// Returns an error if:
/// - User not found in database
/// - Database query fails
/// - User does not have admin role (returns 403 Forbidden)
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::auth::AuthResult;
/// use pierre_mcp_server::middleware::admin_guard::require_admin;
///
/// # async fn example(auth: AuthResult, db: std::sync::Arc<pierre_mcp_server::database_plugins::factory::Database>) -> Result<(), pierre_mcp_server::errors::AppError> {
/// let admin = require_admin(auth.user_id, &db).await?;
/// println!("Admin {} authorized", admin.email);
/// # Ok(())
/// # }
/// ```
pub async fn require_admin(user_id: Uuid, database: &Arc<Database>) -> Result<User, AppError> {
    let user = database
        .get_user(user_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to get user: {e}")))?
        .ok_or_else(|| AppError::not_found("User not found"))?;

    if !user.role.is_admin_or_higher() {
        return Err(AppError::new(
            ErrorCode::PermissionDenied,
            "Admin privileges required",
        ));
    }

    Ok(user)
}
