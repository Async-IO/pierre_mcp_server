// ABOUTME: Core database operations module with encrypted storage and query management
// ABOUTME: Provides unified database interface, migrations, and encrypted data persistence
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Database Management
//!
//! This module provides database functionality for the multi-tenant Pierre MCP Server.
//! It handles user storage, token encryption, and secure data access patterns.

pub mod a2a;
mod analytics;
mod api_keys;
mod tokens;
mod users;

pub mod tests;

pub use a2a::{A2AUsage, A2AUsageStats};

use anyhow::Result;
use sqlx::{Pool, Sqlite, SqlitePool};

/// Database manager for user and token storage
#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
    encryption_key: Vec<u8>,
}

impl Database {
    /// Create a new database connection
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database URL is invalid or malformed
    /// - Database connection fails
    /// - `SQLite` file creation fails
    /// - Migration process fails
    /// - Encryption key is invalid
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Ensure SQLite creates the database file if it doesn't exist
        let connection_options = if database_url.starts_with("sqlite:") {
            format!("{database_url}?mode=rwc")
        } else {
            database_url.to_string()
        };

        let pool = SqlitePool::connect(&connection_options).await?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    /// Get a reference to the database pool for advanced operations
    #[must_use]
    pub const fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Run database migrations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Migration SQL statements fail to execute
    /// - Database schema creation fails
    /// - Insufficient database permissions
    /// - Database connection is lost during migration
    pub async fn migrate(&self) -> Result<()> {
        // User tables
        self.migrate_users().await?;

        // API key tables
        self.migrate_api_keys().await?;

        // Analytics tables
        self.migrate_analytics().await?;

        // A2A tables
        self.migrate_a2a().await?;

        // Admin tables
        self.migrate_admin().await?;

        // Tenant management tables
        self.migrate_tenant_management().await?;

        // Security and key rotation tables
        self.migrate_security().await?;

        Ok(())
    }

