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
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Pool, Sqlite, SqlitePool};
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
    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Self::
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        // Call inherent impl directly to avoid infinite recursion
        Database::new_impl(database_url, encryption_key).await
    }

    #[allow(clippy::use_self)] // Must use Database:: to avoid infinite recursion with Self::
    async fn migrate(&self) -> Result<()> {
        // Call inherent impl directly
        Database::migrate_impl(self).await
    }

    async fn create_user(&self, user: &User) -> Result<Uuid> {
        Self::create_user(self, user).await
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        Self::get_user(self, user_id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Self::get_user_by_email(self, email).await
    }

    async fn get_user_by_email_required(&self, email: &str) -> Result<User> {
        Self::get_user_by_email_required(self, email).await
    }

    async fn update_last_active(&self, user_id: Uuid) -> Result<()> {
        Self::update_last_active(self, user_id).await
    }

    async fn get_user_count(&self) -> Result<i64> {
        Self::get_user_count(self).await
    }

    async fn get_users_by_status(&self, status: &str) -> Result<Vec<User>> {
        Self::get_users_by_status(self, status).await
    }

    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> Result<crate::pagination::CursorPage<User>> {
        Self::get_users_by_status_cursor(self, status, params).await
    }

    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> Result<User> {
        Self::update_user_status(self, user_id, new_status, admin_token_id).await
    }

    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> Result<()> {
        Self::update_user_tenant_id(self, user_id, tenant_id).await
    }

    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> Result<()> {
        Self::upsert_user_profile(self, user_id, profile_data).await
    }

    async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<Value>> {
        Self::get_user_profile(self, user_id).await
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> Result<String> {
        Self::create_goal(self, user_id, goal_data).await
    }

    async fn get_user_goals(&self, user_id: Uuid) -> Result<Vec<Value>> {
        Self::get_user_goals(self, user_id).await
    }

    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> Result<()> {
        Self::update_goal_progress(self, goal_id, current_value).await
    }

    async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        Self::get_user_configuration(self, user_id).await
    }

    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
        Self::save_user_configuration(self, user_id, config_json).await
    }

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> Result<String> {
        Self::store_insight(self, user_id, insight_data).await
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Value>> {
        Self::get_user_insights(self, user_id, insight_type, limit).await
    }

    async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        Self::create_api_key(self, api_key).await
    }

    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> Result<Option<ApiKey>> {
        Self::get_api_key_by_prefix(self, prefix, hash).await
    }

    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        Self::get_user_api_keys(self, user_id).await
    }

    async fn update_api_key_last_used(&self, api_key_id: &str) -> Result<()> {
        Self::update_api_key_last_used(self, api_key_id).await
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> Result<()> {
        Self::deactivate_api_key(self, api_key_id, user_id).await
    }

    async fn get_api_key_by_id(&self, api_key_id: &str) -> Result<Option<ApiKey>> {
        Self::get_api_key_by_id(self, api_key_id).await
    }

    async fn get_api_keys_filtered(
        &self,
        _user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ApiKey>> {
        Self::get_api_keys_filtered(
            self,
            None,
            None,
            Some(active_only),
            limit.unwrap_or(10),
            offset.unwrap_or(0),
        )
        .await
    }

    async fn cleanup_expired_api_keys(&self) -> Result<u64> {
        Self::cleanup_expired_api_keys(self).await
    }

    async fn get_expired_api_keys(&self) -> Result<Vec<ApiKey>> {
        Self::get_expired_api_keys(self).await
    }

    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> Result<()> {
        Self::record_api_key_usage(self, usage).await
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> Result<u32> {
        Self::get_api_key_current_usage(self, api_key_id).await
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<ApiKeyUsageStats> {
        Self::get_api_key_usage_stats(self, api_key_id, start_date, end_date).await
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> Result<()> {
        Self::record_jwt_usage(self, usage).await
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> Result<u32> {
        Self::get_jwt_current_usage(self, user_id).await
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

    async fn get_system_stats(&self) -> Result<(u64, u64)> {
        Self::get_system_stats(self).await
    }

    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> Result<String> {
        Self::create_a2a_client(self, client, client_secret, api_key_id).await
    }

    async fn get_a2a_client(&self, client_id: &str) -> Result<Option<A2AClient>> {
        Self::get_a2a_client(self, client_id).await
    }

    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> Result<Option<A2AClient>> {
        Self::get_a2a_client_by_api_key_id(self, api_key_id).await
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> Result<Option<A2AClient>> {
        Self::get_a2a_client_by_name(self, name).await
    }

    async fn list_a2a_clients(&self, user_id: &Uuid) -> Result<Vec<A2AClient>> {
        Self::list_a2a_clients(self, user_id).await
    }

    async fn deactivate_a2a_client(&self, client_id: &str) -> Result<()> {
        Self::deactivate_a2a_client(self, client_id).await
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, String)>> {
        Self::get_a2a_client_credentials(self, client_id).await
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> Result<()> {
        Self::invalidate_a2a_client_sessions(self, client_id).await
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> Result<()> {
        Self::deactivate_client_api_keys(self, client_id).await
    }

    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> Result<String> {
        Self::create_a2a_session(self, client_id, user_id, granted_scopes, expires_in_hours).await
    }

    async fn get_a2a_session(&self, session_token: &str) -> Result<Option<A2ASession>> {
        Self::get_a2a_session(self, session_token).await
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> Result<()> {
        Self::update_a2a_session_activity(self, session_token).await
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> Result<Vec<A2ASession>> {
        Self::get_active_a2a_sessions(self, client_id).await
    }

    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> Result<String> {
        Self::create_a2a_task(self, client_id, session_id, task_type, input_data).await
    }

    async fn get_a2a_task(&self, task_id: &str) -> Result<Option<A2ATask>> {
        Self::get_a2a_task(self, task_id).await
    }

    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<A2ATask>> {
        Self::list_a2a_tasks(self, client_id, status_filter, limit, offset).await
    }

    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        Self::update_a2a_task_status(self, task_id, status, result, error).await
    }

    async fn record_a2a_usage(&self, usage: &A2AUsage) -> Result<()> {
        Self::record_a2a_usage(self, usage).await
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> Result<u32> {
        Self::get_a2a_client_current_usage(self, client_id).await
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<crate::database::A2AUsageStats> {
        Self::get_a2a_usage_stats(self, client_id, start_date, end_date).await
    }

    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u32, u32)>> {
        Self::get_a2a_client_usage_history(self, client_id, days).await
    }

    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        Self::get_provider_last_sync(self, user_id, provider).await
    }

    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> Result<()> {
        Self::update_provider_last_sync(self, user_id, provider, sync_time).await
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        Self::get_top_tools_analysis(self, user_id, start_time, end_time).await
    }

    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> Result<crate::admin::models::GeneratedAdminToken> {
        Self::create_admin_token(self, request, admin_jwt_secret, jwks_manager).await
    }

    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        Self::get_admin_token_by_id(self, token_id).await
    }

    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> Result<Option<crate::admin::models::AdminToken>> {
        Self::get_admin_token_by_prefix(self, token_prefix).await
    }

    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<crate::admin::models::AdminToken>> {
        Self::list_admin_tokens(self, include_inactive).await
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> Result<()> {
        Self::deactivate_admin_token(self, token_id).await
    }

    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        Self::update_admin_token_last_used(self, token_id, ip_address).await
    }

    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> Result<()> {
        Self::record_admin_token_usage(self, usage).await
    }

    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<crate::admin::models::AdminTokenUsage>> {
        Self::get_admin_token_usage_history(self, token_id, start_date, end_date).await
    }

    async fn record_admin_provisioned_key(
        &self,
        admin_token_id: &str,
        api_key_id: &str,
        user_email: &str,
        tier: &str,
        rate_limit_requests: u32,
        rate_limit_period: &str,
    ) -> Result<()> {
        Self::record_admin_provisioned_key(
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

    async fn get_admin_provisioned_keys(
        &self,
        admin_token_id: Option<&str>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<serde_json::Value>> {
        Self::get_admin_provisioned_keys(self, admin_token_id, start_date, end_date).await
    }

    async fn save_rsa_keypair(
        &self,
        kid: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
        key_size_bits: i32,
    ) -> Result<()> {
        Self::save_rsa_keypair(
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

    async fn load_rsa_keypairs(
        &self,
    ) -> Result<Vec<(String, String, String, DateTime<Utc>, bool)>> {
        Self::load_rsa_keypairs(self).await
    }

    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> Result<()> {
        Self::update_rsa_keypair_active_status(self, kid, is_active).await
    }

    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        Self::create_tenant(self, tenant).await
    }

    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        Self::get_tenant_by_id(self, tenant_id).await
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        Self::get_tenant_by_slug(self, slug).await
    }

    async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        Self::list_tenants_for_user(self, user_id).await
    }

    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        Self::store_tenant_oauth_credentials(self, credentials).await
    }

    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        Self::get_tenant_oauth_providers(self, tenant_id).await
    }

    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        Self::get_tenant_oauth_credentials(self, tenant_id, provider).await
    }

    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        Self::create_oauth_app(self, app).await
    }

    async fn get_oauth_app_by_client_id(&self, client_id: &str) -> Result<crate::models::OAuthApp> {
        Self::get_oauth_app_by_client_id(self, client_id).await
    }

    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        Self::list_oauth_apps_for_user(self, user_id).await
    }

    async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<()> {
        Self::store_oauth2_client(self, client).await
    }

    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>> {
        Self::get_oauth2_client(self, client_id).await
    }

    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Self::store_oauth2_auth_code(self, auth_code).await
    }

    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Self::get_oauth2_auth_code(self, code).await
    }

    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Self::update_oauth2_auth_code(self, auth_code).await
    }

    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<()> {
        Self::store_oauth2_refresh_token(self, refresh_token).await
    }

    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::get_oauth2_refresh_token(self, token).await
    }

    async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()> {
        Self::revoke_oauth2_refresh_token(self, token).await
    }

    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Self::consume_auth_code(self, code, client_id, redirect_uri, now).await
    }

    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::consume_refresh_token(self, token, client_id, now).await
    }

    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::get_refresh_token_by_value(self, token).await
    }

    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> Result<()> {
        Self::store_authorization_code(self, code, client_id, redirect_uri, scope, user_id).await
    }

    async fn get_authorization_code(&self, code: &str) -> Result<crate::models::AuthorizationCode> {
        Self::get_authorization_code(self, code).await
    }

    async fn delete_authorization_code(&self, code: &str) -> Result<()> {
        Self::delete_authorization_code(self, code).await
    }

    async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<()> {
        Self::store_oauth2_state(self, state).await
    }

    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>> {
        Self::consume_oauth2_state(self, state_value, client_id, now).await
    }

    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        Self::store_key_version(self, version).await
    }

    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        Self::get_key_versions(self, tenant_id).await
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        Self::get_current_key_version(self, tenant_id).await
    }

    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> Result<()> {
        Self::update_key_version_status(self, tenant_id, version, is_active).await
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        Self::delete_old_key_versions(self, tenant_id, keep_count).await
    }

    async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        Self::get_all_tenants(self).await
    }

    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> Result<()> {
        Self::store_audit_event(self, event).await
    }

    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        Self::get_audit_events(self, tenant_id, event_type, limit).await
    }

    async fn get_user_tenant_role(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<String>> {
        Self::get_user_tenant_role(self, &user_id.to_string(), &tenant_id.to_string()).await
    }

    async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        Self::get_or_create_system_secret(self, secret_type).await
    }

    async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        Self::get_system_secret(self, secret_type).await
    }

    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> Result<()> {
        Self::update_system_secret(self, secret_type, new_value).await
    }

    async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> Result<String> {
        Self::store_oauth_notification(self, user_id, provider, success, message, expires_at).await
    }

    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Self::get_unread_oauth_notifications(self, user_id).await
    }

    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> Result<bool> {
        Self::mark_oauth_notification_read(self, notification_id, user_id).await
    }

    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> Result<u64> {
        Self::mark_all_oauth_notifications_read(self, user_id).await
    }

    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Self::get_all_oauth_notifications(self, user_id, limit).await
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

        Self::upsert_user_oauth_token(self, &token_data).await
    }

    async fn get_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Option<UserOAuthToken>> {
        Self::get_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    async fn get_user_oauth_tokens(&self, user_id: Uuid) -> Result<Vec<UserOAuthToken>> {
        Self::get_user_oauth_tokens(self, user_id).await
    }

    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> Result<Vec<UserOAuthToken>> {
        Self::get_tenant_provider_tokens(self, tenant_id, provider).await
    }

    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> Result<()> {
        Self::delete_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> Result<()> {
        Self::delete_user_oauth_tokens(self, user_id).await
    }

    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        Self::refresh_user_oauth_token(
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

    async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        Self::store_user_oauth_app(
            self,
            user_id,
            provider,
            client_id,
            client_secret,
            redirect_uri,
        )
        .await
    }

    async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>> {
        Self::get_user_oauth_app(self, user_id, provider).await
    }

    async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>> {
        Self::list_user_oauth_apps(self, user_id).await
    }

    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        Self::remove_user_oauth_app(self, user_id, provider).await
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
