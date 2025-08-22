// ABOUTME: Core database management with migration system for SQLite and PostgreSQL
// ABOUTME: Handles schema setup, user management, API keys, analytics, and A2A authentication

pub mod a2a;
pub mod analytics;
pub mod api_keys;
pub mod user_oauth_tokens;
pub mod users;

pub mod tests;

pub use a2a::{A2AUsage, A2AUsageStats};

use anyhow::Result;
use sqlx::{Pool, Sqlite, SqlitePool};

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

    /// Run all database migrations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any migration fails
    /// - Database connection is lost during migration
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

        // UserOAuthToken tables
        self.migrate_user_oauth_tokens().await?;

        // Tenant management tables
        self.migrate_tenant_management().await?;

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
                provider TEXT NOT NULL CHECK (provider IN ('strava', 'fitbit', 'garmin', 'runkeeper')),
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

    /// Update Strava token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_strava_token(
        &self,
        user_id: uuid::Uuid,
        token: &crate::models::DecryptedToken,
    ) -> Result<()> {
        // Encrypt the tokens before storing
        let encrypted_access = self.encrypt_data(&token.access_token)?;
        let encrypted_refresh = self.encrypt_data(&token.refresh_token)?;

        sqlx::query(
            r"
            UPDATE users SET
                strava_access_token = $1,
                strava_refresh_token = $2,
                strava_expires_at = $3,
                strava_scope = $4,
                strava_nonce = $5
            WHERE id = $6
            ",
        )
        .bind(encrypted_access)
        .bind(encrypted_refresh)
        .bind(token.expires_at.timestamp())
        .bind(&token.scope)
        .bind("v1") // Default nonce value for new tokens
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get Strava token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_strava_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<crate::models::DecryptedToken>> {
        let user = self.get_user(user_id).await?;
        if let Some(encrypted_token) = user.and_then(|u| u.strava_token) {
            // Decrypt the tokens
            let decrypted_access = self.decrypt_data(&encrypted_token.access_token)?;
            let decrypted_refresh = self.decrypt_data(&encrypted_token.refresh_token)?;

            Ok(Some(crate::models::DecryptedToken {
                access_token: decrypted_access,
                refresh_token: decrypted_refresh,
                expires_at: encrypted_token.expires_at,
                scope: encrypted_token.scope,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update Fitbit token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_fitbit_token(
        &self,
        user_id: uuid::Uuid,
        token: &crate::models::DecryptedToken,
    ) -> Result<()> {
        // Encrypt the tokens before storing
        let encrypted_access = self.encrypt_data(&token.access_token)?;
        let encrypted_refresh = self.encrypt_data(&token.refresh_token)?;

        sqlx::query(
            r"
            UPDATE users SET
                fitbit_access_token = $1,
                fitbit_refresh_token = $2,
                fitbit_expires_at = $3,
                fitbit_scope = $4,
                fitbit_nonce = $5
            WHERE id = $6
            ",
        )
        .bind(encrypted_access)
        .bind(encrypted_refresh)
        .bind(token.expires_at.timestamp())
        .bind(&token.scope)
        .bind("v1") // Default nonce value for new tokens
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get Fitbit token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_fitbit_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<crate::models::DecryptedToken>> {
        let user = self.get_user(user_id).await?;
        if let Some(encrypted_token) = user.and_then(|u| u.fitbit_token) {
            // Decrypt the tokens
            let decrypted_access = self.decrypt_data(&encrypted_token.access_token)?;
            let decrypted_refresh = self.decrypt_data(&encrypted_token.refresh_token)?;

            Ok(Some(crate::models::DecryptedToken {
                access_token: decrypted_access,
                refresh_token: decrypted_refresh,
                expires_at: encrypted_token.expires_at,
                scope: encrypted_token.scope,
            }))
        } else {
            Ok(None)
        }
    }

    /// Clear Strava token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn clear_strava_token(&self, user_id: uuid::Uuid) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users SET
                strava_access_token = NULL,
                strava_refresh_token = NULL,
                strava_expires_at = NULL,
                strava_scope = NULL,
                strava_nonce = NULL
            WHERE id = $1
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Clear Fitbit token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn clear_fitbit_token(&self, user_id: uuid::Uuid) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users SET
                fitbit_access_token = NULL,
                fitbit_refresh_token = NULL,
                fitbit_expires_at = NULL,
                fitbit_scope = NULL,
                fitbit_nonce = NULL
            WHERE id = $1
            ",
        )
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
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
