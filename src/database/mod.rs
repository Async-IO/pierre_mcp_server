// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Database Management
//!
//! This module provides database functionality for the multi-tenant Pierre MCP Server.
//! It handles user storage, token encryption, and secure data access patterns.

mod a2a;
mod analytics;
mod api_keys;
mod tokens;
mod users;

pub use a2a::*;
pub use analytics::*;

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
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Run database migrations
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

        Ok(())
    }

    /// Create admin tables
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
            r#"
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
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create admin_provisioned_keys table
        sqlx::query(
            r#"
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
            "#,
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

/// Generate a secure encryption key (32 bytes for AES-256)
pub fn generate_encryption_key() -> [u8; 32] {
    use rand::Rng;
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) async fn create_test_db() -> Result<Database> {
        // Use a simple in-memory database - each connection gets its own isolated instance
        let database_url = "sqlite::memory:";
        Database::new(database_url, vec![0u8; 32]).await
    }
}