    /// Create admin tables
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table creation SQL fails
    /// - Index creation fails
    /// - Database constraints cannot be applied
    /// - SQL syntax errors in migration statements
    async fn migrate_admin(&self) -> Result<()> {
        // Create admin_tokens table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS admin_tokens (
                id TEXT PRIMARY KEY,
                service_name TEXT NOT NULL,
                service_description TEXT,
                token_hash TEXT NOT NULL,
                token_prefix TEXT NOT NULL,
                jwt_secret_hash TEXT NOT NULL,
                permissions TEXT NOT NULL DEFAULT '["provision_keys"]',
                is_super_admin BOOLEAN NOT NULL DEFAULT false,
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME,
                last_used_at DATETIME,
                last_used_ip TEXT,
                usage_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create admin_token_usage table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                action TEXT NOT NULL,
                target_resource TEXT,
                ip_address TEXT,
                user_agent TEXT,
                request_size_bytes INTEGER,
                success BOOLEAN NOT NULL,
                error_message TEXT,
                response_time_ms INTEGER
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create admin_provisioned_keys table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS admin_provisioned_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
                api_key_id TEXT NOT NULL,
                user_email TEXT NOT NULL,
                requested_tier TEXT NOT NULL,
                provisioned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                provisioned_by_service TEXT NOT NULL,
                rate_limit_requests INTEGER NOT NULL,
                rate_limit_period TEXT NOT NULL,
                key_status TEXT NOT NULL DEFAULT 'active',
                revoked_at DATETIME,
                revoked_reason TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for admin tables
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_service ON admin_tokens(service_name)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_tokens_prefix ON admin_tokens(token_prefix)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_usage_token_id ON admin_token_usage(admin_token_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_admin_usage_timestamp ON admin_token_usage(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_admin_provisioned_token ON admin_provisioned_keys(admin_token_id)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create tenant management tables
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table creation SQL fails
    /// - Index creation fails
    /// - Database constraints cannot be applied
    /// - SQL syntax errors in migration statements
    async fn migrate_tenant_management(&self) -> Result<()> {
        // Read and execute the tenant management migration
        let migration_sql = include_str!("../../migrations/004_tenant_management.sql");

        // Execute the migration in a transaction
        let mut tx = self.pool.begin().await?;

        // Split SQL statements properly, handling triggers with BEGIN/END blocks
        let statements = Self::split_sql_statements_properly(migration_sql);

        for statement in statements {
            let statement = statement.trim();
            if !statement.is_empty() {
                sqlx::query(statement).execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;

        tracing::info!("Tenant management tables migration completed successfully");
        Ok(())
    }

    /// Create security and key rotation tables
    ///
    /// # Errors
    ///
    /// Returns an error if the security migration fails
    async fn migrate_security(&self) -> Result<()> {
        // Read and execute the security migration
        let migration_sql = include_str!("../../migrations/005_security.sql");

        // Execute the migration in a transaction
        let mut tx = self.pool.begin().await?;

        // Split SQL statements properly
        let statements = Self::split_sql_statements_properly(migration_sql);

        for statement in statements {
            sqlx::query(&statement).execute(&mut *tx).await?;
        }

        tx.commit().await?;

        tracing::info!("Security tables migration completed successfully");
        Ok(())
    }

    /// Split SQL text into individual statements, properly handling triggers
    fn split_sql_statements_properly(sql: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut in_trigger = false;
        let mut trigger_depth = 0;

        for line in sql.lines() {
            let trimmed = line.trim();

            // Skip empty lines completely
            if trimmed.is_empty() {
                continue;
            }

            // Handle inline comments - split line at --
            let (code_part, _comment_part) =
                trimmed.find("--").map_or((trimmed, ""), |comment_pos| {
                    let code = trimmed[..comment_pos].trim();
                    let comment = trimmed[comment_pos..].trim();
                    (code, comment)
                });

            // Skip lines that are pure comments
            if code_part.is_empty() {
                continue;
            }

            // Check if we're starting a trigger
            if code_part.to_uppercase().starts_with("CREATE TRIGGER") {
                in_trigger = true;
                trigger_depth = 0;
            }

            // Add line to current statement
            if !current_statement.is_empty() {
                current_statement.push(' ');
            }
            current_statement.push_str(code_part);

            // Count BEGIN/END depth in triggers
            if in_trigger {
                if code_part.to_uppercase().contains("BEGIN") {
                    trigger_depth += 1;
                }
                if code_part.to_uppercase().contains("END") {
                    trigger_depth -= 1;
                }
            }

            // Check if statement is complete
            if code_part.ends_with(';') {
                if in_trigger && trigger_depth > 0 {
                    // Still inside trigger block, continue to next line
                } else {
                    // Statement is complete
                    statements.push(current_statement.clone());
                    current_statement.clear();
                    in_trigger = false;
                    trigger_depth = 0;
                }
            }
        }

        // Add any remaining statement
        if !current_statement.trim().is_empty() {
            statements.push(current_statement);
        }

        statements
    }
}

/// Encryption helper trait for token operations
pub(crate) trait EncryptionHelper {
    fn encryption_key(&self) -> &[u8];
}

impl EncryptionHelper for Database {
    fn encryption_key(&self) -> &[u8] {
        &self.encryption_key
    }
}

/// Database encryption utilities
impl Database {
    /// Encrypt sensitive data using AES-256-GCM
    ///
    /// # Note
    ///
    /// This method is kept for backward compatibility. New code should use
    /// `security::TenantEncryptionManager` for per-tenant encryption.
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_data(&self, data: &str) -> Result<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Generate unique nonce
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt data
        let mut data_bytes = data.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut data_bytes)?;

        // Combine nonce and encrypted data, then base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(data_bytes);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    /// Decrypt sensitive data
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails or data is malformed
    pub fn decrypt_data(&self, encrypted_data: &str) -> Result<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decode from base64
        let combined = general_purpose::STANDARD.decode(encrypted_data)?;

        if combined.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted data: too short"));
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into()?);

        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt data
        let mut decrypted_data = encrypted_bytes.to_vec();
        let decrypted = key.open_in_place(nonce, Aad::empty(), &mut decrypted_data)?;

        String::from_utf8(decrypted.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to convert decrypted data to string: {e}"))
    }

    /// Hash sensitive data using SHA-256
    ///
    /// # Errors
    ///
    /// Returns an error if hashing fails
    pub fn hash_data(&self, data: &str) -> Result<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::digest::{digest, SHA256};

        let hash = digest(&SHA256, data.as_bytes());
        Ok(general_purpose::STANDARD.encode(hash.as_ref()))
    }
}

/// Generate a secure encryption key (32 bytes for AES-256)
#[must_use]
pub fn generate_encryption_key() -> [u8; 32] {
    use rand::Rng;
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    key
}
