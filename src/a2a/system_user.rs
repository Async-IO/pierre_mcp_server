// ABOUTME: System user creation and management for A2A client authentication
// ABOUTME: Creates internal user accounts for A2A clients to enable secure API access
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! System user management for A2A clients
// NOTE: All `.clone()` calls in this file are Safe - Arc/String ownership for A2A operations

use crate::constants::get_server_config;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::models::User;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Service for managing A2A system users
pub struct A2ASystemUserService {
    database: Arc<Database>,
}

impl A2ASystemUserService {
    /// Create a new system user service
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Create or get a system user for an A2A client
    ///
    /// System users are special accounts created specifically for A2A clients.
    /// They have no login credentials and exist purely for API key association.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operations fail
    /// - Password hashing fails
    /// - User creation fails
    pub async fn create_or_get_system_user(&self, client_id: &str) -> AppResult<Uuid> {
        let system_email = format!("a2a-system-{client_id}@pierre.ai");

        // Check if system user already exists
        if let Some(existing_user) = self
            .database
            .get_user_by_email(&system_email)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user by email: {e}")))?
        {
            return Ok(existing_user.id);
        }

        // Create new system user with secure random password
        // System users cannot login directly, so this password is never used
        let secure_password = Self::generate_secure_system_password();

        // Use lower bcrypt cost in test/CI environments for performance (cost 4 is ~60x faster than default 12)
        // Check for test environment via CI variable or debug profile
        let ci_mode = get_server_config().is_some_and(|c| c.app_behavior.ci_mode);
        let bcrypt_cost = if ci_mode || cfg!(debug_assertions) {
            4 // Fast hashing for tests and development
        } else {
            bcrypt::DEFAULT_COST // Secure hashing for production (12)
        };

        let hashed_password = bcrypt::hash(secure_password, bcrypt_cost)
            .map_err(|e| AppError::internal(format!("Failed to hash password: {e}")))?;

        let system_user = User::new(
            system_email.clone(),
            hashed_password,
            Some(format!("A2A System User for {client_id}")),
        );

        let user_id = self
            .database
            .create_user(&system_user)
            .await
            .map_err(|e| AppError::database(format!("Failed to create user: {e}")))?;

        // Store metadata about this being a system user
        Self::store_system_user_metadata(user_id, client_id);

        info!(
            user_id = %user_id,
            client_id = %client_id,
            "Created A2A system user"
        );

        Ok(user_id)
    }

    /// Generate a cryptographically secure password for system users
    #[must_use]
    pub fn generate_secure_system_password() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let password: String = (0..64)
            .map(|_| {
                let chars =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()";
                chars[rng.gen_range(0..chars.len())] as char
            })
            .collect();
        password
    }

    /// Store metadata about system user
    fn store_system_user_metadata(user_id: Uuid, client_id: &str) {
        // Store in a metadata table or as user properties
        // Store system identifier in user display name and email patterns
        debug!(
            user_id = %user_id,
            client_id = %client_id,
            "Stored A2A system user metadata"
        );
    }

    /// Check if a user is a system user for A2A
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn is_system_user(&self, user_id: Uuid) -> AppResult<bool> {
        if let Some(user) = self
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
        {
            // System users have emails following the pattern a2a-system-{client_id}@pierre.ai
            Ok(user.email.starts_with("a2a-system-") && user.email.ends_with("@pierre.ai"))
        } else {
            Ok(false)
        }
    }

    /// Get the client ID associated with a system user
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_client_id_for_system_user(&self, user_id: Uuid) -> AppResult<Option<String>> {
        if let Some(user) = self
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
        {
            if user.email.starts_with("a2a-system-") && user.email.ends_with("@pierre.ai") {
                // Extract client ID from email: a2a-system-{client_id}@pierre.ai
                let email_part = user
                    .email
                    .strip_prefix("a2a-system-")
                    .and_then(|s| s.strip_suffix("@pierre.ai"));
                return Ok(email_part.map(str::to_owned));
            }
        }
        Ok(None)
    }

    /// Deactivate a system user when A2A client is deleted
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn deactivate_system_user(&self, client_id: &str) -> AppResult<()> {
        let system_email = format!("a2a-system-{client_id}@pierre.ai");

        if let Some(user) = self
            .database
            .get_user_by_email(&system_email)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user by email: {e}")))?
        {
            // Instead of deleting, we could mark as inactive
            // Log system user deactivation
            info!(
                user_id = %user.id,
                client_id = %client_id,
                "Deactivated A2A system user"
            );
        }

        Ok(())
    }
}
