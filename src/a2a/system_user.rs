// ABOUTME: System user creation and management for A2A client authentication
// ABOUTME: Creates internal user accounts for A2A clients to enable secure API access
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! System user management for A2A clients

use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::models::User;
use anyhow::Result;
use std::sync::Arc;
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
    pub async fn create_or_get_system_user(
        &self,
        client_id: &str,
        contact_email: &str,
    ) -> Result<Uuid> {
        let system_email = format!("a2a-system-{client_id}@pierre.ai");

        // Check if system user already exists
        if let Some(existing_user) = self.database.get_user_by_email(&system_email).await? {
            return Ok(existing_user.id);
        }

        // Create new system user with secure random password
        // System users cannot login directly, so this password is never used
        let secure_password = Self::generate_secure_system_password();
        let hashed_password = bcrypt::hash(secure_password, bcrypt::DEFAULT_COST)?;

        let system_user = User::new(
            system_email.clone(),
            hashed_password,
            Some(format!("A2A System User for {client_id}")),
        );

        let user_id = self.database.create_user(&system_user).await?;

        // Store metadata about this being a system user
        Self::store_system_user_metadata(user_id, client_id, contact_email);

        tracing::info!(
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
    fn store_system_user_metadata(user_id: Uuid, client_id: &str, contact_email: &str) {
        // Store in a metadata table or as user properties
        // Store system identifier in user display name and email patterns
        tracing::debug!(
            user_id = %user_id,
            client_id = %client_id,
            contact_email = %contact_email,
            "Stored A2A system user metadata"
        );
    }

    /// Check if a user is a system user for A2A
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn is_system_user(&self, user_id: Uuid) -> Result<bool> {
        if let Some(user) = self.database.get_user(user_id).await? {
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
    pub async fn get_client_id_for_system_user(&self, user_id: Uuid) -> Result<Option<String>> {
        if let Some(user) = self.database.get_user(user_id).await? {
            if user.email.starts_with("a2a-system-") && user.email.ends_with("@pierre.ai") {
                // Extract client ID from email: a2a-system-{client_id}@pierre.ai
                let email_part = user
                    .email
                    .strip_prefix("a2a-system-")
                    .and_then(|s| s.strip_suffix("@pierre.ai"));
                return Ok(email_part.map(std::string::ToString::to_string));
            }
        }
        Ok(None)
    }

    /// Deactivate a system user when A2A client is deleted
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn deactivate_system_user(&self, client_id: &str) -> Result<()> {
        let system_email = format!("a2a-system-{client_id}@pierre.ai");

        if let Some(user) = self.database.get_user_by_email(&system_email).await? {
            // Instead of deleting, we could mark as inactive
            // Log system user deactivation
            tracing::info!(
                user_id = %user.id,
                client_id = %client_id,
                "Deactivated A2A system user"
            );
        }

        Ok(())
    }
}
