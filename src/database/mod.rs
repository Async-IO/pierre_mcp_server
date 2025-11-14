// ABOUTME: Core database management with migration system for SQLite and PostgreSQL
// ABOUTME: Handles schema setup, user management, API keys, analytics, and A2A authentication
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Agent-to-Agent (A2A) authentication and usage tracking
pub mod a2a;
/// Admin token management and authorization
pub mod admin;
/// Analytics and usage statistics database operations
pub mod analytics;
/// API key management and validation
pub mod api_keys;
/// Database error types
pub mod errors;
/// User fitness configuration storage and retrieval
pub mod fitness_configurations;
/// OAuth callback notification handling
pub mod oauth_notifications;
/// User OAuth token storage and management
pub mod user_oauth_tokens;
/// User account management and authentication
pub mod users;

/// Test utilities for database operations
pub mod test_utils;

pub use a2a::{A2AUsage, A2AUsageStats};
pub use errors::{DatabaseError, DatabaseResult};

use crate::a2a::auth::A2AClient;
use crate::a2a::client::A2ASession;
use crate::a2a::protocol::{A2ATask, TaskStatus};
use crate::api_keys::{ApiKey, ApiKeyUsage, ApiKeyUsageStats};
use crate::errors::AppError;
use crate::models::{User, UserOAuthApp, UserOAuthToken};
use crate::rate_limiting::JwtUsage;
use anyhow::{Context, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use uuid::Uuid;

/// Database connection pool with encryption support
#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
    encryption_key: Vec<u8>,
}

impl Database {
    /// Create a new database connection (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database URL is invalid or malformed
    /// - Database connection fails
    /// - `SQLite` file creation fails
    /// - Migration process fails
    /// - Encryption key is invalid
    async fn new_impl(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Ensure SQLite creates the database file if it doesn't exist
        let connection_options = if database_url.starts_with("sqlite:") {
            format!("{database_url}?mode=rwc")
        } else {
            database_url.to_owned()
        };

        let pool = SqlitePool::connect(&connection_options).await?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate_impl().await?;

        Ok(db)
    }

