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
use crate::errors::{AppError, AppResult};
use crate::models::{User, UserOAuthApp, UserOAuthToken};
use crate::rate_limiting::JwtUsage;
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
    async fn new_impl(database_url: &str, encryption_key: Vec<u8>) -> AppResult<Self> {
        // Ensure SQLite creates the database file if it doesn't exist
        let connection_options = if database_url.starts_with("sqlite:") {
            format!("{database_url}?mode=rwc")
        } else {
            database_url.to_owned()
        };

        let pool = SqlitePool::connect(&connection_options)
            .await
            .map_err(|e| AppError::database(format!("Failed to connect to database: {e}")))?;

        let db = Self {
            pool,
            encryption_key,
        };

        // Run migrations
        db.migrate_impl()
            .await
            .map_err(|e| AppError::database(format!("Database migration failed: {e}")))?;

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
    pub async fn new(database_url: &str, encryption_key: Vec<u8>) -> AppResult<Self> {
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
    pub async fn migrate(&self) -> AppResult<()> {
        self.migrate_impl().await
    }

    /// Encrypt data using AES-256-GCM with AAD (public API)
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_data_with_aad(&self, data: &str, aad_context: &str) -> AppResult<String> {
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
    pub fn decrypt_data_with_aad(
        &self,
        encrypted_data: &str,
        aad_context: &str,
    ) -> AppResult<String> {
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
    async fn migrate_impl(&self) -> AppResult<()> {
        tracing::info!("Running database migrations...");

        // Run all pending migrations embedded at compile-time from ./migrations directory
        // Using compile-time macro which embeds migrations into the binary
        // This ensures migrations are available regardless of working directory
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Migration failed: {e}")))?;

        tracing::info!("Database migrations completed successfully");
        Ok(())
    }

    /// Encrypt sensitive data using AES-256-GCM
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_data(&self, data: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Generate unique nonce
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|e| AppError::internal(format!("Failed to generate nonce: {e}")))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| AppError::internal(format!("Failed to create encryption key: {e}")))?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt data
        let mut data_bytes = data.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut data_bytes)
            .map_err(|e| AppError::internal(format!("Failed to encrypt data: {e}")))?;

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
    pub fn decrypt_data(&self, encrypted_data: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decode from base64
        let combined = general_purpose::STANDARD
            .decode(encrypted_data)
            .map_err(|e| AppError::internal(format!("Failed to decode base64: {e}")))?;

        if combined.len() < 12 {
            return Err(AppError::internal("Invalid encrypted data: too short"));
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes
                .try_into()
                .map_err(|e| AppError::internal(format!("Invalid nonce size: {e}")))?,
        );

        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| AppError::internal(format!("Failed to create decryption key: {e}")))?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt data
        let mut decrypted_data = encrypted_bytes.to_vec();
        let decrypted = key
            .open_in_place(nonce, Aad::empty(), &mut decrypted_data)
            .map_err(|e| AppError::internal(format!("Failed to decrypt data: {e}")))?;

        String::from_utf8(decrypted.to_vec()).map_err(|e| {
            AppError::internal(format!("Failed to convert decrypted data to string: {e}"))
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
    fn encrypt_data_with_aad_impl(&self, data: &str, aad_context: &str) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Generate unique nonce
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|e| AppError::internal(format!("Failed to generate nonce: {e}")))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| AppError::internal(format!("Failed to create encryption key: {e}")))?;
        let key = LessSafeKey::new(unbound_key);

        // Encrypt data with AAD binding
        let mut data_bytes = data.as_bytes().to_vec();
        let aad = Aad::from(aad_context.as_bytes());
        key.seal_in_place_append_tag(nonce, aad, &mut data_bytes)
            .map_err(|e| AppError::internal(format!("Failed to encrypt data: {e}")))?;

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
    ) -> AppResult<String> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decode from base64
        let combined = general_purpose::STANDARD
            .decode(encrypted_data)
            .map_err(|e| AppError::internal(format!("Failed to decode base64: {e}")))?;

        if combined.len() < 12 {
            return Err(AppError::internal("Invalid encrypted data: too short"));
        }

        // Extract nonce and encrypted data
        let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes
                .try_into()
                .map_err(|e| AppError::internal(format!("Invalid nonce size: {e}")))?,
        );

        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| AppError::internal(format!("Failed to create decryption key: {e}")))?;
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
            AppError::internal(format!("Failed to convert decrypted data to string: {e}"))
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
    ) -> AppResult<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM tenant_users WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        Ok(row.map(|r| r.0))
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
    pub fn hash_data(&self, data: &str) -> AppResult<String> {
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
    ) -> AppResult<()> {
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
        .bind(i64::try_from(key_size_bits).map_err(|e| AppError::invalid_input(format!("RSA key size exceeds maximum supported value: {e}")))?)
        .execute(&self.pool)

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Load all RSA keypairs from database
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn load_rsa_keypairs(
        &self,
    ) -> AppResult<Vec<(String, String, String, chrono::DateTime<chrono::Utc>, bool)>> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT kid, private_key_pem, public_key_pem, created_at, is_active FROM rsa_keypairs ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)

            .await

            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        let mut keypairs = Vec::new();
        for row in rows {
            let kid: String = row
                .try_get("kid")
                .map_err(|e| AppError::database(format!("Failed to get kid: {e}")))?;
            let private_key_pem: String = row
                .try_get("private_key_pem")
                .map_err(|e| AppError::database(format!("Failed to get private_key_pem: {e}")))?;
            let public_key_pem: String = row
                .try_get("public_key_pem")
                .map_err(|e| AppError::database(format!("Failed to get public_key_pem: {e}")))?;
            let created_at: chrono::DateTime<chrono::Utc> = row
                .try_get("created_at")
                .map_err(|e| AppError::database(format!("Failed to get created_at: {e}")))?;
            let is_active: bool = row
                .try_get("is_active")
                .map_err(|e| AppError::database(format!("Failed to get is_active: {e}")))?;

            keypairs.push((kid, private_key_pem, public_key_pem, created_at, is_active));
        }

        Ok(keypairs)
    }

    /// Update active status of RSA keypair
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails
    pub async fn update_rsa_keypair_active_status(
        &self,
        kid: &str,
        is_active: bool,
    ) -> AppResult<()> {
        sqlx::query("UPDATE rsa_keypairs SET is_active = $1 WHERE kid = $2")
            .bind(is_active)
            .bind(kid)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Create a new tenant and add the owner to `tenant_users`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database insert fails
    /// - Tenant already exists with the same slug
    pub async fn create_tenant_impl(&self, tenant: &crate::models::Tenant) -> AppResult<()> {
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

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        // Add the owner as an admin of the tenant
        let tenant_user_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r"
            INSERT INTO tenant_users (id, tenant_id, user_id, role, invited_at, joined_at)
            VALUES (?, ?, ?, 'owner', ?, ?)
            ",
        )
        .bind(&tenant_user_id)
        .bind(tenant.id.to_string())
        .bind(tenant.owner_user_id.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Get or create system secret (generates if not exists)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Unknown secret type requested
    pub async fn get_or_create_system_secret_impl(&self, secret_type: &str) -> AppResult<String> {
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
                return Err(AppError::invalid_input(format!(
                    "Unknown secret type: {secret_type}"
                )))
            }
        };

        // Store in database
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO system_secrets (secret_type, secret_value, created_at, updated_at) VALUES (?, ?, ?, ?)")
            .bind(secret_type)
            .bind(&secret_value)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(secret_value)
    }

    /// Get existing system secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - Secret not found
    pub async fn get_system_secret_impl(&self, secret_type: &str) -> AppResult<String> {
        let row = sqlx::query("SELECT secret_value FROM system_secrets WHERE secret_type = ?")
            .bind(secret_type)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        row.try_get("secret_value")
            .map_err(|e| AppError::database(format!("Failed to get secret_value: {e}")))
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
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE system_secrets SET secret_value = ?, updated_at = CURRENT_TIMESTAMP WHERE secret_type = ?",
        )
        .bind(new_value)
        .bind(secret_type)
        .execute(&self.pool)

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Get tenant by ID
    ///
    /// # Errors
    ///
    /// Returns an error if tenant not found or database query fails
    pub async fn get_tenant_by_id_impl(&self, tenant_id: Uuid) -> AppResult<crate::models::Tenant> {
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
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        match row {
            Some(row) => {
                let id_str: String = row
                    .try_get("id")
                    .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
                let user_id_str: String = row
                    .try_get("user_id")
                    .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?;

                Ok(crate::models::Tenant {
                    id: Uuid::parse_str(&id_str)?,
                    name: row
                        .try_get("name")
                        .map_err(|e| AppError::database(format!("Failed to get name: {e}")))?,
                    slug: row
                        .try_get("slug")
                        .map_err(|e| AppError::database(format!("Failed to get slug: {e}")))?,
                    domain: row
                        .try_get("domain")
                        .map_err(|e| AppError::database(format!("Failed to get domain: {e}")))?,
                    plan: row
                        .try_get("plan")
                        .map_err(|e| AppError::database(format!("Failed to get plan: {e}")))?,
                    owner_user_id: Uuid::parse_str(&user_id_str)?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AppError::database(format!("Failed to get created_at: {e}"))
                    })?,
                    updated_at: row.try_get("updated_at").map_err(|e| {
                        AppError::database(format!("Failed to get updated_at: {e}"))
                    })?,
                })
            }
            None => Err(AppError::not_found("Tenant")),
        }
    }

    /// Get tenant by slug
    ///
    /// # Errors
    ///
    /// Returns an error if tenant not found or database query fails
    pub async fn get_tenant_by_slug_impl(&self, slug: &str) -> AppResult<crate::models::Tenant> {
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
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        match row {
            Some(row) => {
                let id_str: String = row
                    .try_get("id")
                    .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
                let user_id_str: String = row
                    .try_get("user_id")
                    .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?;

                Ok(crate::models::Tenant {
                    id: Uuid::parse_str(&id_str)?,
                    name: row
                        .try_get("name")
                        .map_err(|e| AppError::database(format!("Failed to get name: {e}")))?,
                    slug: row
                        .try_get("slug")
                        .map_err(|e| AppError::database(format!("Failed to get slug: {e}")))?,
                    domain: row
                        .try_get("domain")
                        .map_err(|e| AppError::database(format!("Failed to get domain: {e}")))?,
                    plan: row
                        .try_get("plan")
                        .map_err(|e| AppError::database(format!("Failed to get plan: {e}")))?,
                    owner_user_id: Uuid::parse_str(&user_id_str)?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AppError::database(format!("Failed to get created_at: {e}"))
                    })?,
                    updated_at: row.try_get("updated_at").map_err(|e| {
                        AppError::database(format!("Failed to get updated_at: {e}"))
                    })?,
                })
            }
            None => Err(AppError::not_found("Tenant")),
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
    ) -> AppResult<Vec<crate::models::Tenant>> {
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
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        let tenants = rows
            .into_iter()
            .map(|row| {
                let id_str: String = row
                    .try_get("id")
                    .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
                let owner_user_id_str: String = row
                    .try_get("owner_user_id")
                    .map_err(|e| AppError::database(format!("Failed to get owner_user_id: {e}")))?;

                Ok(crate::models::Tenant {
                    id: Uuid::parse_str(&id_str)?,
                    name: row
                        .try_get("name")
                        .map_err(|e| AppError::database(format!("Failed to get name: {e}")))?,
                    slug: row
                        .try_get("slug")
                        .map_err(|e| AppError::database(format!("Failed to get slug: {e}")))?,
                    domain: row
                        .try_get("domain")
                        .map_err(|e| AppError::database(format!("Failed to get domain: {e}")))?,
                    plan: row
                        .try_get("plan")
                        .map_err(|e| AppError::database(format!("Failed to get plan: {e}")))?,
                    owner_user_id: Uuid::parse_str(&owner_user_id_str)?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AppError::database(format!("Failed to get created_at: {e}"))
                    })?,
                    updated_at: row.try_get("updated_at").map_err(|e| {
                        AppError::database(format!("Failed to get updated_at: {e}"))
                    })?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok(tenants)
    }

    /// Get all tenants
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_all_tenants_impl(&self) -> AppResult<Vec<crate::models::Tenant>> {
        let rows = sqlx::query(
            r"
            SELECT id, slug, name, domain, plan, owner_user_id, created_at, updated_at
            FROM tenants
            WHERE is_active = 1
            ORDER BY created_at
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        let tenants = rows
            .into_iter()
            .map(|row| {
                let id_str: String = row
                    .try_get("id")
                    .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?;
                let owner_user_id_str: String = row
                    .try_get("owner_user_id")
                    .map_err(|e| AppError::database(format!("Failed to get owner_user_id: {e}")))?;

                Ok(crate::models::Tenant {
                    id: Uuid::parse_str(&id_str)?,
                    name: row
                        .try_get("name")
                        .map_err(|e| AppError::database(format!("Failed to get name: {e}")))?,
                    slug: row
                        .try_get("slug")
                        .map_err(|e| AppError::database(format!("Failed to get slug: {e}")))?,
                    domain: row
                        .try_get("domain")
                        .map_err(|e| AppError::database(format!("Failed to get domain: {e}")))?,
                    plan: row
                        .try_get("plan")
                        .map_err(|e| AppError::database(format!("Failed to get plan: {e}")))?,
                    owner_user_id: Uuid::parse_str(&owner_user_id_str)?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AppError::database(format!("Failed to get created_at: {e}"))
                    })?,
                    updated_at: row.try_get("updated_at").map_err(|e| {
                        AppError::database(format!("Failed to get updated_at: {e}"))
                    })?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

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
    ) -> AppResult<()> {
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
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO tenant_oauth_credentials
                (tenant_id, provider, client_id, client_secret_encrypted,
                 redirect_uri, scopes, rate_limit_per_day, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
            ON CONFLICT (tenant_id, provider)
            DO UPDATE SET
                client_id = excluded.client_id,
                client_secret_encrypted = excluded.client_secret_encrypted,
                redirect_uri = excluded.redirect_uri,
                scopes = excluded.scopes,
                rate_limit_per_day = excluded.rate_limit_per_day,
                updated_at = excluded.updated_at
            ",
        )
        .bind(credentials.tenant_id.to_string())
        .bind(&credentials.provider)
        .bind(&credentials.client_id)
        .bind(&encrypted_secret)
        .bind(&credentials.redirect_uri)
        .bind(&scopes_json)
        .bind(i64::from(credentials.rate_limit_per_day))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to store OAuth credentials: {e}")))?;

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
    ) -> AppResult<Vec<crate::tenant::TenantOAuthCredentials>> {
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
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        let credentials = rows
            .into_iter()
            .map(|row| {
                let provider: String = row
                    .try_get("provider")
                    .map_err(|e| AppError::database(format!("Failed to get provider: {e}")))?;
                let client_id: String = row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?;
                let encrypted_secret: String =
                    row.try_get("client_secret_encrypted").map_err(|e| {
                        AppError::database(format!("Failed to get client_secret_encrypted: {e}"))
                    })?;
                let redirect_uri: String = row
                    .try_get("redirect_uri")
                    .map_err(|e| AppError::database(format!("Failed to get redirect_uri: {e}")))?;
                let scopes_json: String = row
                    .try_get("scopes")
                    .map_err(|e| AppError::database(format!("Failed to get scopes: {e}")))?;
                let rate_limit: i64 = row.try_get("rate_limit_per_day").map_err(|e| {
                    AppError::database(format!("Failed to get rate_limit_per_day: {e}"))
                })?;

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
            .collect::<AppResult<Vec<_>>>()?;

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
    ) -> AppResult<Option<crate::tenant::TenantOAuthCredentials>> {
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
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        match row {
            Some(row) => {
                let client_id: String = row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?;
                let encrypted_secret: String =
                    row.try_get("client_secret_encrypted").map_err(|e| {
                        AppError::database(format!("Failed to get client_secret_encrypted: {e}"))
                    })?;
                let redirect_uri: String = row
                    .try_get("redirect_uri")
                    .map_err(|e| AppError::database(format!("Failed to get redirect_uri: {e}")))?;
                let scopes_json: String = row
                    .try_get("scopes")
                    .map_err(|e| AppError::database(format!("Failed to get scopes: {e}")))?;
                let rate_limit: i64 = row.try_get("rate_limit_per_day").map_err(|e| {
                    AppError::database(format!("Failed to get rate_limit_per_day: {e}"))
                })?;

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
    // User Configuration (SQLite implementations)
    // ================================

    /// Get user configuration data (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn get_user_configuration_impl(&self, user_id: &str) -> AppResult<Option<String>> {
        // First ensure the user_configurations table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_configurations (
                user_id TEXT PRIMARY KEY,
                config_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        let query = "SELECT config_data FROM user_configurations WHERE user_id = ?1";

        let row = sqlx::query(query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            Ok(Some(row.try_get("config_data").map_err(|e| {
                AppError::database(format!("Failed to get config_data: {e}"))
            })?))
        } else {
            Ok(None)
        }
    }

    /// Save user configuration data (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn save_user_configuration_impl(
        &self,
        user_id: &str,
        config_json: &str,
    ) -> AppResult<()> {
        // First ensure the user_configurations table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_configurations (
                user_id TEXT PRIMARY KEY,
                config_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        // Insert or update configuration
        let now = chrono::Utc::now().to_rfc3339();
        let query = r"
            INSERT INTO user_configurations (user_id, config_data, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?3)
            ON CONFLICT(user_id) DO UPDATE SET
                config_data = EXCLUDED.config_data,
                updated_at = ?3
        ";

        sqlx::query(query)
            .bind(user_id)
            .bind(config_json)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    // ================================
    // RSA Keypair Management (SQLite implementations)
    // ================================

    /// Update RSA keypair active status (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn update_rsa_keypair_active_status_impl(
        &self,
        kid: &str,
        is_active: bool,
    ) -> AppResult<()> {
        sqlx::query("UPDATE rsa_keypairs SET is_active = ?1 WHERE kid = ?2")
            .bind(is_active)
            .bind(kid)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    // ================================
    // OAuth App Management (SQLite implementations)
    // ================================

    /// Create OAuth app (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn create_oauth_app_impl(&self, app: &crate::models::OAuthApp) -> AppResult<()> {
        let redirect_uris_json = serde_json::to_string(&app.redirect_uris)?;
        let scopes_json = serde_json::to_string(&app.scopes)?;

        sqlx::query(
            r"
            INSERT INTO oauth_apps
                (id, client_id, client_secret_hash, name, description, redirect_uris,
                 scopes, app_type, owner_user_id, is_active, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11)
            ",
        )
        .bind(app.id.to_string())
        .bind(&app.client_id)
        .bind(&app.client_secret)
        .bind(&app.name)
        .bind(&app.description)
        .bind(&redirect_uris_json)
        .bind(&scopes_json)
        .bind(&app.app_type)
        .bind(app.owner_user_id.to_string())
        .bind(app.created_at)
        .bind(app.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to create OAuth app: {e}")))?;

        Ok(())
    }

    /// Get OAuth app by client ID (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or app not found
    async fn get_oauth_app_by_client_id_impl(
        &self,
        client_id: &str,
    ) -> AppResult<crate::models::OAuthApp> {
        let row = sqlx::query(
            r"
            SELECT id, client_id, client_secret_hash, name, description, redirect_uris,
                   scopes, app_type, owner_user_id, created_at, updated_at
            FROM oauth_apps
            WHERE client_id = ?1 AND is_active = 1
            ",
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        match row {
            Some(row) => {
                let redirect_uris_json: String = row.get("redirect_uris");
                let scopes_json: String = row.get("scopes");

                Ok(crate::models::OAuthApp {
                    id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                    client_id: row.get("client_id"),
                    client_secret: row.get("client_secret_hash"),
                    name: row.get("name"),
                    description: row.get("description"),
                    redirect_uris: serde_json::from_str(&redirect_uris_json)?,
                    scopes: serde_json::from_str(&scopes_json)?,
                    app_type: row.get("app_type"),
                    owner_user_id: Uuid::parse_str(&row.get::<String, _>("owner_user_id"))?,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            }
            None => Err(AppError::not_found("OAuth app")),
        }
    }

    // ================================
    // OAuth2 Server (SQLite implementations)
    // ================================

    /// Revoke `OAuth2` refresh token (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn revoke_oauth2_refresh_token_impl(&self, token: &str) -> AppResult<()> {
        sqlx::query("UPDATE oauth2_refresh_tokens SET revoked = 1 WHERE token = ?1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    /// Store `OAuth2` client (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn store_oauth2_client_impl(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> AppResult<()> {
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

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2Client>> {
        let row = sqlx::query(
            r"
            SELECT id, client_id, client_secret_hash, redirect_uris, grant_types, response_types, client_name, client_uri, scope, created_at, expires_at
            FROM oauth2_clients
            WHERE client_id = ?1
            "
        )
        .bind(client_id)
        .fetch_optional(&self.pool)

            .await

            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            let redirect_uris_str: String = row
                .try_get("redirect_uris")
                .map_err(|e| AppError::database(format!("Failed to get redirect_uris: {e}")))?;
            let grant_types_str: String = row
                .try_get("grant_types")
                .map_err(|e| AppError::database(format!("Failed to get grant_types: {e}")))?;
            let response_types_str: String = row
                .try_get("response_types")
                .map_err(|e| AppError::database(format!("Failed to get response_types: {e}")))?;

            Ok(Some(crate::oauth2_server::models::OAuth2Client {
                id: row
                    .try_get("id")
                    .map_err(|e| AppError::database(format!("Failed to get id: {e}")))?,
                client_id: row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?,
                client_secret_hash: row.try_get("client_secret_hash").map_err(|e| {
                    AppError::database(format!("Failed to get client_secret_hash: {e}"))
                })?,
                redirect_uris: serde_json::from_str(&redirect_uris_str)?,
                grant_types: serde_json::from_str(&grant_types_str)?,
                response_types: serde_json::from_str(&response_types_str)?,
                client_name: row
                    .try_get("client_name")
                    .map_err(|e| AppError::database(format!("Failed to get client_name: {e}")))?,
                client_uri: row
                    .try_get("client_uri")
                    .map_err(|e| AppError::database(format!("Failed to get client_uri: {e}")))?,
                scope: row
                    .try_get("scope")
                    .map_err(|e| AppError::database(format!("Failed to get scope: {e}")))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| AppError::database(format!("Failed to get created_at: {e}")))?,
                expires_at: row
                    .try_get("expires_at")
                    .map_err(|e| AppError::database(format!("Failed to get expires_at: {e}")))?,
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
    ) -> AppResult<()> {
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

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        let row = sqlx::query(
            r"
            SELECT code, client_id, user_id, tenant_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at, used, state
            FROM oauth2_auth_codes
            WHERE code = ?1
            "
        )
        .bind(code)
        .fetch_optional(&self.pool)

            .await

            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2AuthCode {
                code: row
                    .try_get("code")
                    .map_err(|e| AppError::database(format!("Failed to get code: {e}")))?,
                client_id: row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?,
                user_id: Uuid::parse_str(
                    &row.try_get::<String, _>("user_id")
                        .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?,
                )
                .map_err(|e| AppError::database(format!("Failed to parse user_id: {e}")))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?,
                redirect_uri: row
                    .try_get("redirect_uri")
                    .map_err(|e| AppError::database(format!("Failed to get redirect_uri: {e}")))?,
                scope: row
                    .try_get("scope")
                    .map_err(|e| AppError::database(format!("Failed to get scope: {e}")))?,
                code_challenge: row.try_get("code_challenge").map_err(|e| {
                    AppError::database(format!("Failed to get code_challenge: {e}"))
                })?,
                code_challenge_method: row.try_get("code_challenge_method").map_err(|e| {
                    AppError::database(format!("Failed to get code_challenge_method: {e}"))
                })?,
                expires_at: row
                    .try_get("expires_at")
                    .map_err(|e| AppError::database(format!("Failed to get expires_at: {e}")))?,
                used: row
                    .try_get("used")
                    .map_err(|e| AppError::database(format!("Failed to get used: {e}")))?,
                state: row
                    .try_get("state")
                    .map_err(|e| AppError::database(format!("Failed to get state: {e}")))?,
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
    ) -> AppResult<()> {
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
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<()> {
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

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        let row = sqlx::query(
            r"
            SELECT token, client_id, user_id, tenant_id, scope, created_at, expires_at, revoked
            FROM oauth2_refresh_tokens
            WHERE token = ?1
            ",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2RefreshToken {
                token: row
                    .try_get("token")
                    .map_err(|e| AppError::database(format!("Failed to get token: {e}")))?,
                client_id: row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?,
                user_id: Uuid::parse_str(
                    &row.try_get::<String, _>("user_id")
                        .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?,
                )
                .map_err(|e| AppError::database(format!("Failed to parse user_id: {e}")))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?,
                scope: row
                    .try_get("scope")
                    .map_err(|e| AppError::database(format!("Failed to get scope: {e}")))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| AppError::database(format!("Failed to get created_at: {e}")))?,
                expires_at: row
                    .try_get("expires_at")
                    .map_err(|e| AppError::database(format!("Failed to get expires_at: {e}")))?,
                revoked: row
                    .try_get("revoked")
                    .map_err(|e| AppError::database(format!("Failed to get revoked: {e}")))?,
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
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
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

            .await

            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2AuthCode {
                code: row
                    .try_get("code")
                    .map_err(|e| AppError::database(format!("Failed to get code: {e}")))?,
                client_id: row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?,
                user_id: Uuid::parse_str(
                    &row.try_get::<String, _>("user_id")
                        .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?,
                )
                .map_err(|e| AppError::database(format!("Failed to parse user_id: {e}")))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?,
                redirect_uri: row
                    .try_get("redirect_uri")
                    .map_err(|e| AppError::database(format!("Failed to get redirect_uri: {e}")))?,
                scope: row
                    .try_get("scope")
                    .map_err(|e| AppError::database(format!("Failed to get scope: {e}")))?,
                expires_at: row
                    .try_get("expires_at")
                    .map_err(|e| AppError::database(format!("Failed to get expires_at: {e}")))?,
                used: row
                    .try_get("used")
                    .map_err(|e| AppError::database(format!("Failed to get used: {e}")))?,
                state: row
                    .try_get("state")
                    .map_err(|e| AppError::database(format!("Failed to get state: {e}")))?,
                code_challenge: row.try_get("code_challenge").map_err(|e| {
                    AppError::database(format!("Failed to get code_challenge: {e}"))
                })?,
                code_challenge_method: row.try_get("code_challenge_method").map_err(|e| {
                    AppError::database(format!("Failed to get code_challenge_method: {e}"))
                })?,
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
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
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
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2RefreshToken {
                token: row
                    .try_get("token")
                    .map_err(|e| AppError::database(format!("Failed to get token: {e}")))?,
                client_id: row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?,
                user_id: Uuid::parse_str(
                    &row.try_get::<String, _>("user_id")
                        .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?,
                )
                .map_err(|e| AppError::database(format!("Failed to parse user_id: {e}")))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?,
                scope: row
                    .try_get("scope")
                    .map_err(|e| AppError::database(format!("Failed to get scope: {e}")))?,
                expires_at: row
                    .try_get("expires_at")
                    .map_err(|e| AppError::database(format!("Failed to get expires_at: {e}")))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| AppError::database(format!("Failed to get created_at: {e}")))?,
                revoked: row
                    .try_get("revoked")
                    .map_err(|e| AppError::database(format!("Failed to get revoked: {e}")))?,
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
    ) -> AppResult<()> {
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

            .await

            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

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
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2State>> {
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

            .await

            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        if let Some(row) = row {
            Ok(Some(crate::oauth2_server::models::OAuth2State {
                state: row
                    .try_get("state")
                    .map_err(|e| AppError::database(format!("Failed to get state: {e}")))?,
                client_id: row
                    .try_get("client_id")
                    .map_err(|e| AppError::database(format!("Failed to get client_id: {e}")))?,
                user_id: row
                    .try_get("user_id")
                    .map_err(|e| AppError::database(format!("Failed to get user_id: {e}")))?,
                tenant_id: row
                    .try_get("tenant_id")
                    .map_err(|e| AppError::database(format!("Failed to get tenant_id: {e}")))?,
                redirect_uri: row
                    .try_get("redirect_uri")
                    .map_err(|e| AppError::database(format!("Failed to get redirect_uri: {e}")))?,
                scope: row
                    .try_get("scope")
                    .map_err(|e| AppError::database(format!("Failed to get scope: {e}")))?,
                code_challenge: row.try_get("code_challenge").map_err(|e| {
                    AppError::database(format!("Failed to get code_challenge: {e}"))
                })?,
                code_challenge_method: row.try_get("code_challenge_method").map_err(|e| {
                    AppError::database(format!("Failed to get code_challenge_method: {e}"))
                })?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| AppError::database(format!("Failed to get created_at: {e}")))?,
                expires_at: row
                    .try_get("expires_at")
                    .map_err(|e| AppError::database(format!("Failed to get expires_at: {e}")))?,
                used: row
                    .try_get("used")
                    .map_err(|e| AppError::database(format!("Failed to get used: {e}")))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get authorization code (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or code not found/expired
    async fn get_authorization_code_impl(
        &self,
        code: &str,
    ) -> AppResult<crate::models::AuthorizationCode> {
        let row = sqlx::query(
            r"
            SELECT code, client_id, user_id, redirect_uri, scope, created_at, expires_at
            FROM oauth2_auth_codes
            WHERE code = ?1 AND expires_at > CURRENT_TIMESTAMP
            ",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        match row {
            Some(row) => Ok(crate::models::AuthorizationCode {
                code: row.get("code"),
                client_id: row.get("client_id"),
                redirect_uri: row.get("redirect_uri"),
                scope: row.get("scope"),
                user_id: Some(Uuid::parse_str(&row.get::<String, _>("user_id"))?),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                is_used: false, // If we can fetch it, it hasn't been used yet
            }),
            None => Err(AppError::not_found("authorization code")),
        }
    }

    /// Delete authorization code (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn delete_authorization_code_impl(&self, code: &str) -> AppResult<()> {
        let result = sqlx::query(
            r"
            DELETE FROM oauth2_auth_codes
            WHERE code = ?1
            ",
        )
        .bind(code)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Failed to delete authorization code: {e}")))?;

        if result.rows_affected() == 0 {
            tracing::warn!("Authorization code not found for deletion: {}", code);
        }

        Ok(())
    }

    // ================================
    // Audit Events (SQLite implementations)
    // ================================

    /// Store audit event (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn store_audit_event_impl(
        &self,
        event: &crate::security::audit::AuditEvent,
    ) -> AppResult<()> {
        let query = r"
            INSERT INTO audit_events (
                id, event_type, severity, message, source, result,
                tenant_id, user_id, ip_address, user_agent, metadata, timestamp
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ";

        let event_type_str = format!("{:?}", event.event_type);
        let severity_str = format!("{:?}", event.severity);
        let metadata_json = serde_json::to_string(&event.metadata)?;

        sqlx::query(query)
            .bind(event.event_id.to_string())
            .bind(&event_type_str)
            .bind(&severity_str)
            .bind(&event.description)
            .bind("security") // source - using generic security source
            .bind(&event.result)
            .bind(event.tenant_id.map(|id| id.to_string()))
            .bind(event.user_id.map(|id| id.to_string()))
            .bind(&event.source_ip)
            .bind(&event.user_agent)
            .bind(&metadata_json)
            .bind(event.timestamp)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }

    // ================================
    // Tenant Management (SQLite implementations)
    // ================================

    /// Get user tenant role (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn get_user_tenant_role_impl(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> AppResult<Option<String>> {
        let row =
            sqlx::query("SELECT role FROM tenant_users WHERE user_id = ?1 AND tenant_id = ?2")
                .bind(user_id.to_string())
                .bind(tenant_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        Ok(row.map(|r| r.get("role")))
    }

    // ================================
    // User OAuth Apps (SQLite implementations)
    // ================================

    /// List user OAuth apps (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn list_user_oauth_apps_impl(
        &self,
        user_id: Uuid,
    ) -> AppResult<Vec<crate::models::UserOAuthApp>> {
        // First ensure the user_oauth_apps table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_oauth_apps (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                client_id TEXT NOT NULL,
                client_secret TEXT NOT NULL,
                redirect_uri TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, provider),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        let rows = sqlx::query(
            r"
            SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
            FROM user_oauth_apps
            WHERE user_id = ?1
            ORDER BY provider
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)

            .await

            .map_err(|e| AppError::database(format!("Database query failed: {e}")))?;

        let mut apps = Vec::new();
        for row in rows {
            apps.push(crate::models::UserOAuthApp {
                id: row.get("id"),
                user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                provider: row.get("provider"),
                client_id: row.get("client_id"),
                client_secret: row.get("client_secret"),
                redirect_uri: row.get("redirect_uri"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(apps)
    }

    /// Remove user OAuth app (internal implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    async fn remove_user_oauth_app_impl(&self, user_id: Uuid, provider: &str) -> AppResult<()> {
        sqlx::query(
            r"
            DELETE FROM user_oauth_apps
            WHERE user_id = ?1 AND provider = ?2
            ",
        )
        .bind(user_id.to_string())
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database(format!("Database operation failed: {e}")))?;

        Ok(())
    }
}

// Implement HasEncryption trait for SQLite (delegates to inherent impl methods)
impl crate::database_plugins::shared::encryption::HasEncryption for Database {
    fn encrypt_data_with_aad(&self, data: &str, aad: &str) -> AppResult<String> {
        // Call inherent impl directly to avoid infinite recursion
        Self::encrypt_data_with_aad_impl(self, data, aad)
    }

    fn decrypt_data_with_aad(&self, encrypted: &str, aad: &str) -> AppResult<String> {
        // Call inherent impl directly to avoid infinite recursion
        Self::decrypt_data_with_aad_impl(self, encrypted, aad)
    }
}

// Implement DatabaseProvider trait for Database (eliminates sqlite.rs wrapper)
use async_trait::async_trait;

#[async_trait]
impl crate::database_plugins::DatabaseProvider for Database {
    async fn new(database_url: &str, encryption_key: Vec<u8>) -> AppResult<Self> {
        // Call inherent impl directly to avoid infinite recursion
        Self::new_impl(database_url, encryption_key).await
    }

    async fn migrate(&self) -> AppResult<()> {
        // Call inherent impl directly
        Self::migrate_impl(self).await
    }

    async fn create_user(&self, user: &User) -> AppResult<Uuid> {
        Self::create_user_impl(self, user).await
    }

    async fn get_user(&self, user_id: Uuid) -> AppResult<Option<User>> {
        Self::get_user_impl(self, user_id).await
    }

    async fn get_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
        Self::get_user_by_email_impl(self, email).await
    }

    async fn get_user_by_email_required(&self, email: &str) -> AppResult<User> {
        Self::get_user_by_email_required_impl(self, email).await
    }

    async fn update_last_active(&self, user_id: Uuid) -> AppResult<()> {
        Self::update_last_active_impl(self, user_id).await
    }

    async fn get_user_count(&self) -> AppResult<i64> {
        Self::get_user_count_impl(self).await
    }

    async fn get_users_by_status(&self, status: &str) -> AppResult<Vec<User>> {
        Self::get_users_by_status_impl(self, status).await
    }

    async fn get_users_by_status_cursor(
        &self,
        status: &str,
        params: &crate::pagination::PaginationParams,
    ) -> AppResult<crate::pagination::CursorPage<User>> {
        Self::get_users_by_status_cursor(self, status, params).await
    }

    async fn update_user_status(
        &self,
        user_id: Uuid,
        new_status: crate::models::UserStatus,
        admin_token_id: &str,
    ) -> AppResult<User> {
        Self::update_user_status(self, user_id, new_status, admin_token_id).await
    }

    async fn update_user_tenant_id(&self, user_id: Uuid, tenant_id: &str) -> AppResult<()> {
        Self::update_user_tenant_id_impl(self, user_id, tenant_id).await
    }

    async fn upsert_user_profile(&self, user_id: Uuid, profile_data: Value) -> AppResult<()> {
        Self::upsert_user_profile_impl(self, user_id, profile_data).await
    }

    async fn get_user_profile(&self, user_id: Uuid) -> AppResult<Option<Value>> {
        Self::get_user_profile_impl(self, user_id).await
    }

    async fn create_goal(&self, user_id: Uuid, goal_data: Value) -> AppResult<String> {
        Self::create_goal_impl(self, user_id, goal_data).await
    }

    async fn get_user_goals(&self, user_id: Uuid) -> AppResult<Vec<Value>> {
        Self::get_user_goals_impl(self, user_id).await
    }

    async fn update_goal_progress(&self, goal_id: &str, current_value: f64) -> AppResult<()> {
        Self::update_goal_progress_impl(self, goal_id, current_value).await
    }

    async fn get_user_configuration(&self, user_id: &str) -> AppResult<Option<String>> {
        Self::get_user_configuration_impl(self, user_id).await
    }

    async fn save_user_configuration(&self, user_id: &str, config_json: &str) -> AppResult<()> {
        Self::save_user_configuration_impl(self, user_id, config_json).await
    }

    async fn store_insight(&self, user_id: Uuid, insight_data: Value) -> AppResult<String> {
        Self::store_insight_impl(self, user_id, insight_data).await
    }

    async fn get_user_insights(
        &self,
        user_id: Uuid,
        insight_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<Value>> {
        Self::get_user_insights(self, user_id, insight_type, limit).await
    }

    async fn create_api_key(&self, api_key: &ApiKey) -> AppResult<()> {
        Self::create_api_key_impl(self, api_key).await
    }

    async fn get_api_key_by_prefix(&self, prefix: &str, hash: &str) -> AppResult<Option<ApiKey>> {
        Self::get_api_key_by_prefix_impl(self, prefix, hash).await
    }

    async fn get_user_api_keys(&self, user_id: Uuid) -> AppResult<Vec<ApiKey>> {
        Self::get_user_api_keys_impl(self, user_id).await
    }

    async fn update_api_key_last_used(&self, api_key_id: &str) -> AppResult<()> {
        Self::update_api_key_last_used_impl(self, api_key_id).await
    }

    async fn deactivate_api_key(&self, api_key_id: &str, user_id: Uuid) -> AppResult<()> {
        Self::deactivate_api_key_impl(self, api_key_id, user_id).await
    }

    async fn get_api_key_by_id(&self, api_key_id: &str) -> AppResult<Option<ApiKey>> {
        Self::get_api_key_by_id_impl(self, api_key_id).await
    }

    async fn get_api_keys_filtered(
        &self,
        _user_email: Option<&str>,
        active_only: bool,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> AppResult<Vec<ApiKey>> {
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

    async fn cleanup_expired_api_keys(&self) -> AppResult<u64> {
        Self::cleanup_expired_api_keys_impl(self).await
    }

    async fn get_expired_api_keys(&self) -> AppResult<Vec<ApiKey>> {
        Self::get_expired_api_keys_impl(self).await
    }

    async fn record_api_key_usage(&self, usage: &ApiKeyUsage) -> AppResult<()> {
        Self::record_api_key_usage_impl(self, usage).await
    }

    async fn get_api_key_current_usage(&self, api_key_id: &str) -> AppResult<u32> {
        Self::get_api_key_current_usage_impl(self, api_key_id).await
    }

    async fn get_api_key_usage_stats(
        &self,
        api_key_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<ApiKeyUsageStats> {
        Self::get_api_key_usage_stats(self, api_key_id, start_date, end_date).await
    }

    async fn record_jwt_usage(&self, usage: &JwtUsage) -> AppResult<()> {
        Self::record_jwt_usage_impl(self, usage).await
    }

    async fn get_jwt_current_usage(&self, user_id: Uuid) -> AppResult<u32> {
        Self::get_jwt_current_usage_impl(self, user_id).await
    }

    async fn get_request_logs(
        &self,
        _api_key_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        _status_filter: Option<&str>,
        _tool_filter: Option<&str>,
    ) -> AppResult<Vec<crate::dashboard_routes::RequestLog>> {
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

    async fn get_system_stats(&self) -> AppResult<(u64, u64)> {
        Self::get_system_stats_impl(self).await
    }

    async fn create_a2a_client(
        &self,
        client: &A2AClient,
        client_secret: &str,
        api_key_id: &str,
    ) -> AppResult<String> {
        Self::create_a2a_client(self, client, client_secret, api_key_id).await
    }

    async fn get_a2a_client(&self, client_id: &str) -> AppResult<Option<A2AClient>> {
        Self::get_a2a_client_impl(self, client_id).await
    }

    async fn get_a2a_client_by_api_key_id(&self, api_key_id: &str) -> AppResult<Option<A2AClient>> {
        Self::get_a2a_client_by_api_key_id_impl(self, api_key_id).await
    }

    async fn get_a2a_client_by_name(&self, name: &str) -> AppResult<Option<A2AClient>> {
        Self::get_a2a_client_by_name_impl(self, name).await
    }

    async fn list_a2a_clients(&self, user_id: &Uuid) -> AppResult<Vec<A2AClient>> {
        Self::list_a2a_clients_impl(self, user_id).await
    }

    async fn deactivate_a2a_client(&self, client_id: &str) -> AppResult<()> {
        Self::deactivate_a2a_client_impl(self, client_id).await
    }

    async fn get_a2a_client_credentials(
        &self,
        client_id: &str,
    ) -> AppResult<Option<(String, String)>> {
        Self::get_a2a_client_credentials(self, client_id).await
    }

    async fn invalidate_a2a_client_sessions(&self, client_id: &str) -> AppResult<()> {
        Self::invalidate_a2a_client_sessions_impl(self, client_id).await
    }

    async fn deactivate_client_api_keys(&self, client_id: &str) -> AppResult<()> {
        Self::deactivate_client_api_keys_impl(self, client_id).await
    }

    async fn create_a2a_session(
        &self,
        client_id: &str,
        user_id: Option<&Uuid>,
        granted_scopes: &[String],
        expires_in_hours: i64,
    ) -> AppResult<String> {
        Self::create_a2a_session(self, client_id, user_id, granted_scopes, expires_in_hours).await
    }

    async fn get_a2a_session(&self, session_token: &str) -> AppResult<Option<A2ASession>> {
        Self::get_a2a_session_impl(self, session_token).await
    }

    async fn update_a2a_session_activity(&self, session_token: &str) -> AppResult<()> {
        Self::update_a2a_session_activity_impl(self, session_token).await
    }

    async fn get_active_a2a_sessions(&self, client_id: &str) -> AppResult<Vec<A2ASession>> {
        Self::get_active_a2a_sessions_impl(self, client_id).await
    }

    async fn create_a2a_task(
        &self,
        client_id: &str,
        session_id: Option<&str>,
        task_type: &str,
        input_data: &Value,
    ) -> AppResult<String> {
        Self::create_a2a_task(self, client_id, session_id, task_type, input_data).await
    }

    async fn get_a2a_task(&self, task_id: &str) -> AppResult<Option<A2ATask>> {
        Self::get_a2a_task_impl(self, task_id).await
    }

    async fn list_a2a_tasks(
        &self,
        client_id: Option<&str>,
        status_filter: Option<&TaskStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<A2ATask>> {
        Self::list_a2a_tasks(self, client_id, status_filter, limit, offset).await
    }

    async fn update_a2a_task_status(
        &self,
        task_id: &str,
        status: &TaskStatus,
        result: Option<&Value>,
        error: Option<&str>,
    ) -> AppResult<()> {
        Self::update_a2a_task_status(self, task_id, status, result, error).await
    }

    async fn record_a2a_usage(&self, usage: &A2AUsage) -> AppResult<()> {
        Self::record_a2a_usage_impl(self, usage).await
    }

    async fn get_a2a_client_current_usage(&self, client_id: &str) -> AppResult<u32> {
        Self::get_a2a_client_current_usage_impl(self, client_id).await
    }

    async fn get_a2a_usage_stats(
        &self,
        client_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<crate::database::A2AUsageStats> {
        Self::get_a2a_usage_stats(self, client_id, start_date, end_date).await
    }

    async fn get_a2a_client_usage_history(
        &self,
        client_id: &str,
        days: u32,
    ) -> AppResult<Vec<(DateTime<Utc>, u32, u32)>> {
        Self::get_a2a_client_usage_history(self, client_id, days).await
    }

    async fn get_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<DateTime<Utc>>> {
        Self::get_provider_last_sync(self, user_id, provider).await
    }

    async fn update_provider_last_sync(
        &self,
        user_id: Uuid,
        provider: &str,
        sync_time: DateTime<Utc>,
    ) -> AppResult<()> {
        Self::update_provider_last_sync(self, user_id, provider, sync_time).await
    }

    async fn get_top_tools_analysis(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> AppResult<Vec<crate::dashboard_routes::ToolUsage>> {
        Self::get_top_tools_analysis(self, user_id, start_time, end_time).await
    }

    async fn create_admin_token(
        &self,
        request: &crate::admin::models::CreateAdminTokenRequest,
        admin_jwt_secret: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> AppResult<crate::admin::models::GeneratedAdminToken> {
        Self::create_admin_token(self, request, admin_jwt_secret, jwks_manager).await
    }

    async fn get_admin_token_by_id(
        &self,
        token_id: &str,
    ) -> AppResult<Option<crate::admin::models::AdminToken>> {
        Self::get_admin_token_by_id(self, token_id).await
    }

    async fn get_admin_token_by_prefix(
        &self,
        token_prefix: &str,
    ) -> AppResult<Option<crate::admin::models::AdminToken>> {
        Self::get_admin_token_by_prefix(self, token_prefix).await
    }

    async fn list_admin_tokens(
        &self,
        include_inactive: bool,
    ) -> AppResult<Vec<crate::admin::models::AdminToken>> {
        Self::list_admin_tokens(self, include_inactive).await
    }

    async fn deactivate_admin_token(&self, token_id: &str) -> AppResult<()> {
        Self::deactivate_admin_token_impl(self, token_id).await
    }

    async fn update_admin_token_last_used(
        &self,
        token_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        Self::update_admin_token_last_used(self, token_id, ip_address).await
    }

    async fn record_admin_token_usage(
        &self,
        usage: &crate::admin::models::AdminTokenUsage,
    ) -> AppResult<()> {
        Self::record_admin_token_usage(self, usage).await
    }

    async fn get_admin_token_usage_history(
        &self,
        token_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> AppResult<Vec<crate::admin::models::AdminTokenUsage>> {
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
    ) -> AppResult<()> {
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
    ) -> AppResult<Vec<serde_json::Value>> {
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
    ) -> AppResult<()> {
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
    ) -> AppResult<Vec<(String, String, String, DateTime<Utc>, bool)>> {
        Self::load_rsa_keypairs(self).await
    }

    async fn update_rsa_keypair_active_status(&self, kid: &str, is_active: bool) -> AppResult<()> {
        Self::update_rsa_keypair_active_status_impl(self, kid, is_active).await
    }

    async fn create_tenant(&self, tenant: &crate::models::Tenant) -> AppResult<()> {
        Self::create_tenant_impl(self, tenant).await
    }

    async fn get_tenant_by_id(&self, tenant_id: Uuid) -> AppResult<crate::models::Tenant> {
        Self::get_tenant_by_id_impl(self, tenant_id).await
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> AppResult<crate::models::Tenant> {
        Self::get_tenant_by_slug_impl(self, slug).await
    }

    async fn list_tenants_for_user(&self, user_id: Uuid) -> AppResult<Vec<crate::models::Tenant>> {
        Self::list_tenants_for_user_impl(self, user_id).await
    }

    async fn store_tenant_oauth_credentials(
        &self,
        credentials: &crate::tenant::TenantOAuthCredentials,
    ) -> AppResult<()> {
        Self::store_tenant_oauth_credentials_impl(self, credentials).await
    }

    async fn get_tenant_oauth_providers(
        &self,
        tenant_id: Uuid,
    ) -> AppResult<Vec<crate::tenant::TenantOAuthCredentials>> {
        Self::get_tenant_oauth_providers_impl(self, tenant_id).await
    }

    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<crate::tenant::TenantOAuthCredentials>> {
        Self::get_tenant_oauth_credentials_impl(self, tenant_id, provider).await
    }

    async fn create_oauth_app(&self, app: &crate::models::OAuthApp) -> AppResult<()> {
        Self::create_oauth_app_impl(self, app).await
    }

    async fn get_oauth_app_by_client_id(
        &self,
        client_id: &str,
    ) -> AppResult<crate::models::OAuthApp> {
        Self::get_oauth_app_by_client_id_impl(self, client_id).await
    }

    async fn list_oauth_apps_for_user(
        &self,
        user_id: Uuid,
    ) -> AppResult<Vec<crate::models::OAuthApp>> {
        Self::list_oauth_apps_for_user(self, user_id).await
    }

    async fn store_oauth2_client(
        &self,
        client: &crate::oauth2_server::models::OAuth2Client,
    ) -> AppResult<()> {
        Self::store_oauth2_client_impl(self, client).await
    }

    async fn get_oauth2_client(
        &self,
        client_id: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2Client>> {
        Self::get_oauth2_client_impl(self, client_id).await
    }

    async fn store_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> AppResult<()> {
        Self::store_oauth2_auth_code_impl(self, auth_code).await
    }

    async fn get_oauth2_auth_code(
        &self,
        code: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Self::get_oauth2_auth_code_impl(self, code).await
    }

    async fn update_oauth2_auth_code(
        &self,
        auth_code: &crate::oauth2_server::models::OAuth2AuthCode,
    ) -> AppResult<()> {
        Self::update_oauth2_auth_code_impl(self, auth_code).await
    }

    async fn store_oauth2_refresh_token(
        &self,
        refresh_token: &crate::oauth2_server::models::OAuth2RefreshToken,
    ) -> AppResult<()> {
        Self::store_oauth2_refresh_token_impl(self, refresh_token).await
    }

    async fn get_oauth2_refresh_token(
        &self,
        token: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::get_oauth2_refresh_token_impl(self, token).await
    }

    async fn revoke_oauth2_refresh_token(&self, token: &str) -> AppResult<()> {
        Self::revoke_oauth2_refresh_token_impl(self, token).await
    }

    async fn consume_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2AuthCode>> {
        Self::consume_auth_code_impl(self, code, client_id, redirect_uri, now).await
    }

    async fn consume_refresh_token(
        &self,
        token: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::consume_refresh_token_impl(self, token, client_id, now).await
    }

    async fn get_refresh_token_by_value(
        &self,
        token: &str,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2RefreshToken>> {
        Self::get_refresh_token_by_value(self, token).await
    }

    async fn store_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        user_id: Uuid,
    ) -> AppResult<()> {
        Self::store_authorization_code(self, code, client_id, redirect_uri, scope, user_id).await
    }

    async fn get_authorization_code(
        &self,
        code: &str,
    ) -> AppResult<crate::models::AuthorizationCode> {
        Self::get_authorization_code_impl(self, code).await
    }

    async fn delete_authorization_code(&self, code: &str) -> AppResult<()> {
        Self::delete_authorization_code_impl(self, code).await
    }

    async fn store_oauth2_state(
        &self,
        state: &crate::oauth2_server::models::OAuth2State,
    ) -> AppResult<()> {
        Self::store_oauth2_state_impl(self, state).await
    }

    async fn consume_oauth2_state(
        &self,
        state_value: &str,
        client_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Option<crate::oauth2_server::models::OAuth2State>> {
        Self::consume_oauth2_state_impl(self, state_value, client_id, now).await
    }

    async fn store_key_version(
        &self,
        version: &crate::security::key_rotation::KeyVersion,
    ) -> AppResult<()> {
        Self::store_key_version(self, version).await
    }

    async fn get_key_versions(
        &self,
        tenant_id: Option<Uuid>,
    ) -> AppResult<Vec<crate::security::key_rotation::KeyVersion>> {
        Self::get_key_versions(self, tenant_id).await
    }

    async fn get_current_key_version(
        &self,
        tenant_id: Option<Uuid>,
    ) -> AppResult<Option<crate::security::key_rotation::KeyVersion>> {
        Self::get_current_key_version(self, tenant_id).await
    }

    async fn update_key_version_status(
        &self,
        tenant_id: Option<Uuid>,
        version: u32,
        is_active: bool,
    ) -> AppResult<()> {
        Self::update_key_version_status(self, tenant_id, version, is_active).await
    }

    async fn delete_old_key_versions(
        &self,
        tenant_id: Option<Uuid>,
        keep_count: u32,
    ) -> AppResult<u64> {
        Self::delete_old_key_versions(self, tenant_id, keep_count).await
    }

    async fn get_all_tenants(&self) -> AppResult<Vec<crate::models::Tenant>> {
        Self::get_all_tenants_impl(self).await
    }

    async fn store_audit_event(&self, event: &crate::security::audit::AuditEvent) -> AppResult<()> {
        Self::store_audit_event_impl(self, event).await
    }

    async fn get_audit_events(
        &self,
        tenant_id: Option<Uuid>,
        event_type: Option<&str>,
        limit: Option<u32>,
    ) -> AppResult<Vec<crate::security::audit::AuditEvent>> {
        Self::get_audit_events(self, tenant_id, event_type, limit).await
    }

    async fn get_user_tenant_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> AppResult<Option<String>> {
        Self::get_user_tenant_role_impl(self, user_id, tenant_id).await
    }

    async fn get_or_create_system_secret(&self, secret_type: &str) -> AppResult<String> {
        Self::get_or_create_system_secret_impl(self, secret_type).await
    }

    async fn get_system_secret(&self, secret_type: &str) -> AppResult<String> {
        Self::get_system_secret_impl(self, secret_type).await
    }

    async fn update_system_secret(&self, secret_type: &str, new_value: &str) -> AppResult<()> {
        Self::update_system_secret_impl(self, secret_type, new_value).await
    }

    async fn store_oauth_notification(
        &self,
        user_id: Uuid,
        provider: &str,
        success: bool,
        message: &str,
        expires_at: Option<&str>,
    ) -> AppResult<String> {
        Self::store_oauth_notification(self, user_id, provider, success, message, expires_at).await
    }

    async fn get_unread_oauth_notifications(
        &self,
        user_id: Uuid,
    ) -> AppResult<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Self::get_unread_oauth_notifications(self, user_id).await
    }

    async fn mark_oauth_notification_read(
        &self,
        notification_id: &str,
        user_id: Uuid,
    ) -> AppResult<bool> {
        Self::mark_oauth_notification_read(self, notification_id, user_id).await
    }

    async fn mark_all_oauth_notifications_read(&self, user_id: Uuid) -> AppResult<u64> {
        Self::mark_all_oauth_notifications_read_impl(self, user_id).await
    }

    async fn get_all_oauth_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> AppResult<Vec<crate::database::oauth_notifications::OAuthNotification>> {
        Self::get_all_oauth_notifications(self, user_id, limit).await
    }

    async fn save_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
        config: &crate::config::fitness_config::FitnessConfig,
    ) -> AppResult<String> {
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
    ) -> AppResult<String> {
        let manager = self.fitness_configurations();
        manager
            .save_user_config(tenant_id, user_id, configuration_name, config)
            .await
    }

    async fn get_tenant_fitness_config(
        &self,
        tenant_id: &str,
        configuration_name: &str,
    ) -> AppResult<Option<crate::config::fitness_config::FitnessConfig>> {
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
    ) -> AppResult<Option<crate::config::fitness_config::FitnessConfig>> {
        let manager = self.fitness_configurations();
        manager
            .get_user_config(tenant_id, user_id, configuration_name)
            .await
    }

    async fn list_tenant_fitness_configurations(&self, tenant_id: &str) -> AppResult<Vec<String>> {
        let manager = self.fitness_configurations();
        manager.list_tenant_configurations(tenant_id).await
    }

    async fn list_user_fitness_configurations(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<String>> {
        let manager = self.fitness_configurations();
        manager.list_user_configurations(tenant_id, user_id).await
    }

    async fn delete_fitness_config(
        &self,
        tenant_id: &str,
        user_id: Option<&str>,
        configuration_name: &str,
    ) -> AppResult<bool> {
        let manager = self.fitness_configurations();
        manager
            .delete_config(tenant_id, user_id, configuration_name)
            .await
    }

    // OAuth Token Management
    async fn upsert_user_oauth_token(&self, token: &UserOAuthToken) -> AppResult<()> {
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
    ) -> AppResult<Option<UserOAuthToken>> {
        Self::get_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    async fn get_user_oauth_tokens(&self, user_id: Uuid) -> AppResult<Vec<UserOAuthToken>> {
        Self::get_user_oauth_tokens_impl(self, user_id).await
    }

    async fn get_tenant_provider_tokens(
        &self,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<Vec<UserOAuthToken>> {
        Self::get_tenant_provider_tokens(self, tenant_id, provider).await
    }

    async fn delete_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) -> AppResult<()> {
        Self::delete_user_oauth_token(self, user_id, tenant_id, provider).await
    }

    async fn delete_user_oauth_tokens(&self, user_id: Uuid) -> AppResult<()> {
        Self::delete_user_oauth_tokens_impl(self, user_id).await
    }

    async fn refresh_user_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
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
    ) -> AppResult<()> {
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
    ) -> AppResult<Option<UserOAuthApp>> {
        Self::get_user_oauth_app(self, user_id, provider).await
    }

    async fn list_user_oauth_apps(&self, user_id: Uuid) -> AppResult<Vec<UserOAuthApp>> {
        Self::list_user_oauth_apps_impl(self, user_id).await
    }

    async fn remove_user_oauth_app(&self, user_id: Uuid, provider: &str) -> AppResult<()> {
        Self::remove_user_oauth_app_impl(self, user_id, provider).await
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
