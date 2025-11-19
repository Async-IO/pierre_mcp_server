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
/// Repository pattern implementation for database abstraction
pub mod repositories;
/// User OAuth token storage and management
pub mod user_oauth_tokens;
/// User account management and authentication
pub mod users;

/// Test utilities for database operations
pub mod test_utils;

pub use a2a::{A2AUsage, A2AUsageStats};
pub use errors::{DatabaseError, DatabaseResult};

use crate::errors::AppError;
use crate::models::UserOAuthApp;
use anyhow::{Context, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
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

    // ================================
    // Repository Pattern Accessors
    // ================================

    /// Get `UserRepository` for user account management
    #[must_use]
    pub fn users(&self) -> repositories::UserRepositoryImpl {
        repositories::UserRepositoryImpl::new(crate::database_plugins::factory::Database::SQLite(
            self.clone(),
        ))
    }

    /// Get `OAuthTokenRepository` for OAuth token storage
    #[must_use]
    pub fn oauth_tokens(&self) -> repositories::OAuthTokenRepositoryImpl {
        repositories::OAuthTokenRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
    }

    /// Get `ApiKeyRepository` for API key management
    #[must_use]
    pub fn api_keys(&self) -> repositories::ApiKeyRepositoryImpl {
        repositories::ApiKeyRepositoryImpl::new(crate::database_plugins::factory::Database::SQLite(
            self.clone(),
        ))
    }

    /// Get `UsageRepository` for usage tracking and analytics
    #[must_use]
    pub fn usage(&self) -> repositories::UsageRepositoryImpl {
        repositories::UsageRepositoryImpl::new(crate::database_plugins::factory::Database::SQLite(
            self.clone(),
        ))
    }

    /// Get `A2ARepository` for Agent-to-Agent management
    #[must_use]
    pub fn a2a(&self) -> repositories::A2ARepositoryImpl {
        repositories::A2ARepositoryImpl::new(crate::database_plugins::factory::Database::SQLite(
            self.clone(),
        ))
    }

    /// Get `ProfileRepository` for user profiles and goals
    #[must_use]
    pub fn profiles(&self) -> repositories::ProfileRepositoryImpl {
        repositories::ProfileRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
    }

    /// Get `InsightRepository` for AI-generated insights
    #[must_use]
    pub fn insights(&self) -> repositories::InsightRepositoryImpl {
        repositories::InsightRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
    }

    /// Get `AdminRepository` for admin token management
    #[must_use]
    pub fn admin(&self) -> repositories::AdminRepositoryImpl {
        repositories::AdminRepositoryImpl::new(crate::database_plugins::factory::Database::SQLite(
            self.clone(),
        ))
    }

    /// Get `TenantRepository` for multi-tenant management
    #[must_use]
    pub fn tenants(&self) -> repositories::TenantRepositoryImpl {
        repositories::TenantRepositoryImpl::new(crate::database_plugins::factory::Database::SQLite(
            self.clone(),
        ))
    }

    /// Get `OAuth2ServerRepository` for OAuth 2.0 server functionality
    #[must_use]
    pub fn oauth2_server(&self) -> repositories::OAuth2ServerRepositoryImpl {
        repositories::OAuth2ServerRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
    }

    /// Get `SecurityRepository` for key rotation and audit
    #[must_use]
    pub fn security(&self) -> repositories::SecurityRepositoryImpl {
        repositories::SecurityRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
    }

    /// Get `NotificationRepository` for OAuth notifications
    #[must_use]
    pub fn notifications(&self) -> repositories::NotificationRepositoryImpl {
        repositories::NotificationRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
    }

    /// Get `FitnessConfigRepository` for fitness configuration management
    #[must_use]
    pub fn fitness_configs(&self) -> repositories::FitnessConfigRepositoryImpl {
        repositories::FitnessConfigRepositoryImpl::new(
            crate::database_plugins::factory::Database::SQLite(self.clone()),
        )
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

        // Create user_configurations table for storing user configuration data
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_configurations (
                user_id TEXT PRIMARY KEY,
                config_json TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
    pub async fn create_tenant_impl(&self, tenant: &crate::models::Tenant) -> Result<()> {
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
    pub async fn get_or_create_system_secret_impl(&self, secret_type: &str) -> Result<String> {
        // Try to get existing secret
        if let Ok(secret) = self.get_system_secret_impl(secret_type).await {
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
    pub async fn get_system_secret_impl(&self, secret_type: &str) -> Result<String> {
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
    pub async fn update_system_secret_impl(
        &self,
        secret_type: &str,
        new_value: &str,
    ) -> Result<()> {
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
    pub async fn get_tenant_by_id_impl(&self, tenant_id: Uuid) -> Result<crate::models::Tenant> {
        let row = sqlx::query(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.plan, tu.user_id, t.created_at, t.updated_at
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
                plan: row.try_get("plan")?,
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
    pub async fn get_tenant_by_slug_impl(&self, slug: &str) -> Result<crate::models::Tenant> {
        let row = sqlx::query(
            r"
            SELECT t.id, t.name, t.slug, t.domain, t.plan, tu.user_id, t.created_at, t.updated_at
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
                plan: row.try_get("plan")?,
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
    pub async fn list_tenants_for_user_impl(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Tenant>> {
        let rows = sqlx::query(
            r"
            SELECT DISTINCT t.id, t.name, t.slug, t.domain, t.plan,
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
                    plan: row.try_get("plan")?,
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
    pub async fn get_all_tenants_impl(&self) -> Result<Vec<crate::models::Tenant>> {
        let rows = sqlx::query(
            r"
            SELECT id, slug, name, domain, plan, owner_user_id, created_at, updated_at
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
    pub async fn store_tenant_oauth_credentials_impl(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        // Encrypt the client secret using AES-256-GCM with AAD binding
        // AAD context format: "{tenant_id}|{provider}|tenant_oauth_credentials"
        let aad_context = format!(
            "{}|{}|tenant_oauth_credentials",
            credentials.tenant_id, credentials.provider
        );
        let encrypted_secret =
            self.encrypt_data_with_aad(&credentials.client_secret, &aad_context)?;

        // Convert scopes to JSON array for SQLite
        let scopes_json = serde_json::to_string(&credentials.scopes)?;

        sqlx::query(
            r"
            INSERT INTO tenant_oauth_credentials
                (tenant_id, provider, client_id, client_secret_encrypted,
                 redirect_uri, scopes, rate_limit_per_day, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1)
            ON CONFLICT (tenant_id, provider)
            DO UPDATE SET
                client_id = excluded.client_id,
                client_secret_encrypted = excluded.client_secret_encrypted,
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
    pub async fn get_tenant_oauth_providers_impl(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        let rows = sqlx::query(
            r"
            SELECT provider, client_id, client_secret_encrypted,
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_credentials
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
                let encrypted_secret: String = row.try_get("client_secret_encrypted")?;
                let redirect_uri: String = row.try_get("redirect_uri")?;
                let scopes_json: String = row.try_get("scopes")?;
                let rate_limit: i64 = row.try_get("rate_limit_per_day")?;

                // Decrypt the client secret using AAD binding
                // AAD context format: "{tenant_id}|{provider}|tenant_oauth_credentials"
                let aad_context = format!("{tenant_id}|{provider}|tenant_oauth_credentials");
                let client_secret = self.decrypt_data_with_aad(&encrypted_secret, &aad_context)?;

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
    pub async fn get_tenant_oauth_credentials_impl(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        let row = sqlx::query(
            r"
            SELECT client_id, client_secret_encrypted,
                   redirect_uri, scopes, rate_limit_per_day
            FROM tenant_oauth_credentials
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
                let encrypted_secret: String = row.try_get("client_secret_encrypted")?;
                let redirect_uri: String = row.try_get("redirect_uri")?;
                let scopes_json: String = row.try_get("scopes")?;
                let rate_limit: i64 = row.try_get("rate_limit_per_day")?;

                // Decrypt the client secret using AAD binding
                // AAD context format: "{tenant_id}|{provider}|tenant_oauth_credentials"
                let aad_context = format!("{tenant_id}|{provider}|tenant_oauth_credentials");
                let client_secret = self.decrypt_data_with_aad(&encrypted_secret, &aad_context)?;

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

    // ================================
    // OAuth2 Server (SQLite implementations)
    // ================================

    /// Store `OAuth2` client (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn store_oauth2_client_impl(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO oauth2_clients (id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "
        )
        .bind(&client.id)
        .bind(&client.client_id)
        .bind(&client.client_secret_hash)
        .bind(serde_json::to_string(&client.redirect_uris)?)
        .bind(serde_json::to_string(&client.grant_types)?)
        .bind(serde_json::to_string(&client.response_types)?)
        .bind(&client.client_name)
        .bind(&client.client_uri)
        .bind(&client.scope)
        .bind(client.created_at)
        .bind(client.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get `OAuth2` client (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_oauth2_client_impl(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>> {
        let row = sqlx::query(
            r"
            SELECT id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at
            FROM oauth2_clients
            WHERE client_id = ?1
            "
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2Client {
                id: row.try_get("id")?,
                client_id: row.try_get("client_id")?,
                client_secret_hash: row.try_get("client_secret_hash")?,
                redirect_uris: serde_json::from_str(&row.try_get::<String, _>("redirect_uris")?)?,
                grant_types: serde_json::from_str(&row.try_get::<String, _>("grant_types")?)?,
                response_types: serde_json::from_str(&row.try_get::<String, _>("response_types")?)?,
                client_name: row.try_get("client_name")?,
                client_uri: row.try_get("client_uri")?,
                scope: row.try_get("scope")?,
                created_at: row.try_get("created_at")?,
                expires_at: row.try_get("expires_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Store `OAuth2` auth code (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn store_oauth2_auth_code_impl(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO oauth2_auth_codes (code, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at, used, state)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "
        )
        .bind(&auth_code.code)
        .bind(&auth_code.client_id)
        .bind(auth_code.user_id.to_string())
        .bind(&auth_code.tenant_id)
        .bind(&auth_code.redirect_uri)
        .bind(&auth_code.scope)
        .bind(&auth_code.code_challenge)
        .bind(&auth_code.code_challenge_method)
        .bind(auth_code.expires_at)
        .bind(auth_code.used)
        .bind(&auth_code.state)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get `OAuth2` auth code (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_oauth2_auth_code_impl(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        let row = sqlx::query(
            r"
            SELECT code, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at, used, state
            FROM oauth2_auth_codes
            WHERE code = ?1
            "
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2AuthCode {
                code: row.try_get("code")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                tenant_id: row.try_get("tenant_id")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scope: row.try_get("scope")?,
                code_challenge: row.try_get("code_challenge")?,
                code_challenge_method: row.try_get("code_challenge_method")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get("used")?,
                state: row.try_get("state")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update `OAuth2` auth code (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn update_oauth2_auth_code_impl(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE oauth2_auth_codes
            SET used = ?1
            WHERE code = ?2
            ",
        )
        .bind(auth_code.used)
        .bind(&auth_code.code)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store `OAuth2` refresh token (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn store_oauth2_refresh_token_impl(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO oauth2_refresh_tokens (token, client_id, user_id, tenant_id, scope, created_at, expires_at, revoked)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "
        )
        .bind(&refresh_token.token)
        .bind(&refresh_token.client_id)
        .bind(refresh_token.user_id.to_string())
        .bind(&refresh_token.tenant_id)
        .bind(&refresh_token.scope)
        .bind(refresh_token.created_at)
        .bind(refresh_token.expires_at)
        .bind(refresh_token.revoked)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get `OAuth2` refresh token (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_oauth2_refresh_token_impl(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        let row = sqlx::query(
            r"
            SELECT token, client_id, user_id, tenant_id, scope, created_at, expires_at, revoked
            FROM oauth2_refresh_tokens
            WHERE token = ?1
            ",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                tenant_id: row.try_get("tenant_id")?,
                scope: row.try_get("scope")?,
                created_at: row.try_get("created_at")?,
                expires_at: row.try_get("expires_at")?,
                revoked: row.try_get("revoked")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Atomically consume `OAuth2` auth code (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn consume_auth_code_impl(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        let row = sqlx::query(
            r"
            UPDATE oauth2_auth_codes
            SET used = 1
            WHERE code = ?1
              AND client_id = ?2
              AND redirect_uri = ?3
              AND used = 0
              AND expires_at > ?4
            RETURNING code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method
            "
        )
        .bind(code)
        .bind(client_id)
        .bind(redirect_uri)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2AuthCode {
                code: row.try_get("code")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                tenant_id: row.try_get("tenant_id")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scope: row.try_get("scope")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get("used")?,
                state: row.try_get("state")?,
                code_challenge: row.try_get("code_challenge")?,
                code_challenge_method: row.try_get("code_challenge_method")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Atomically consume `OAuth2` refresh token (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn consume_refresh_token_impl(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        let row = sqlx::query(
            r"
            UPDATE oauth2_refresh_tokens
            SET revoked = 1
            WHERE token = ?1
              AND client_id = ?2
              AND revoked = 0
              AND expires_at > ?3
            RETURNING token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
            ",
        )
        .bind(token)
        .bind(client_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?)?,
                tenant_id: row.try_get("tenant_id")?,
                scope: row.try_get("scope")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                revoked: row.try_get("revoked")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Store `OAuth2` state (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn store_oauth2_state_impl(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO oauth2_states (state, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, created_at, expires_at, used)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "
        )
        .bind(&state.state)
        .bind(&state.client_id)
        .bind(state.user_id)
        .bind(&state.tenant_id)
        .bind(&state.redirect_uri)
        .bind(&state.scope)
        .bind(&state.code_challenge)
        .bind(&state.code_challenge_method)
        .bind(state.created_at)
        .bind(state.expires_at)
        .bind(state.used)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Atomically consume `OAuth2` state (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn consume_oauth2_state_impl(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>> {
        let row = sqlx::query(
            r"
            UPDATE oauth2_states
            SET used = 1
            WHERE state = ?1
              AND client_id = ?2
              AND used = 0
              AND expires_at > ?3
            RETURNING state, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, created_at, expires_at, used
            "
        )
        .bind(state_value)
        .bind(client_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2State {
                state: row.try_get("state")?,
                client_id: row.try_get("client_id")?,
                user_id: row.try_get("user_id")?,
                tenant_id: row.try_get("tenant_id")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scope: row.try_get("scope")?,
                code_challenge: row.try_get("code_challenge")?,
                code_challenge_method: row.try_get("code_challenge_method")?,
                created_at: row.try_get("created_at")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get("used")?,
            }))
        } else {
            Ok(None)
        }
    }

    // ================================
    // Public Wrapper Methods (delegates to impl methods or repositories)
    // ================================

    /// Get user configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues occur
    pub async fn get_user_configuration(&self, user_id: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT config_json FROM user_configurations WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_get("config_json").ok()))
    }

    /// Save user configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert or update fails
    /// - Database connection issues occur
    pub async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO user_configurations (user_id, config_json, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET
                config_json = excluded.config_json,
                updated_at = CURRENT_TIMESTAMP
            ",
        )
        .bind(user_id)
        .bind(config_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get top tools analysis
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization from usage repository fails
    /// - Database connection issues occur
    pub async fn get_top_tools_analysis(
        &self,
        _user_id: Uuid,
        _start_time: chrono::DateTime<chrono::Utc>,
        _end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::dashboard_routes::ToolUsage>> {
        // NOTE: This method is not actually used. Dashboard routes has its own
        // implementation that aggregates data from get_api_key_usage_stats.
        // Returning empty vector to avoid circular delegation through UsageRepository.
        tokio::task::yield_now().await;
        Ok(Vec::new())
    }

    /// Get request logs with filters (avoids circular delegation)
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_request_logs_with_filters(
        &self,
        api_key_id: Option<&str>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        _status_filter: Option<&str>,
        _tool_filter: Option<&str>,
    ) -> Result<Vec<crate::dashboard_routes::RequestLog>> {
        let mut query = "SELECT id, user_id, api_key_id, timestamp, method, endpoint, status_code, response_time_ms, error_message FROM request_logs WHERE 1=1".to_owned();

        if api_key_id.is_some() {
            query.push_str(" AND api_key_id = ?");
        }
        if start_time.is_some() {
            query.push_str(" AND timestamp >= ?");
        }
        if end_time.is_some() {
            query.push_str(" AND timestamp <= ?");
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT 100");

        let mut sql_query = sqlx::query(&query);
        if let Some(key_id) = api_key_id {
            sql_query = sql_query.bind(key_id);
        }
        if let Some(start) = start_time {
            sql_query = sql_query.bind(start.to_rfc3339());
        }
        if let Some(end) = end_time {
            sql_query = sql_query.bind(end.to_rfc3339());
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut logs = Vec::new();
        for row in rows {
            let timestamp_str: String = row.try_get("timestamp")?;
            let api_key_id_opt: Option<String> = row.try_get("api_key_id")?;

            logs.push(crate::dashboard_routes::RequestLog {
                id: row.try_get::<i64, _>("id")?.to_string(),
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc),
                api_key_id: api_key_id_opt.unwrap_or_default(),
                api_key_name: String::new(),
                tool_name: row.try_get("endpoint")?,
                status_code: row.try_get::<i32, _>("status_code")?,
                response_time_ms: row.try_get("response_time_ms")?,
                error_message: row.try_get("error_message")?,
                request_size_bytes: None,
                response_size_bytes: None,
            });
        }

        Ok(logs)
    }

    /// Create tenant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Tenant already exists with the same slug
    /// - Database connection issues occur
    pub async fn create_tenant(&self, tenant: &crate::models::Tenant) -> Result<()> {
        Self::create_tenant_impl(self, tenant).await
    }

    /// Get tenant by ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant not found with the given ID
    /// - Database query execution fails
    /// - Database connection issues occur
    pub async fn get_tenant_by_id(&self, id: Uuid) -> Result<crate::models::Tenant> {
        Self::get_tenant_by_id_impl(self, id).await
    }

    /// Get tenant by slug
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant not found with the given slug
    /// - Database query execution fails
    /// - Database connection issues occur
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<crate::models::Tenant> {
        Self::get_tenant_by_slug_impl(self, slug).await
    }

    /// List tenants for user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn list_tenants_for_user(&self, user_id: Uuid) -> Result<Vec<crate::models::Tenant>> {
        Self::list_tenants_for_user_impl(self, user_id).await
    }

    /// Get all tenants
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_all_tenants(&self) -> Result<Vec<crate::models::Tenant>> {
        Self::get_all_tenants_impl(self).await
    }

    /// Store tenant OAuth credentials
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Encryption of client secret fails
    /// - Database insert or update fails
    /// - Database connection issues occur
    pub async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> Result<()> {
        Self::store_tenant_oauth_credentials_impl(self, credentials).await
    }

    /// Get tenant OAuth providers
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Decryption of client secret fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::tenant::TenantOAuthCredentials>> {
        Self::get_tenant_oauth_providers_impl(self, tenant_id).await
    }

    /// Get tenant OAuth credentials
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Decryption of client secret fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::tenant::TenantOAuthCredentials>> {
        Self::get_tenant_oauth_credentials_impl(self, tenant_id, provider).await
    }

    /// Create OAuth app
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - OAuth app already exists with the same client ID
    /// - Database connection issues occur
    pub async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO oauth_apps (id, client_id, client_secret_hash, name, description, redirect_uris, scopes, app_type, owner_user_id, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(app.id.to_string())
        .bind(&app.client_id)
        .bind(&app.client_secret)
        .bind(&app.name)
        .bind(&app.description)
        .bind(serde_json::to_string(&app.redirect_uris)?)
        .bind(serde_json::to_string(&app.scopes)?)
        .bind(&app.app_type)
        .bind(app.owner_user_id.to_string())
        .bind(true) // is_active
        .bind(app.created_at)
        .bind(app.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get OAuth app by client ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - OAuth app not found with the given client ID
    /// - Database query execution fails
    /// - Database connection issues occur
    pub async fn get_oauth_app_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<crate::models::OAuthApp> {
        let row = sqlx::query(
            r"
            SELECT id, client_id, client_secret_hash, name, description, redirect_uris, scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps
            WHERE client_id = ?
            ",
        )
        .bind(client_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::OAuthApp {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?)?,
            client_id: row.try_get("client_id")?,
            client_secret: row.try_get("client_secret_hash")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            redirect_uris: serde_json::from_str(&row.try_get::<String, _>("redirect_uris")?)?,
            scopes: serde_json::from_str(&row.try_get::<String, _>("scopes")?)?,
            app_type: row.try_get("app_type")?,
            owner_user_id: Uuid::parse_str(&row.try_get::<String, _>("owner_user_id")?)?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// List OAuth apps for user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::OAuthApp>> {
        let rows = sqlx::query(
            r"
            SELECT id, client_id, client_secret_hash, name, description, redirect_uris, scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps
            WHERE owner_user_id = ?
            ORDER BY created_at DESC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut apps = Vec::new();
        for row in rows {
            apps.push(crate::models::OAuthApp {
                id: Uuid::parse_str(&row.try_get::<String, _>("id")?)?,
                client_id: row.try_get("client_id")?,
                client_secret: row.try_get("client_secret_hash")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                redirect_uris: serde_json::from_str(&row.try_get::<String, _>("redirect_uris")?)?,
                scopes: serde_json::from_str(&row.try_get::<String, _>("scopes")?)?,
                app_type: row.try_get("app_type")?,
                owner_user_id: Uuid::parse_str(&row.try_get::<String, _>("owner_user_id")?)?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(apps)
    }

    /// Store user OAuth app
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert or update fails
    /// - Encryption of OAuth credentials fails
    /// - Database connection issues occur
    pub async fn store_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        // Encrypt the client secret
        let aad_context = format!("user_oauth_app:{user_id}:{provider}");
        let encrypted_secret = self.encrypt_data_with_aad(client_secret, &aad_context)?;

        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO user_oauth_app_credentials (id, user_id, provider, client_id, client_secret, redirect_uri)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, provider)
             DO UPDATE SET client_id = excluded.client_id, client_secret = excluded.client_secret,
                          redirect_uri = excluded.redirect_uri, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(provider)
        .bind(client_id)
        .bind(&encrypted_secret)
        .bind(redirect_uri)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user OAuth app
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Decryption of OAuth credentials fails
    /// - Database connection issues occur
    pub async fn get_user_oauth_app(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserOAuthApp>> {
        let row = sqlx::query(
            "SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
             FROM user_oauth_app_credentials
             WHERE user_id = ? AND provider = ?",
        )
        .bind(user_id.to_string())
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id: String = row.try_get("id")?;
            let user_id_str: String = row.try_get("user_id")?;
            let provider: String = row.try_get("provider")?;
            let client_id: String = row.try_get("client_id")?;
            let encrypted_secret: String = row.try_get("client_secret")?;
            let redirect_uri: String = row.try_get("redirect_uri")?;
            let created_at: String = row.try_get("created_at")?;
            let updated_at: String = row.try_get("updated_at")?;

            // Decrypt the client secret
            let aad_context = format!("user_oauth_app:{user_id_str}:{provider}");
            let client_secret = self.decrypt_data_with_aad(&encrypted_secret, &aad_context)?;

            Ok(Some(UserOAuthApp {
                id,
                user_id: Uuid::parse_str(&user_id_str)?,
                provider,
                client_id,
                client_secret,
                redirect_uri,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    /// List user OAuth apps
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Decryption of OAuth credentials fails
    /// - Database connection issues occur
    pub async fn list_user_oauth_apps(&self, user_id: Uuid) -> Result<Vec<UserOAuthApp>> {
        let rows = sqlx::query(
            "SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
             FROM user_oauth_app_credentials
             WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut apps = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let user_id_str: String = row.try_get("user_id")?;
            let provider: String = row.try_get("provider")?;
            let client_id: String = row.try_get("client_id")?;
            let encrypted_secret: String = row.try_get("client_secret")?;
            let redirect_uri: String = row.try_get("redirect_uri")?;
            let created_at: String = row.try_get("created_at")?;
            let updated_at: String = row.try_get("updated_at")?;

            // Decrypt the client secret
            let aad_context = format!("user_oauth_app:{user_id_str}:{provider}");
            let client_secret = self.decrypt_data_with_aad(&encrypted_secret, &aad_context)?;

            apps.push(UserOAuthApp {
                id,
                user_id: Uuid::parse_str(&user_id_str)?,
                provider,
                client_id,
                client_secret,
                redirect_uri,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
            });
        }

        Ok(apps)
    }

    /// Remove user OAuth app
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database delete operation fails
    /// - Database connection issues occur
    pub async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_oauth_app_credentials WHERE user_id = ? AND provider = ?")
            .bind(user_id.to_string())
            .bind(provider)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store `OAuth2` client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Client ID already exists
    /// - Data serialization fails
    /// - Database connection issues occur
    pub async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> Result<()> {
        Self::store_oauth2_client_impl(self, client).await
    }

    /// Get `OAuth2` client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2Client>> {
        Self::get_oauth2_client_impl(self, client_id).await
    }

    /// Store `OAuth2` authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Authorization code already exists
    /// - Database connection issues occur
    pub async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Self::store_oauth2_auth_code_impl(self, auth_code).await
    }

    /// Get `OAuth2` authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Self::get_oauth2_auth_code_impl(self, code).await
    }

    /// Update `OAuth2` authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Authorization code not found
    /// - Database connection issues occur
    pub async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        Self::update_oauth2_auth_code_impl(self, auth_code).await
    }

    /// Store `OAuth2` refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Refresh token already exists
    /// - Database connection issues occur
    pub async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> Result<()> {
        Self::store_oauth2_refresh_token_impl(self, refresh_token).await
    }

    /// Get `OAuth2` refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::get_oauth2_refresh_token_impl(self, token).await
    }

    /// Revoke `OAuth2` refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Refresh token not found
    /// - Database connection issues occur
    pub async fn revoke_oauth2_refresh_token(&self, token: &str) -> Result<()> {
        sqlx::query("UPDATE oauth2_refresh_tokens SET revoked = 1 WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get refresh token by value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        let row = sqlx::query(
            "SELECT token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
             FROM oauth2_refresh_tokens WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let user_id_str: String = row.try_get("user_id")?;
            let expires_at_str: String = row.try_get("expires_at")?;
            let created_at_str: String = row.try_get("created_at")?;
            let revoked: bool = row.try_get("revoked")?;

            Ok(Some(crate::oauth2_server::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&user_id_str)?,
                tenant_id: row.try_get("tenant_id")?,
                scope: row.try_get("scope")?,
                expires_at: DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                revoked,
            }))
        } else {
            Ok(None)
        }
    }

    /// Consume auth code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Authorization code validation fails
    /// - Database connection issues occur
    pub async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        // First fetch the auth code
        let row = sqlx::query(
            "SELECT code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method
             FROM oauth2_auth_codes WHERE code = ? AND client_id = ? AND redirect_uri = ?",
        )
        .bind(code)
        .bind(client_id)
        .bind(redirect_uri)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let used: bool = row.try_get("used")?;
            let expires_at_str: String = row.try_get("expires_at")?;
            let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc);

            // Validate: not used and not expired
            if used || expires_at < now {
                return Ok(None);
            }

            // Mark as used
            sqlx::query("UPDATE oauth2_auth_codes SET used = 1 WHERE code = ?")
                .bind(code)
                .execute(&self.pool)
                .await?;

            let user_id_str: String = row.try_get("user_id")?;

            Ok(Some(crate::oauth2_server::models::OAuth2AuthCode {
                code: row.try_get("code")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&user_id_str)?,
                tenant_id: row.try_get("tenant_id")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scope: row.try_get("scope")?,
                expires_at,
                used: true, // Now marked as used
                state: row.try_get("state")?,
                code_challenge: row.try_get("code_challenge")?,
                code_challenge_method: row.try_get("code_challenge_method")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Consume refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Refresh token validation fails
    /// - Database connection issues occur
    pub async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        // First fetch the refresh token
        let row = sqlx::query(
            "SELECT token, client_id, user_id, tenant_id, scope, expires_at, created_at, revoked
             FROM oauth2_refresh_tokens WHERE token = ? AND client_id = ?",
        )
        .bind(token)
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let revoked: bool = row.try_get("revoked")?;
            let expires_at_str: String = row.try_get("expires_at")?;
            let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc);

            // Validate: not revoked and not expired
            if revoked || expires_at < now {
                return Ok(None);
            }

            // Mark as revoked (consumed)
            sqlx::query("UPDATE oauth2_refresh_tokens SET revoked = 1 WHERE token = ?")
                .bind(token)
                .execute(&self.pool)
                .await?;

            let user_id_str: String = row.try_get("user_id")?;
            let created_at_str: String = row.try_get("created_at")?;

            Ok(Some(crate::oauth2_server::models::OAuth2RefreshToken {
                token: row.try_get("token")?,
                client_id: row.try_get("client_id")?,
                user_id: Uuid::parse_str(&user_id_str)?,
                tenant_id: row.try_get("tenant_id")?,
                scope: row.try_get("scope")?,
                expires_at,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                revoked: true, // Now marked as revoked
            }))
        } else {
            Ok(None)
        }
    }

    /// Store authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Authorization code already exists
    /// - Database connection issues occur
    pub async fn store_authorization_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth2_auth_codes (code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&auth_code.code)
        .bind(&auth_code.client_id)
        .bind(auth_code.user_id.to_string())
        .bind(&auth_code.tenant_id)
        .bind(&auth_code.redirect_uri)
        .bind(&auth_code.scope)
        .bind(auth_code.expires_at.to_rfc3339())
        .bind(auth_code.used)
        .bind(&auth_code.state)
        .bind(&auth_code.code_challenge)
        .bind(&auth_code.code_challenge_method)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authorization code not found
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<crate::oauth2_server::models::OAuth2AuthCode> {
        let row = sqlx::query(
            "SELECT code, client_id, user_id, tenant_id, redirect_uri, scope, expires_at, used, state, code_challenge, code_challenge_method
             FROM oauth2_auth_codes WHERE code = ? AND client_id = ? AND redirect_uri = ?",
        )
        .bind(code)
        .bind(client_id)
        .bind(redirect_uri)
        .fetch_one(&self.pool)
        .await?;

        let user_id_str: String = row.try_get("user_id")?;
        let expires_at_str: String = row.try_get("expires_at")?;

        Ok(crate::oauth2_server::models::OAuth2AuthCode {
            code: row.try_get("code")?,
            client_id: row.try_get("client_id")?,
            user_id: Uuid::parse_str(&user_id_str)?,
            tenant_id: row.try_get("tenant_id")?,
            redirect_uri: row.try_get("redirect_uri")?,
            scope: row.try_get("scope")?,
            expires_at: DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc),
            used: row.try_get("used")?,
            state: row.try_get("state")?,
            code_challenge: row.try_get("code_challenge")?,
            code_challenge_method: row.try_get("code_challenge_method")?,
        })
    }

    /// Delete authorization code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database delete operation fails
    /// - Database connection issues occur
    pub async fn delete_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM oauth2_auth_codes WHERE code = ? AND client_id = ? AND redirect_uri = ?",
        )
        .bind(code)
        .bind(client_id)
        .bind(redirect_uri)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Store `OAuth2` state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - State value already exists
    /// - Database connection issues occur
    pub async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> Result<()> {
        Self::store_oauth2_state_impl(self, state).await
    }

    /// Consume `OAuth2` state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - State validation fails
    /// - Database connection issues occur
    pub async fn consume_oauth2_state(
        &self,
        state: &str,
        client_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<crate::oauth2_server::models::OAuth2State>> {
        Self::consume_oauth2_state_impl(self, state, client_id, now).await
    }

    /// Store audit event
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Event serialization fails
    /// - Database connection issues occur
    pub async fn store_audit_event(
        &self,
        tenant_id: Option<Uuid>,
        event: &crate::security::audit::AuditEvent,
    ) -> Result<()> {
        let event_type_json = serde_json::to_string(&event.event_type)?;
        let severity_json = serde_json::to_string(&event.severity)?;
        let metadata_json = serde_json::to_string(&event.metadata)?;

        sqlx::query(
            "INSERT INTO audit_events (id, event_type, severity, message, source, result, tenant_id, user_id, ip_address, user_agent, metadata, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.event_id.to_string())
        .bind(&event_type_json)
        .bind(&severity_json)
        .bind(&event.description)
        .bind(&event.action)
        .bind(&event.result)
        .bind(tenant_id.map(|id| id.to_string()))
        .bind(event.user_id.map(|id| id.to_string()))
        .bind(&event.source_ip)
        .bind(&event.user_agent)
        .bind(&metadata_json)
        .bind(event.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get audit events
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Event deserialization fails
    /// - Database connection issues occur
    pub async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<crate::security::audit::AuditEvent>> {
        let mut query = "SELECT id, event_type, severity, message, source, result, tenant_id, user_id, ip_address, user_agent, metadata, timestamp FROM audit_events WHERE 1=1".to_owned();

        if tenant_id.is_some() {
            query.push_str(" AND tenant_id = ?");
        }
        if event_type.is_some() {
            query.push_str(" AND event_type = ?");
        }
        query.push_str(" ORDER BY timestamp DESC");
        if let Some(lim) = limit {
            use std::fmt::Write;
            let _ = write!(query, " LIMIT {lim}");
        }

        let mut sql_query = sqlx::query(&query);
        if let Some(tid) = tenant_id {
            sql_query = sql_query.bind(tid.to_string());
        }
        if let Some(et) = event_type {
            sql_query = sql_query.bind(et);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut events = Vec::new();
        for row in rows {
            let event_id_str: String = row.try_get("id")?;
            let event_type_json: String = row.try_get("event_type")?;
            let severity_json: String = row.try_get("severity")?;
            let metadata_json: String = row.try_get("metadata")?;
            let timestamp_str: String = row.try_get("timestamp")?;

            let user_id_opt: Option<String> = row.try_get("user_id")?;
            let tenant_id_opt: Option<String> = row.try_get("tenant_id")?;

            events.push(crate::security::audit::AuditEvent {
                event_id: Uuid::parse_str(&event_id_str)?,
                event_type: serde_json::from_str(&event_type_json)?,
                severity: serde_json::from_str(&severity_json)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc),
                user_id: user_id_opt.and_then(|s| Uuid::parse_str(&s).ok()),
                tenant_id: tenant_id_opt.and_then(|s| Uuid::parse_str(&s).ok()),
                source_ip: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                session_id: None, // Not stored in current schema
                description: row.try_get("message")?,
                metadata: serde_json::from_str(&metadata_json)?,
                resource: None, // Not stored in current schema
                action: row.try_get("source")?,
                result: row.try_get("result")?,
            });
        }

        Ok(events)
    }

    /// Get or create system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Unknown secret type requested
    /// - Database connection issues occur
    pub async fn get_or_create_system_secret(&self, secret_type: &str) -> Result<String> {
        Self::get_or_create_system_secret_impl(self, secret_type).await
    }

    /// Get system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Secret not found
    /// - Database connection issues occur
    pub async fn get_system_secret(&self, secret_type: &str) -> Result<String> {
        Self::get_system_secret_impl(self, secret_type).await
    }

    /// Update system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Database connection issues occur
    pub async fn update_system_secret(&self, secret_type: &str, secret_value: &str) -> Result<()> {
        Self::update_system_secret_impl(self, secret_type, secret_value).await
    }

    /// Store key version
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Key version already exists
    /// - Database connection issues occur
    pub async fn store_key_version(
        &self,
        tenant_id: Option<Uuid>,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO key_versions (tenant_id, version, created_at, expires_at, is_active, algorithm)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id.map(|id| id.to_string()))
        .bind(version.version)
        .bind(version.created_at.to_rfc3339())
        .bind(version.expires_at.to_rfc3339())
        .bind(version.is_active)
        .bind(&version.algorithm)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get key versions
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<crate::security::key_rotation::KeyVersion>> {
        let rows = if let Some(tid) = tenant_id {
            sqlx::query(
                "SELECT version, created_at, expires_at, is_active, tenant_id, algorithm
                 FROM key_versions WHERE tenant_id = ? ORDER BY version DESC",
            )
            .bind(tid.to_string())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT version, created_at, expires_at, is_active, tenant_id, algorithm
                 FROM key_versions WHERE tenant_id IS NULL ORDER BY version DESC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        let mut versions = Vec::new();
        for row in rows {
            let version_num: u32 = row.try_get::<i64, _>("version")?.try_into()?;
            let created_at_str: String = row.try_get("created_at")?;
            let expires_at_str: String = row.try_get("expires_at")?;
            let tenant_id_opt: Option<String> = row.try_get("tenant_id")?;

            versions.push(crate::security::key_rotation::KeyVersion {
                version: version_num,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                expires_at: DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc),
                is_active: row.try_get("is_active")?,
                tenant_id: tenant_id_opt.and_then(|s| Uuid::parse_str(&s).ok()),
                algorithm: row.try_get("algorithm")?,
            });
        }

        Ok(versions)
    }

    /// Get current key version
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Data deserialization fails
    /// - Database connection issues occur
    pub async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<crate::security::key_rotation::KeyVersion>> {
        let row = if let Some(tid) = tenant_id {
            sqlx::query(
                "SELECT version, created_at, expires_at, is_active, tenant_id, algorithm
                 FROM key_versions WHERE tenant_id = ? AND is_active = 1 ORDER BY version DESC LIMIT 1",
            )
            .bind(tid.to_string())
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT version, created_at, expires_at, is_active, tenant_id, algorithm
                 FROM key_versions WHERE tenant_id IS NULL AND is_active = 1 ORDER BY version DESC LIMIT 1",
            )
            .fetch_optional(&self.pool)
            .await?
        };

        if let Some(row) = row {
            let version_num: u32 = row.try_get::<i64, _>("version")?.try_into()?;
            let created_at_str: String = row.try_get("created_at")?;
            let expires_at_str: String = row.try_get("expires_at")?;
            let tenant_id_opt: Option<String> = row.try_get("tenant_id")?;

            Ok(Some(crate::security::key_rotation::KeyVersion {
                version: version_num,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
                expires_at: DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc),
                is_active: row.try_get("is_active")?,
                tenant_id: tenant_id_opt.and_then(|s| Uuid::parse_str(&s).ok()),
                algorithm: row.try_get("algorithm")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update key version status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database update fails
    /// - Key version not found
    /// - Database connection issues occur
    pub async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version_id: u32,
        is_active: bool,
    ) -> Result<()> {
        if let Some(tid) = tenant_id {
            sqlx::query(
                "UPDATE key_versions SET is_active = ? WHERE tenant_id = ? AND version = ?",
            )
            .bind(is_active)
            .bind(tid.to_string())
            .bind(version_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE key_versions SET is_active = ? WHERE tenant_id IS NULL AND version = ?",
            )
            .bind(is_active)
            .bind(version_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Delete old key versions
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database delete operation fails
    /// - Database connection issues occur
    pub async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> Result<u64> {
        // Delete all but the most recent keep_count versions
        let result = if let Some(tid) = tenant_id {
            sqlx::query(
                "DELETE FROM key_versions WHERE tenant_id = ? AND id NOT IN (
                   SELECT id FROM key_versions WHERE tenant_id = ? ORDER BY version DESC LIMIT ?
                 )",
            )
            .bind(tid.to_string())
            .bind(tid.to_string())
            .bind(keep_count)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                "DELETE FROM key_versions WHERE tenant_id IS NULL AND id NOT IN (
                   SELECT id FROM key_versions WHERE tenant_id IS NULL ORDER BY version DESC LIMIT ?
                 )",
            )
            .bind(keep_count)
            .execute(&self.pool)
            .await?
        };

        Ok(result.rows_affected())
    }

    /// Save tenant fitness config
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert or update fails
    /// - Configuration serialization fails
    /// - Database connection issues occur
    pub async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO fitness_configurations (id, tenant_id, user_id, configuration_name, config_data)
             VALUES (?, ?, NULL, ?, ?)
             ON CONFLICT(tenant_id, user_id, configuration_name)
             DO UPDATE SET config_data = excluded.config_data, updated_at = datetime('now')",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(configuration_name)
        .bind(&config_json)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get tenant fitness config
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Configuration deserialization fails
    /// - Database connection issues occur
    pub async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let row = sqlx::query(
            "SELECT config_data FROM fitness_configurations
             WHERE tenant_id = ? AND user_id IS NULL AND configuration_name = ?",
        )
        .bind(tenant_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let config_json: String = row.try_get("config_data")?;
            let config: crate::config::fitness_config::FitnessConfig =
                serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// List tenant fitness configurations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues occur
    pub async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT configuration_name FROM fitness_configurations
             WHERE tenant_id = ? AND user_id IS NULL",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let names: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("configuration_name").ok())
            .collect();

        Ok(names)
    }

    /// Save user fitness config
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert or update fails
    /// - Configuration serialization fails
    /// - Database connection issues occur
    pub async fn save_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> Result<String> {
        let config_json = serde_json::to_string(config)?;
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO fitness_configurations (id, tenant_id, user_id, configuration_name, config_data)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(tenant_id, user_id, configuration_name)
             DO UPDATE SET config_data = excluded.config_data, updated_at = datetime('now')",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(user_id)
        .bind(configuration_name)
        .bind(&config_json)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get user fitness config
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Configuration deserialization fails
    /// - Database connection issues occur
    pub async fn get_user_fitness_config(
        &self,
        tenant_id: &str,
        user_id: &str,
        configuration_name: &str,
    ) -> Result<Option<crate::config::fitness_config::FitnessConfig>> {
        let row = sqlx::query(
            "SELECT config_data FROM fitness_configurations
             WHERE tenant_id = ? AND user_id = ? AND configuration_name = ?",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(configuration_name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let config_json: String = row.try_get("config_data")?;
            let config: crate::config::fitness_config::FitnessConfig =
                serde_json::from_str(&config_json)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// List user fitness configurations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query execution fails
    /// - Database connection issues occur
    pub async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT configuration_name FROM fitness_configurations
             WHERE tenant_id = ? AND user_id = ?",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let names: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("configuration_name").ok())
            .collect();

        Ok(names)
    }

    /// Delete fitness config
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database delete operation fails
    /// - Database connection issues occur
    pub async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> Result<bool> {
        let result = if let Some(uid) = user_id {
            sqlx::query(
                "DELETE FROM fitness_configurations
                 WHERE tenant_id = ? AND user_id = ? AND configuration_name = ?",
            )
            .bind(tenant_id)
            .bind(uid)
            .bind(configuration_name)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                "DELETE FROM fitness_configurations
                 WHERE tenant_id = ? AND user_id IS NULL AND configuration_name = ?",
            )
            .bind(tenant_id)
            .bind(configuration_name)
            .execute(&self.pool)
            .await?
        };

        Ok(result.rows_affected() > 0)
    }
}

// Implement HasEncryption trait for SQLite (delegates to inherent impl methods)
impl crate::database_plugins::shared::encryption::HasEncryption for Database {
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> Result<String> {
        // Call inherent impl directly to avoid infinite recursion
        Self::encrypt_data_with_aad_impl(self, data, aad)
    }

    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> Result<String> {
        // Call inherent impl directly to avoid infinite recursion
        Self::decrypt_data_with_aad_impl(self, encrypted, aad)
    }
}

// Implement DatabaseProvider trait for Database (eliminates sqlite.rs wrapper)
/// Generate a secure encryption key (32 bytes for AES-256)
#[must_use]
pub fn generate_encryption_key() -> [u8; 32] {
    use rand::Rng;
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    key
}