    /// Create a new database connection (public API)
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
        Self::new_impl(database_url, encryption_key).await
    }

    /// Get a reference to the database pool for advanced operations
    #[must_use]
    pub const fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Run all database migrations (public API)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any migration fails
    /// - Database connection is lost during migration
    /// - Insufficient database permissions
    pub async fn migrate(&self) -> Result<()> {
        self.migrate_impl().await
    }

    /// Encrypt data using AES-256-GCM with AAD (public API)
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_data_with_aad(&self, data: &str, aad_context: &str) -> Result<String> {
        Self::encrypt_data_with_aad_impl(self, data, aad_context)
    }

    /// Decrypt data using AES-256-GCM with AAD (public API)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Decryption fails
    /// - Data is malformed
    /// - AAD context does not match
    pub fn decrypt_data_with_aad(&self, encrypted_data: &str, aad_context: &str) -> Result<String> {
        Self::decrypt_data_with_aad_impl(self, encrypted_data, aad_context)
    }

    /// Run all database migrations (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any migration fails
    /// - Database connection is lost during migration
    /// - Insufficient database permissions
    /// - Database connection is lost during migration
    async fn migrate_impl(&self) -> Result<()> {
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

        // UserOAuthToken tables
        self.migrate_user_oauth_tokens().await?;

        // OAuth notifications tables
        self.migrate_oauth_notifications().await?;

        // OAuth 2.0 Server tables
        self.migrate_oauth2().await?;

        // Tenant management tables
        self.migrate_tenant_management().await?;

        // Fitness configuration tables
        self.migrate_fitness_configurations().await?;

        Ok(())
    }

    /// Create tenant management tables
    ///
    /// # Errors
    ///
    /// Create OAuth 2.0 server tables for RFC 7591 client registration
    async fn migrate_oauth2(&self) -> Result<()> {
        // Create oauth2_clients table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth2_clients (
                id TEXT PRIMARY KEY,
                client_id TEXT UNIQUE NOT NULL,
                client_secret_hash TEXT NOT NULL,
                redirect_uris TEXT NOT NULL, -- JSON array
                grant_types TEXT NOT NULL,   -- JSON array
                response_types TEXT NOT NULL, -- JSON array
                client_name TEXT,
                client_uri TEXT,
                scope TEXT,
                created_at DATETIME NOT NULL,
                expires_at DATETIME
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create oauth2_auth_codes table with PKCE support
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth2_auth_codes (
                code TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                redirect_uri TEXT NOT NULL,
                scope TEXT,
                expires_at DATETIME NOT NULL,
                used BOOLEAN NOT NULL DEFAULT 0,
                state TEXT,
                code_challenge TEXT,
                code_challenge_method TEXT,
                FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indices for performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_oauth2_clients_client_id ON oauth2_clients(client_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_oauth2_auth_codes_code ON oauth2_auth_codes(code)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth2_auth_codes_expires_at ON oauth2_auth_codes(expires_at)")
            .execute(&self.pool)
            .await?;

        // Index for tenant-scoped queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth2_auth_codes_tenant_user ON oauth2_auth_codes(tenant_id, user_id)")
            .execute(&self.pool)
            .await?;

        // Create oauth2_refresh_tokens table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth2_refresh_tokens (
                token TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                scope TEXT,
                expires_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL,
                revoked BOOLEAN NOT NULL DEFAULT 0,
                FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create index for refresh token lookups
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_oauth2_refresh_tokens_token ON oauth2_refresh_tokens(token)",
        )
        .execute(&self.pool)
        .await?;

        // Index for tenant-scoped refresh token queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth2_refresh_tokens_tenant_user ON oauth2_refresh_tokens(tenant_id, user_id)")
            .execute(&self.pool)
            .await?;

        // Create index for user refresh tokens
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_oauth2_refresh_tokens_user_id ON oauth2_refresh_tokens(user_id)",
        )
        .execute(&self.pool)
        .await?;

        // Create oauth2_states table and indexes
        self.migrate_oauth2_state_table().await?;

        Ok(())
    }

    /// Create `OAuth2` state validation table for `CSRF` protection
    async fn migrate_oauth2_state_table(&self) -> Result<()> {
        // Create oauth2_states table for server-side state validation (CSRF protection)
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth2_states (
                state TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                user_id TEXT,
                tenant_id TEXT,
                redirect_uri TEXT NOT NULL,
                scope TEXT,
                code_challenge TEXT,
                code_challenge_method TEXT,
                created_at DATETIME NOT NULL,
                expires_at DATETIME NOT NULL,
                used BOOLEAN NOT NULL DEFAULT 0,
                FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create index for state lookups and expiration cleanup
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth2_states_state ON oauth2_states(state)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_oauth2_states_expires_at ON oauth2_states(expires_at)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Returns an error if:
    /// - Table creation SQL fails
    /// - Index creation fails
    /// - Database constraints cannot be applied
    /// - SQL syntax errors in migration statements
    #[allow(clippy::too_many_lines)] // Long function: Defines complete tenant management schema
    async fn migrate_tenant_management(&self) -> Result<()> {
        // Create tenants table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                domain TEXT UNIQUE,
                plan TEXT NOT NULL DEFAULT 'starter' CHECK (plan IN ('starter', 'professional', 'enterprise')),
                owner_user_id TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_oauth_credentials table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_oauth_credentials (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                provider TEXT NOT NULL CHECK (provider IN ('strava', 'fitbit')),
                client_id TEXT NOT NULL,
                client_secret_encrypted TEXT NOT NULL,
                redirect_uri TEXT NOT NULL,
                scopes TEXT NOT NULL, -- JSON array
                rate_limit_per_day INTEGER DEFAULT 1000,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, provider)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create oauth_apps table for OAuth application registration
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS oauth_apps (
                id TEXT PRIMARY KEY,
                client_id TEXT UNIQUE NOT NULL,
                client_secret_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                redirect_uris TEXT NOT NULL, -- JSON array
                scopes TEXT NOT NULL, -- JSON array
                app_type TEXT NOT NULL DEFAULT 'public' CHECK (app_type IN ('public', 'confidential')),
                owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create key_versions table for tenant encryption
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS key_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM',
                UNIQUE(tenant_id, version)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create audit_events table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                message TEXT NOT NULL,
                source TEXT NOT NULL,
                result TEXT NOT NULL,
                tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
                user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
                ip_address TEXT,
                user_agent TEXT,
                metadata TEXT, -- JSON
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create tenant_users table for role-based permissions
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tenant_users (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
                permissions TEXT, -- JSON array of specific permissions
                invited_by TEXT REFERENCES users(id) ON DELETE SET NULL,
                invited_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                joined_at DATETIME,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                UNIQUE(tenant_id, user_id)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for tenant tables
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenants_owner ON tenants(owner_user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_oauth_tenant ON tenant_oauth_credentials(tenant_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_oauth_provider ON tenant_oauth_credentials(provider)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_key_versions_tenant ON key_versions(tenant_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_key_versions_active ON key_versions(tenant_id, is_active)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_audit_events_tenant ON audit_events(tenant_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_apps_client_id ON oauth_apps(client_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_apps_owner ON oauth_apps(owner_user_id)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Tenant management tables migration completed successfully");
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
    // Long function: Defines complete admin database schema with multiple tables and indices
    #[allow(clippy::too_many_lines)]
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

        // Create system_secrets table for centralized secret management
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS system_secrets (
                secret_type TEXT PRIMARY KEY,
                secret_value TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create rsa_keypairs table for persistent JWT signing keys
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS rsa_keypairs (
                kid TEXT PRIMARY KEY,
                private_key_pem TEXT NOT NULL,
                public_key_pem TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT false,
                key_size_bits INTEGER NOT NULL
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

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_system_secrets_type ON system_secrets(secret_type)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_rsa_keypairs_active ON rsa_keypairs(is_active)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Encrypt sensitive data using AES-256-GCM
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
            return Err(AppError::internal("Invalid encrypted data: too short").into());
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

        String::from_utf8(decrypted.to_vec()).map_err(|e| {
            AppError::internal(format!("Failed to convert decrypted data to string: {e}")).into()
        })
    }

    /// Encrypt sensitive data using AES-256-GCM with Additional Authenticated Data (AAD)
    ///
    /// AAD binds the encrypted data to a specific context (tenant|user|provider|table)
    /// preventing ciphertext from being moved between contexts or users.
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    fn encrypt_data_with_aad_impl(&self, data: &str, aad_context: &str) -> Result<String> {
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

        // Encrypt data with AAD binding
        let mut data_bytes = data.as_bytes().to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        key.seal_in_place_append_tag(nonce, aad, &mut data_bytes)?;

        // Combine nonce and encrypted data, then base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(data_bytes);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    /// Decrypt sensitive data using AES-256-GCM with Additional Authenticated Data (AAD)
    ///
    /// The same AAD context used for encryption MUST be provided for successful decryption.
    /// This prevents ciphertext from being moved between contexts.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Decryption fails
    /// - Data is malformed
    /// - AAD context does not match (authentication fails)
    fn decrypt_data_with_aad_impl(
        &self,
        encrypted_data: &str,
        aad_context: &str,
    ) -> Result<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decode from base64
        let combined = general_purpose::STANDARD.decode(encrypted_data)?;

        if combined.len() < 12 {
            return Err(AppError::internal("Invalid encrypted data: too short").into());
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into()?);

        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt data with AAD verification
        let mut decrypted_data = encrypted_bytes.to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        let decrypted = key
            .open_in_place(nonce, aad, &mut decrypted_data)
            .map_err(|e| {
                AppError::internal(format!(
                    "Decryption failed (possible AAD mismatch or tampered data): {e:?}"
                ))
            })?;

        String::from_utf8(decrypted.to_vec()).map_err(|e| {
            AppError::internal(format!("Failed to convert decrypted data to string: {e}")).into()
        })
    }

    /// Get user role for a specific tenant
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_user_tenant_role(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM tenant_users WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }

    /// Create fitness configuration tables
    ///
    /// # Errors
    ///
    /// Returns an error if table creation fails or database connection is lost
    async fn migrate_fitness_configurations(&self) -> Result<()> {
        // Create fitness_configurations table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS fitness_configurations (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                user_id TEXT,
                configuration_name TEXT NOT NULL DEFAULT 'default',
                config_data TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(tenant_id, user_id, configuration_name)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for fitness configurations
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_fitness_configs_tenant ON fitness_configurations(tenant_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_fitness_configs_user ON fitness_configurations(user_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_fitness_configs_tenant_user ON fitness_configurations(tenant_id, user_id)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get fitness configuration manager
    #[must_use]
    pub fn fitness_configurations(&self) -> fitness_configurations::FitnessConfigurationManager {
        fitness_configurations::FitnessConfigurationManager::new(self.pool.clone())
        // Safe: Pool clone for database manager
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

    /// Save RSA keypair to database for persistence across restarts
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: chrono::DateTime<chrono::Utc>,
        is_active: bool,
        key_size_bits: usize,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO rsa_keypairs (kid, private_key_pem, public_key_pem, created_at, is_active, key_size_bits)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(kid) DO UPDATE SET
                private_key_pem = EXCLUDED.private_key_pem,
                public_key_pem = EXCLUDED.public_key_pem,
                is_active = EXCLUDED.is_active
            ",
        )
        .bind(kid)
        .bind(private_key_pem)
        .bind(public_key_pem)
        .bind(created_at)
        .bind(is_active)
        .bind(i64::try_from(key_size_bits).context("RSA key size exceeds maximum supported value")?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load all RSA keypairs from database
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, chrono::DateTime<chrono::Utc>, bool)>> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT kid, private_key_pem, public_key_pem, created_at, is_active FROM rsa_keypairs ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut keypairs = Vec::new();
        for row in rows {
            let kid: String = row.try_get("kid")?;
            let private_key_pem: String = row.try_get("private_key_pem")?;
            let public_key_pem: String = row.try_get("public_key_pem")?;
            let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
            let is_active: bool = row.try_get("is_active")?;

            keypairs.push((kid, private_key_pem, public_key_pem, created_at, is_active));
        }

        Ok(keypairs)
    }

    /// Update active status of RSA keypair
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()> {
        sqlx::query("UPDATE rsa_keypairs SET is_active = $1 WHERE kid = $2")
            .bind(is_active)
            .bind(kid)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a new tenant and add the owner to `tenant_users`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Tenant already exists with the same slug
    pub async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO tenants (id, name, slug, domain, plan, owner_user_id, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(tenant.id.to_string())
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&tenant.domain)
        .bind(&tenant.plan)
        .bind(tenant.owner_user_id.to_string())
        .bind(true)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .execute(&self.pool)
        .await?;

        // Add the owner as an admin of the tenant
        sqlx::query(
            r"
            INSERT INTO tenant_users (tenant_id, user_id, role, joined_at)
            VALUES (?, ?, 'owner', datetime('now'))
            ",
        )
        .bind(tenant.id.to_string())
        .bind(tenant.owner_user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get or create system secret (generates if not exists)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Unknown secret type requested
    pub async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        // Try to get existing secret
        if let Ok(secret) = self.get_system_secret(secret_type).await {
            return Ok(secret);
        }

        // Generate new secret
        let secret_value = match secret_type {
            "admin_jwt_secret" => crate::admin::jwt::AdminJwtManager::generate_jwt_secret(),
            "database_encryption_key" => {
                // Return existing key (already loaded during initialization)
                return Ok(base64::engine::general_purpose::STANDARD.encode(&self.encryption_key));
            }
            _ => {
                return Err(
                    AppError::invalid_input(format!("Unknown secret type: {secret_type}")).into(),
                )
            }
        };

        // Store in database
        sqlx::query("INSERT INTO system_secrets (secret_type, secret_value) VALUES (?, ?)")
            .bind(secret_type)
            .bind(&secret_value)
            .execute(&self.pool)
            .await?;

        Ok(secret_value)
    }

    /// Get existing system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Secret not found
    pub async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        let row = sqlx::query("SELECT secret_value FROM system_secrets WHERE secret_type = ?")
            .bind(secret_type)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("secret_value")?)
    }

    /// Update system secret (for rotation)
    ///
    /// # Errors
    ///
    /// Returns an error if database update fails
    pub async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        sqlx::query(
            "UPDATE system_secrets SET secret_value = ?, updated_at = CURRENT_TIMESTAMP WHERE secret_type = ?",
        )
        .bind(new_value)
        .bind(secret_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get tenant by ID
    ///
    /// # Errors
    ///
    /// Returns an error if tenant not found or database query fails
    pub async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        let row = sqlx::query(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.subscription_tier, tu.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id AND tu.role = 'owner'
            WHERE t.id = ? AND t.is_active = 1
            ",
        )
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(crate::models::Tenant {
                id: Uuid::parse_str(&row.try_get::<String, _>("id")?)?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                domain: row.try_get("domain")?,
                plan: row.try_get("subscription_tier")?,
                owner_user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }),
            None => Err(DatabaseError::NotFound {
                entity_type: "Tenant",
                entity_id: tenant_id.to_string(),
            }
            .into()),
        }
    }

    /// Get tenant by slug
    ///
    /// # Errors
    ///
    /// Returns an error if tenant not found or database query fails
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        let row = sqlx::query(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.subscription_tier, tu.user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id AND tu.role = 'owner'
            WHERE t.slug = ? AND t.is_active = 1
            ",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(crate::models::Tenant {
                id: Uuid::parse_str(&row.try_get::<String, _>("id")?)?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                domain: row.try_get("domain")?,
                plan: row.try_get("subscription_tier")?,
                owner_user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }),
            None => Err(DatabaseError::NotFound {
                entity_type: "Tenant",
                entity_id: slug.to_owned(),
            }
            .into()),
        }
    }

    /// List tenants for a user
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT t.id, t.name, t.slug, t.domain, t.subscription_tier,
                   owner.user_id as owner_user_id, t.created_at, t.updated_at
            FROM tenants t
            JOIN tenant_users tu ON t.id = tu.tenant_id
            JOIN tenant_users owner ON t.id = owner.tenant_id AND owner.role = 'owner'
            WHERE tu.user_id = ? AND t.is_active = 1
            ORDER BY t.created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let tenants = rows
            .into_iter()
            .map(|row| {
                Ok(crate::models::Tenant {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id")?)?,
                    name: row.try_get("name")?,
                    slug: row.try_get("slug")?,
                    domain: row.try_get("domain")?,
                    plan: row.try_get("subscription_tier")?,
                    owner_user_id: Uuid::parse_str(&row.try_get::<String, _>("owner_user_id")?)?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(tenants)
    }

    /// Get all tenants
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        let rows = sqlx::query(
            r"
            SELECT id, slug, name, domain, subscription_tier as plan, owner_user_id, created_at, updated_at
            FROM tenants
            WHERE is_active = 1
            ORDER BY created_at
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let tenants = rows
            .into_iter()
            .map(|row| {
                Ok(crate::models::Tenant {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id")?)?,
                    name: row.try_get("name")?,
                    slug: row.try_get("slug")?,
                    domain: row.try_get("domain")?,
                    plan: row.try_get("plan")?,
                    owner_user_id: Uuid::parse_str(&row.try_get::<String, _>("owner_user_id")?)?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(tenants)
    }

    /// Store tenant OAuth credentials
    ///
    /// # Errors
    ///
    /// Returns an error if encryption or database operation fails
    pub async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        // Encrypt the client secret using proper AES-256-GCM
        let encryption_manager = crate::security::TenantEncryptionManager::new(
            ring::digest::digest(
                &ring::digest::SHA256,
                format!("oauth_secret_key_{}", credentials.tenant_id).as_bytes(),
            )
            .as_ref()
            .try_into()
            .map_err(|e| {
                tracing::error!(
                    tenant_id = %credentials.tenant_id,
                    error = ?e,
                    "Failed to create encryption key from SHA256 digest"
                );
                DatabaseError::EncryptionFailed {
                    context: format!(
                        "Failed to create encryption key for tenant {}: {:?}",
                        credentials.tenant_id, e
                    ),
                }
            })?,
        );

        let encrypted_data = encryption_manager
            .encrypt_tenant_data(credentials.tenant_id, &credentials.client_secret)
            .map_err(|e| DatabaseError::EncryptionFailed {
                context: format!("Failed to encrypt OAuth secret: {e}"),
            })?;

        let encrypted_secret = encrypted_data.data.as_bytes().to_vec();
        let nonce = encrypted_data.metadata.key_version.to_le_bytes().to_vec();

        // Convert scopes to JSON array for SQLite
        let scopes_json = serde_json::to_string(&credentials.scopes)?;

        sqlx::query(
            r"
            INSERT INTO tenant_oauth_apps
                (tenant_id, provider, client_id, client_secret_encrypted, client_secret_nonce,
                 redirect_uri, scopes, rate_limit_per_day, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)
            ON CONFLICT (tenant_id, provider)
            DO UPDATE SET
                client_id = excluded.client_id,
                client_secret_encrypted = excluded.client_secret_encrypted,
                client_secret_nonce = excluded.client_secret_nonce,
                redirect_uri = excluded.redirect_uri,
                scopes = excluded.scopes,
                rate_limit_per_day = excluded.rate_limit_per_day,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(credentials.tenant_id.to_string())
        .bind(&credentials.provider)
        .bind(&credentials.client_id)
        .bind(&encrypted_secret)
        .bind(&nonce)
        .bind(&credentials.redirect_uri)
        .bind(&scopes_json)
        .bind(i64::from(credentials.rate_limit_per_day))
        .execute(&self.pool)
        .await
        .map_err(|e| DatabaseError::QueryError {
            context: format!("Failed to store OAuth credentials: {e}"),
        })?;

        Ok(())
    }

    /// Get tenant OAuth providers
    ///
    /// # Errors
    ///
    /// Returns an error if database query or decryption fails
    pub async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        let rows = sqlx::query(
            r"
            SELECT provider, client_id, client_secret_encrypted, client_secret_nonce,
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_apps
            WHERE tenant_id = ? AND is_active = 1
            ORDER BY provider
            ",
        )
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let credentials = rows
            .into_iter()
            .map(|row| {
                let provider: String = row.try_get("provider")?;
                let client_id: String = row.try_get("client_id")?;
                let encrypted_secret: Vec<u8> = row.try_get("client_secret_encrypted")?;
                let nonce: Vec<u8> = row.try_get("client_secret_nonce")?;
                let redirect_uri: String = row.try_get("redirect_uri")?;
                let scopes_json: String = row.try_get("scopes")?;
                let rate_limit: i64 = row.try_get("rate_limit_per_day")?;

                // Decrypt the client secret
                let encryption_manager = crate::security::TenantEncryptionManager::new(
                    ring::digest::digest(
                        &ring::digest::SHA256,
                        format!("oauth_secret_key_{tenant_id}").as_bytes(),
                    )
                    .as_ref()
                    .try_into()
                    .unwrap_or([0u8; 32]),
                );

                let encrypted_data = crate::security::EncryptedData {
                    data: String::from_utf8_lossy(&encrypted_secret).to_string(),
                    metadata: crate::security::EncryptionMetadata {
                        key_version: u32::from_le_bytes(
                            nonce.as_slice().try_into().unwrap_or([1, 0, 0, 0]),
                        ),
                        tenant_id: Some(tenant_id),
                        algorithm: "AES-256-GCM".to_owned(),
                        encrypted_at: chrono::Utc::now(),
                    },
                };

                let client_secret = encryption_manager
                    .decrypt_tenant_data(tenant_id, &encrypted_data)
                    .unwrap_or_else(|_| "DECRYPTION_FAILED".to_owned());

                let scopes: Vec<String> = serde_json::from_str(&scopes_json)?;

                Ok(crate::tenant::TenantOAuthCredentials {
                    tenant_id,
                    provider,
                    client_id,
                    client_secret,
                    redirect_uri,
                    scopes,
                    rate_limit_per_day: u32::try_from(rate_limit).unwrap_or(0),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(credentials)
    }

    /// Get tenant OAuth credentials for specific provider
    ///
    /// # Errors
    ///
    /// Returns an error if database query or decryption fails
    pub async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        let row = sqlx::query(
            r"
            SELECT client_id, client_secret_encrypted, client_secret_nonce,
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_apps
            WHERE tenant_id = ? AND provider = ? AND is_active = 1
            ",
        )
        .bind(tenant_id.to_string())
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let client_id: String = row.try_get("client_id")?;
                let encrypted_secret: Vec<u8> = row.try_get("client_secret_encrypted")?;
                let nonce: Vec<u8> = row.try_get("client_secret_nonce")?;
                let redirect_uri: String = row.try_get("redirect_uri")?;
                let scopes_json: String = row.try_get("scopes")?;
                let rate_limit: i64 = row.try_get("rate_limit_per_day")?;

                // Decrypt the client secret
                let encryption_manager = crate::security::TenantEncryptionManager::new(
                    ring::digest::digest(
                        &ring::digest::SHA256,
                        format!("oauth_secret_key_{tenant_id}").as_bytes(),
                    )
                    .as_ref()
                    .try_into()
                    .map_err(|e| {
                        tracing::error!(
                            tenant_id = %tenant_id,
                            provider = %provider,
                            error = ?e,
                            "Failed to create decryption key from SHA256 digest"
                        );
                        DatabaseError::DecryptionFailed {
                            context: format!(
                                "Failed to create decryption key for tenant {tenant_id}: {e:?}"
                            ),
                        }
                    })?,
                );

                let encrypted_data = crate::security::EncryptedData {
                    data: String::from_utf8_lossy(&encrypted_secret).to_string(),
                    metadata: crate::security::EncryptionMetadata {
                        key_version: u32::from_le_bytes(
                            nonce.as_slice().try_into().unwrap_or([1, 0, 0, 0]),
                        ),
                        tenant_id: Some(tenant_id),
                        algorithm: "AES-256-GCM".to_owned(),
                        encrypted_at: chrono::Utc::now(),
                    },
                };

                let client_secret = encryption_manager
                    .decrypt_tenant_data(tenant_id, &encrypted_data)
                    .map_err(|e| DatabaseError::DecryptionFailed {
                        context: format!("Failed to decrypt OAuth secret: {e}"),
                    })?;

                let scopes: Vec<String> = serde_json::from_str(&scopes_json)?;

                Ok(Some(crate::tenant::TenantOAuthCredentials {
                    tenant_id,
                    provider: provider.to_owned(),
                    client_id,
                    client_secret,
                    redirect_uri,
                    scopes,
                    rate_limit_per_day: u32::try_from(rate_limit).unwrap_or(0),
                }))
            }
            None => Ok(None),
        }
    }

    /// Save tenant-level fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;

        let result = sqlx::query(
            r"
            INSERT INTO fitness_configurations (tenant_id, user_id, configuration_name, config_data)
            VALUES (?, NULL, ?, ?)
            ON CONFLICT (tenant_id, user_id, configuration_name)
            DO UPDATE SET
                config_data = excluded.config_data,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .bind(&config_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.try_get("id")?)
    }

    /// Get tenant-level fitness configuration
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let result = sqlx::query(
            r"
            SELECT config_data FROM fitness_configurations
            WHERE tenant_id = ? AND user_id IS NULL AND configuration_name = ?
            ",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let config_json: String = row.try_get("config_data")?;
            let config: crate::config::fitness_config::FitnessConfig =
                serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// List all tenant-level fitness configuration names
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT configuration_name FROM fitness_configurations
            WHERE tenant_id = ?
            ORDER BY configuration_name
            ",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let configurations = rows
            .into_iter()
            .map(|row| row.try_get("configuration_name"))
            .collect::<Result<Vec<String>, _>>()?;

        Ok(configurations)
    }
}

// Implement HasEncryption trait for SQLite (delegates to inherent impl methods)
impl crate::database_plugins::shared::encryption::HasEncryption for Database {
    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Self::
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> Result<String> {
        // Call inherent impl directly to avoid infinite recursion
        Database::encrypt_data_with_aad_impl(self, data, aad)
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Self::
    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> Result<String> {
        // Call inherent impl directly to avoid infinite recursion
        Database::decrypt_data_with_aad_impl(self, encrypted, aad)
    }
}

// Implement DatabaseProvider trait for Database (eliminates sqlite.rs wrapper)
use async_trait::async_trait;

#[async_trait]
impl crate::database_plugins::DatabaseProvider for Database {
    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Call inherent impl directly to avoid infinite recursion
        Database::new_impl(database_url, encryption_key).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn migrate(&self) -> Result<()> {
        // Call inherent impl directly
        Database::migrate_impl(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_user(&self, user: &User) -> Result<Uuid> {
        Database::create_user(self, user).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        Database::get_user(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Database::get_user_by_email(self, email).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        Database::get_user_by_email_required(self, email).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        Database::update_last_active(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_count(&self) -> Result<i64> {
        Database::get_user_count(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_users_by_status(&self, status: &str) -> Result<Vec<User>> {
        Database::get_users_by_status(self, status).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<User>> {
        Database::get_users_by_status_cursor(self, status, params).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<User> {
        Database::update_user_status(self, user_id, new_status, admin_token_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> Result<()> {
        Database::update_user_tenant_id(self, user_id, tenant_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> Result<()> {
        Database::upsert_user_profile(self, user_id, profile_data).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<Value>> {
        Database::get_user_profile(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String> {
        Database::create_goal(self, user_id, goal_data).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<Value>> {
        Database::get_user_goals(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        Database::update_goal_progress(self, goal_id, current_value).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        Database::get_user_configuration(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
        Database::save_user_configuration(self, user_id, config_json).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String> {
        Database::store_insight(self, user_id, insight_data).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>> {
        Database::get_user_insights(self, user_id, insight_type, limit).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        Database::create_api_key(self, api_key).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> Result<Option<ApiKey>> {
        Database::get_api_key_by_prefix(self, prefix, hash).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        Database::get_user_api_keys(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        Database::update_api_key_last_used(self, api_key_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
        Database::deactivate_api_key(self, api_key_id, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>> {
        Database::get_api_key_by_id(self, api_key_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_api_keys_filtered(
        &self,
        _user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>> {
        Database::get_api_keys_filtered(
            self,
            None,
            None,
            Some(active_only),
            limit.unwrap_or(10),
            offset.unwrap_or(0),
        )
        .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        Database::cleanup_expired_api_keys(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        Database::get_expired_api_keys(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
        Database::record_api_key_usage(self, usage).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        Database::get_api_key_current_usage(self, api_key_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats> {
        Database::get_api_key_usage_stats(self, api_key_id, start_date, end_date).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        Database::record_jwt_usage(self, usage).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
        Database::get_jwt_current_usage(self, user_id).await
    }

    async fn get_request_logs(
        &self,
        _api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        _status_filter: Option<&str>,
        _tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        let analytics_logs = self
            .get_request_logs(None, start_time, end_time, 10, 0)
            .await?;

        // Convert analytics::RequestLog to dashboard_routes::RequestLog
        Ok(analytics_logs
            .into_iter()
            .map(|log| crate::dashboard_routes::RequestLog {
                id: log.id.to_string(),
                timestamp: log.timestamp,
                api_key_id: log.api_key_id.unwrap_or_default(),
                api_key_name: "Unknown".into(),
                tool_name: "Unknown".into(),
                status_code: i32::from(log.status_code),
                response_time_ms: log.response_time_ms.and_then(|ms| i32::try_from(ms).ok()),
                error_message: log.error_message,
                request_size_bytes: None,
                response_size_bytes: None,
            })
            .collect())
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        Database::get_system_stats(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        Database::create_a2a_client(self, client, client_secret, api_key_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        Database::get_a2a_client(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> Result<Option<A2AClient>> {
        Database::get_a2a_client_by_api_key_id(self, api_key_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        Database::get_a2a_client_by_name(self, name).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        Database::list_a2a_clients(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        Database::deactivate_a2a_client(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        Database::get_a2a_client_credentials(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        Database::invalidate_a2a_client_sessions(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        Database::deactivate_client_api_keys(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        Database::create_a2a_session(self, client_id, user_id, granted_scopes, expires_in_hours)
            .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        Database::get_a2a_session(self, session_token).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        Database::update_a2a_session_activity(self, session_token).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        Database::get_active_a2a_sessions(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String> {
        Database::create_a2a_task(self, client_id, session_id, task_type, input_data).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        Database::get_a2a_task(self, task_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        Database::list_a2a_tasks(self, client_id, status_filter, limit, offset).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        Database::update_a2a_task_status(self, task_id, status, result, error).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()> {
        Database::record_a2a_usage(self, usage).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        Database::get_a2a_client_current_usage(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        Database::get_a2a_usage_stats(self, client_id, start_date, end_date).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        Database::get_a2a_client_usage_history(self, client_id, days).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        Database::get_provider_last_sync(self, user_id, provider).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        Database::update_provider_last_sync(self, user_id, provider, sync_time).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        Database::get_top_tools_analysis(self, user_id, start_time, end_time).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        Database::create_admin_token(self, request, admin_jwt_secret, jwks_manager).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        Database::get_admin_token_by_id(self, token_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        Database::get_admin_token_by_prefix(self, token_prefix).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        Database::list_admin_tokens(self, include_inactive).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        Database::deactivate_admin_token(self, token_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        Database::update_admin_token_last_used(self, token_id, ip_address).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()> {
        Database::record_admin_token_usage(self, usage).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>> {
        Database::get_admin_token_usage_history(self, token_id, start_date, end_date).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<()> {
        Database::record_admin_provisioned_key(
            self,
            admin_token_id,
            api_key_id,
            user_email,
            tier,
            rate_limit_requests,
            rate_limit_period,
        )
        .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>> {
        Database::get_admin_provisioned_keys(self, admin_token_id, start_date, end_date).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: i32,
    ) -> Result<()> {
        Database::save_rsa_keypair(
            self,
            kid,
            private_key_pem,
            public_key_pem,
            created_at,
            is_active,
            key_size_bits.try_into().unwrap_or(2048),
        )
        .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>> {
        Database::load_rsa_keypairs(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()> {
        Database::update_rsa_keypair_active_status(self, kid, is_active).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        Database::create_tenant(self, tenant).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        Database::get_tenant_by_id(self, tenant_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        Database::get_tenant_by_slug(self, slug).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        Database::list_tenants_for_user(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        Database::store_tenant_oauth_credentials(self, credentials).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        Database::get_tenant_oauth_providers(self, tenant_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        Database::get_tenant_oauth_credentials(self, tenant_id, provider).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        Database::create_oauth_app(self, app).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp> {
        Database::get_oauth_app_by_client_id(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        Database::list_oauth_apps_for_user(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<()> {
        Database::store_oauth2_client(self, client).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>> {
        Database::get_oauth2_client(self, client_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Database::store_oauth2_auth_code(self, auth_code).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Database::get_oauth2_auth_code(self, code).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Database::update_oauth2_auth_code(self, auth_code).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<()> {
        Database::store_oauth2_refresh_token(self, refresh_token).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Database::get_oauth2_refresh_token(self, token).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()> {
        Database::revoke_oauth2_refresh_token(self, token).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Database::consume_auth_code(self, code, client_id, redirect_uri, now).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Database::consume_refresh_token(self, token, client_id, now).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Database::get_refresh_token_by_value(self, token).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> Result<()> {
        Database::store_authorization_code(self, code, client_id, redirect_uri, scope, user_id)
            .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode> {
        Database::get_authorization_code(self, code).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn delete_authorization_code(&self, code: &str) -> Result<()> {
        Database::delete_authorization_code(self, code).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<()> {
        Database::store_oauth2_state(self, state).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>> {
        Database::consume_oauth2_state(self, state_value, client_id, now).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        Database::store_key_version(self, version).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        Database::get_key_versions(self, tenant_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        Database::get_current_key_version(self, tenant_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()> {
        Database::update_key_version_status(self, tenant_id, version, is_active).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        Database::delete_old_key_versions(self, tenant_id, keep_count).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        Database::get_all_tenants(self).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> Result<()> {
        Database::store_audit_event(self, event).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        Database::get_audit_events(self, tenant_id, event_type, limit).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>> {
        Database::get_user_tenant_role(self, &user_id.to_string(), &tenant_id.to_string()).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        Database::get_or_create_system_secret(self, secret_type).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        Database::get_system_secret(self, secret_type).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        Database::update_system_secret(self, secret_type, new_value).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String> {
        Database::store_oauth_notification(self, user_id, provider, success, message, expires_at)
            .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Database::get_unread_oauth_notifications(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        Database::mark_oauth_notification_read(self, notification_id, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        Database::mark_all_oauth_notifications_read(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Database::get_all_oauth_notifications(self, user_id, limit).await
    }

    async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let manager = self.fitness_configurations();
        manager
            .save_tenant_config(tenant_id, configuration_name, config)
            .await
    }

    async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let manager = self.fitness_configurations();
        manager
            .save_user_config(tenant_id, user_id, configuration_name, config)
            .await
    }

    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = self.fitness_configurations();
        manager
            .get_tenant_config(tenant_id, configuration_name)
            .await
    }

    async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = self.fitness_configurations();
        manager
            .get_user_config(tenant_id, user_id, configuration_name)
            .await
    }

    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let manager = self.fitness_configurations();
        manager.list_tenant_configurations(tenant_id).await
    }

    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        let manager = self.fitness_configurations();
        manager.list_user_configurations(tenant_id, user_id).await
    }

    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        let manager = self.fitness_configurations();
        manager
            .delete_config(tenant_id, user_id, configuration_name)
            .await
    }

    // OAuth Token Management
    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> Result<()> {
        use crate::database::user_oauth_tokens::OAuthTokenData;

        let token_data = OAuthTokenData {
            id: &token.id,
            user_id: token.user_id,
            tenant_id: &token.tenant_id,
            provider: &token.provider,
            access_token: &token.access_token,
            refresh_token: token.refresh_token.as_deref(),
            token_type: &token.token_type,
            expires_at: token.expires_at,
            scope: token.scope.as_deref().unwrap_or(""),
        };

        Database::upsert_user_oauth_token(self, &token_data).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>> {
        Database::get_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_oauth_tokens(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>> {
        Database::get_user_oauth_tokens(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>> {
        Database::get_tenant_provider_tokens(self, tenant_id, provider).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        Database::delete_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        Database::delete_user_oauth_tokens(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        Database::refresh_user_oauth_token(
            self,
            user_id,
            tenant_id,
            provider,
            access_token,
            refresh_token,
            expires_at,
        )
        .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        Database::store_user_oauth_app(
            self,
            user_id,
            provider,
            client_id,
            client_secret,
            redirect_uri,
        )
        .await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>> {
        Database::get_user_oauth_app(self, user_id, provider).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>> {
        Database::list_user_oauth_apps(self, user_id).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Database::
    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        Database::remove_user_oauth_app(self, user_id, provider).await
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
