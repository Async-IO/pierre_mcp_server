// ABOUTME: Encryption/decryption utilities for OAuth tokens and sensitive data.
// ABOUTME: Uses AES-256-GCM with AAD binding for secure data at rest across database backends.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Encryption/decryption utilities for OAuth tokens and sensitive data
//!
//! This module harmonizes encryption across PostgreSQL and SQLite, ensuring
//! consistent security for sensitive data at rest using AES-256-GCM with AAD binding.

use crate::errors::AppResult;
use uuid::Uuid;

/// Create AAD (Additional Authenticated Data) context for token encryption
///
/// Format: `"{tenant_id}|{user_id}|{provider}|{table}"`
///
/// This prevents cross-tenant token reuse attacks by binding the encrypted
/// token to its specific context. If an attacker copies an encrypted token
/// to a different tenant/user/provider context, decryption will fail due to
/// AAD mismatch.
///
/// # Arguments
/// * `tenant_id` - Tenant ID (or "default" for single-tenant)
/// * `user_id` - User UUID
/// * `provider` - OAuth provider (e.g., "strava", "fitbit", "google")
/// * `table` - Database table name (e.g., `"user_oauth_tokens"`)
///
/// # Returns
/// AAD context string in format: `"{tenant_id}|{user_id}|{provider}|{table}"`
///
/// # Examples
/// ```
/// # use pierre_mcp_server::database_plugins::shared::encryption::create_token_aad_context;
/// # use uuid::Uuid;
/// let user_id = Uuid::new_v4();
/// let aad = create_token_aad_context("tenant-123", user_id, "strava", "user_oauth_tokens");
/// assert!(aad.contains("tenant-123"));
/// assert!(aad.contains("strava"));
/// ```
#[must_use]
pub fn create_token_aad_context(
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
    table: &str,
) -> String {
    format!("{tenant_id}|{user_id}|{provider}|{table}")
}

/// Encrypt OAuth token with AAD binding
///
/// Uses AES-256-GCM encryption with Additional Authenticated Data to prevent
/// cross-tenant token reuse. The AAD context binds the encrypted token to
/// its specific tenant/user/provider combination.
///
/// # Arguments
/// * `db` - Database implementing `HasEncryption` trait
/// * `token` - Plain-text OAuth token to encrypt
/// * `tenant_id` - Tenant ID
/// * `user_id` - User UUID
/// * `provider` - OAuth provider name
///
/// # Returns
/// * `Ok(String)` - Base64-encoded encrypted token with nonce
///
/// # Errors
/// * Returns error if encryption fails
///
/// # Security
/// - Uses AES-256-GCM (AEAD cipher)
/// - Unique nonce per encryption
/// - AAD prevents token tampering and context switching
/// - Compliant with GDPR, HIPAA, SOC 2 encryption-at-rest requirements
///
/// # Examples
/// ```text
/// let encrypted = shared::encryption::encrypt_oauth_token(
///     db,
///     "access_token_here",
///     "tenant-123",
///     user_id,
///     "strava"
/// )?;
/// ```
pub fn encrypt_oauth_token<D>(
    db: &D,
    token: &str,
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
) -> AppResult<String>
where
    D: HasEncryption,
{
    let aad_context = create_token_aad_context(tenant_id, user_id, provider, "user_oauth_tokens");
    db.encrypt_data_with_aad(token, &aad_context)
}

/// Decrypt OAuth token with AAD binding
///
/// Reverses `encrypt_oauth_token`. The same AAD context used for encryption
/// MUST be provided or decryption will fail (authentication error).
///
/// # Arguments
/// * `db` - Database implementing `HasEncryption` trait
/// * `encrypted_token` - Base64-encoded encrypted token (from database)
/// * `tenant_id` - Tenant ID (must match encryption context)
/// * `user_id` - User UUID (must match encryption context)
/// * `provider` - OAuth provider name (must match encryption context)
///
/// # Returns
/// * `Ok(String)` - Decrypted plain-text token
///
/// # Errors
/// * Returns error if:
///   - Decryption fails (wrong key)
///   - AAD mismatch (token moved to different context)
///   - Data corrupted/tampered
///
/// # Security
/// AAD verification ensures the token hasn't been:
/// - Copied to a different tenant
/// - Reassigned to a different user
/// - Switched to a different provider
///
/// # Examples
/// ```text
/// let plain_token = shared::encryption::decrypt_oauth_token(
///     db,
///     &encrypted_from_db,
///     "tenant-123",
///     user_id,
///     "strava"
/// )?;
/// ```
pub fn decrypt_oauth_token<D>(
    db: &D,
    encrypted_token: &str,
    tenant_id: &str,
    user_id: Uuid,
    provider: &str,
) -> AppResult<String>
where
    D: HasEncryption,
{
    let aad_context = create_token_aad_context(tenant_id, user_id, provider, "user_oauth_tokens");
    db.decrypt_data_with_aad(encrypted_token, &aad_context)
}

/// Trait for databases that support encryption
///
/// Both `PostgreSQL` and `SQLite` must implement this trait to use shared
/// encryption helpers. This ensures consistent encryption behavior across
/// database backends.
///
/// # Implementation Requirements
/// - Must use AES-256-GCM (AEAD cipher)
/// - Must generate unique nonce per encryption
/// - Must bind AAD to ciphertext (prevents context switching)
/// - Must encode output as base64 for database storage
///
/// # Examples
/// ```text
/// impl HasEncryption for Database {
///     fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> Result<String> {
///         // AES-256-GCM implementation with ring crate
///         // See src/database/mod.rs:690 for reference
///     }
///
///     fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> Result<String> {
///         // Reverse of encrypt_data_with_aad
///         // See src/database/mod.rs:729 for reference
///     }
/// }
/// ```
pub trait HasEncryption {
    /// Encrypt data using AES-256-GCM with Additional Authenticated Data
    ///
    /// # Arguments
    /// * `data` - Plain-text data to encrypt
    /// * `aad` - Additional Authenticated Data (context binding)
    ///
    /// # Returns
    /// Base64-encoded string containing: nonce (12 bytes) + ciphertext + auth tag
    ///
    /// # Errors
    /// Returns error if encryption fails
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> AppResult<String>;

    /// Decrypt data using AES-256-GCM with Additional Authenticated Data
    ///
    /// # Arguments
    /// * `encrypted` - Base64-encoded encrypted data (from `encrypt_data_with_aad`)
    /// * `aad` - Additional Authenticated Data (MUST match encryption AAD)
    ///
    /// # Returns
    /// Decrypted plain-text data
    ///
    /// # Errors
    /// Returns error if AAD doesn't match or data is tampered/corrupted
    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> AppResult<String>;
}
