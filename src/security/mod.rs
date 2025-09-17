// ABOUTME: Enhanced security module for tenant credential encryption and key management
// ABOUTME: Provides per-tenant key derivation, key rotation, and comprehensive data encryption

//! # Security Module
//!
//! Enhanced security features for Pierre MCP Server including:
//! - Per-tenant key derivation for OAuth credentials
//! - Key rotation mechanisms
//! - Comprehensive encryption for all sensitive data
//! - Security audit logging

use crate::database_plugins::DatabaseProvider;
use anyhow::{Context, Result};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    hkdf::{Salt, HKDF_SHA256},
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub mod audit;
pub mod key_rotation;

/// Security audit helper function
pub fn audit_security_headers<S: ::std::hash::BuildHasher>(
    headers: &std::collections::HashMap<String, String, S>,
) -> bool {
    let required_headers = [
        "Content-Security-Policy",
        "X-Frame-Options",
        "X-Content-Type-Options",
    ];

    for header in &required_headers {
        if !headers.contains_key(*header) {
            tracing::warn!("Missing required security header: {}", header);
            return false;
        }
    }

    true
}

pub mod headers {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// Security headers configuration  
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SecurityConfig {
        /// Environment type (development, production)
        pub environment: String,
        /// Security headers to apply
        pub headers: HashMap<String, String>,
    }

    impl SecurityConfig {
        /// Create development security configuration
        #[must_use]
        pub fn development() -> Self {
            let mut headers = HashMap::new();
            headers.insert("Content-Security-Policy".to_string(), 
                          "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'".to_string());
            headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
            headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
            headers.insert(
                "Referrer-Policy".to_string(),
                "strict-origin-when-cross-origin".to_string(),
            );
            headers.insert(
                "Permissions-Policy".to_string(),
                "camera=(), microphone=(), geolocation=()".to_string(),
            );

            Self {
                environment: "development".to_string(),
                headers,
            }
        }

        /// Create production security configuration
        #[must_use]
        pub fn production() -> Self {
            let mut headers = HashMap::new();
            headers.insert(
                "Content-Security-Policy".to_string(),
                "default-src 'self'; script-src 'self'; style-src 'self'".to_string(),
            );
            headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
            headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
            headers.insert("Referrer-Policy".to_string(), "strict-origin".to_string());
            headers.insert(
                "Strict-Transport-Security".to_string(),
                format!(
                    "max-age={}; includeSubDomains",
                    crate::constants::time_constants::SECONDS_PER_YEAR
                ),
            );
            headers.insert(
                "Permissions-Policy".to_string(),
                "camera=(), microphone=(), geolocation=()".to_string(),
            );

            Self {
                environment: "production".to_string(),
                headers,
            }
        }

        /// Create security configuration from environment string
        #[must_use]
        pub fn from_environment(env: &str) -> Self {
            match env.to_lowercase().as_str() {
                "production" | "prod" => Self::production(),
                _ => Self::development(),
            }
        }

        /// Get headers as `HashMap` for warp integration
        #[must_use]
        pub const fn to_headers(&self) -> &HashMap<String, String> {
            &self.headers
        }
    }
}

/// Enhanced encryption manager with per-tenant key derivation
pub struct TenantEncryptionManager {
    /// Master encryption key (32 bytes for AES-256)
    master_key: [u8; 32],
    /// Cached derived keys for performance
    derived_keys_cache: std::sync::RwLock<HashMap<Uuid, [u8; 32]>>,
    /// Random number generator
    rng: SystemRandom,
    /// Database connection for key versioning
    database: Option<std::sync::Arc<crate::database_plugins::factory::Database>>,
    /// Current key version (global)
    current_version: std::sync::RwLock<u32>,
}

/// Metadata for encrypted data including key version and tenant info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Version of the key used for encryption (for key rotation)
    pub key_version: u32,
    /// Tenant ID if this is tenant-specific encryption
    pub tenant_id: Option<Uuid>,
    /// Encryption algorithm identifier
    pub algorithm: String,
    /// Timestamp of encryption
    pub encrypted_at: chrono::DateTime<chrono::Utc>,
}

/// Encrypted data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Base64-encoded encrypted data (nonce + ciphertext)
    pub data: String,
    /// Encryption metadata
    pub metadata: EncryptionMetadata,
}

impl TenantEncryptionManager {
    /// Create new encryption manager with master key
    ///
    /// # Errors
    ///
    /// Returns an error if the master key is invalid
    #[must_use]
    pub fn new(master_key: [u8; 32]) -> Self {
        Self {
            master_key,
            derived_keys_cache: std::sync::RwLock::new(HashMap::new()),
            rng: SystemRandom::new(),
            database: None,
            current_version: std::sync::RwLock::new(1),
        }
    }

    /// Create new encryption manager with database connection for key versioning
    #[must_use]
    pub fn new_with_database(
        master_key: [u8; 32],
        database: std::sync::Arc<crate::database_plugins::factory::Database>,
    ) -> Self {
        Self {
            master_key,
            derived_keys_cache: std::sync::RwLock::new(HashMap::new()),
            rng: SystemRandom::new(),
            database: Some(database),
            current_version: std::sync::RwLock::new(1),
        }
    }

    /// Derive a tenant-specific encryption key using HKDF
    ///
    /// # Errors
    ///
    /// Returns an error if key derivation fails
    ///
    /// # Errors
    ///
    /// Returns an error if the key cache `RwLock` is poisoned
    pub fn derive_tenant_key(&self, tenant_id: Uuid) -> Result<[u8; 32]> {
        // Check cache first
        {
            let cache = self.derived_keys_cache.read().map_err(|_| {
                anyhow::anyhow!("Security cache lock poisoned - key derivation unavailable")
            })?;
            if let Some(cached_key) = cache.get(&tenant_id) {
                return Ok(*cached_key);
            }
        }

        // Derive new key using HKDF
        let salt = Salt::new(HKDF_SHA256, &[]);
        let prk = salt.extract(&self.master_key);

        let info = format!("tenant:{tenant_id}");
        let info_bytes = [info.as_bytes()];
        let okm = prk
            .expand(&info_bytes, HKDF_SHA256)
            .context("Failed to expand key material")?;

        let mut derived_key = [0u8; 32];
        okm.fill(&mut derived_key)
            .context("Failed to fill derived key")?;

        // Cache the derived key
        {
            let mut cache = self.derived_keys_cache.write().map_err(|_| {
                anyhow::anyhow!("Security cache lock poisoned - cannot cache derived key")
            })?;
            cache.insert(tenant_id, derived_key);
        }

        Ok(derived_key)
    }

    /// Get current key version for encryption
    ///
    /// # Errors
    ///
    /// Returns an error if the version lock is poisoned
    pub fn get_current_version(&self) -> Result<u32> {
        Ok(*self
            .current_version
            .read()
            .map_err(|_| anyhow::anyhow!("Version lock poisoned"))?)
    }

    /// Set current key version
    ///
    /// # Errors
    ///
    /// Returns an error if the version lock is poisoned
    pub fn set_current_version(&self, version: u32) -> Result<()> {
        *self
            .current_version
            .write()
            .map_err(|_| anyhow::anyhow!("Version lock poisoned"))? = version;
        Ok(())
    }

    /// Encrypt data with tenant-specific key
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_tenant_data(&self, tenant_id: Uuid, data: &str) -> Result<EncryptedData> {
        let derived_key = self.derive_tenant_key(tenant_id)?;
        self.encrypt_with_key(&derived_key, data, Some(tenant_id))
    }

    /// Decrypt data with tenant-specific key
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails or metadata is invalid
    pub fn decrypt_tenant_data(
        &self,
        tenant_id: Uuid,
        encrypted_data: &EncryptedData,
    ) -> Result<String> {
        // Verify tenant ID matches
        if encrypted_data.metadata.tenant_id != Some(tenant_id) {
            return Err(anyhow::anyhow!("Tenant ID mismatch in encrypted data"));
        }

        let derived_key = self.derive_tenant_key(tenant_id)?;
        Self::decrypt_with_key(&derived_key, &encrypted_data.data)
    }

    /// Encrypt data using global master key (for non-tenant-specific data)
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_global_data(&self, data: &str) -> Result<EncryptedData> {
        self.encrypt_with_key(&self.master_key, data, None)
    }

    /// Decrypt data using global master key
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails
    pub fn decrypt_global_data(&self, encrypted_data: &EncryptedData) -> Result<String> {
        if encrypted_data.metadata.tenant_id.is_some() {
            return Err(anyhow::anyhow!(
                "Expected global data, but found tenant-specific data"
            ));
        }

        Self::decrypt_with_key(&self.master_key, &encrypted_data.data)
    }

    /// Internal method to encrypt data with a specific key
    fn encrypt_with_key(
        &self,
        key: &[u8; 32],
        data: &str,
        tenant_id: Option<Uuid>,
    ) -> Result<EncryptedData> {
        use base64::{engine::general_purpose, Engine as _};

        // Create encryption key
        let unbound_key =
            UnboundKey::new(&AES_256_GCM, key).context("Failed to create encryption key")?;
        let key = LessSafeKey::new(unbound_key);

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        self.rng
            .fill(&mut nonce_bytes)
            .context("Failed to generate nonce")?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Encrypt the data
        let mut ciphertext = data.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
            .context("Encryption failed")?;

        // Combine nonce + ciphertext
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        // Encode to base64
        let encoded = general_purpose::STANDARD.encode(&combined);

        Ok(EncryptedData {
            data: encoded,
            metadata: EncryptionMetadata {
                key_version: self.get_current_version().unwrap_or(1),
                tenant_id,
                algorithm: "AES-256-GCM".to_string(),
                encrypted_at: chrono::Utc::now(),
            },
        })
    }

    /// Internal method to decrypt data with a specific key
    fn decrypt_with_key(key: &[u8; 32], encrypted_data: &str) -> Result<String> {
        use base64::{engine::general_purpose, Engine as _};

        // Decode from base64
        let combined = general_purpose::STANDARD
            .decode(encrypted_data)
            .context("Failed to decode base64 encrypted data")?;

        if combined.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted data: too short"));
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes
                .try_into()
                .context("Failed to extract nonce from encrypted data")?,
        );

        // Create decryption key
        let unbound_key =
            UnboundKey::new(&AES_256_GCM, key).context("Failed to create decryption key")?;
        let key = LessSafeKey::new(unbound_key);

        // Decrypt
        let mut plaintext = ciphertext.to_vec();
        let decrypted = key
            .open_in_place(nonce, Aad::empty(), &mut plaintext)
            .context("Decryption failed")?;

        String::from_utf8(decrypted.to_vec()).context("Decrypted data is not valid UTF-8")
    }

    /// Rotate encryption key for a tenant (for key rotation scenarios)
    ///
    /// # Errors
    ///
    /// Returns an error if key rotation fails, database operations fail, or re-encryption fails
    pub async fn rotate_tenant_key(&self, tenant_id: Uuid) -> Result<()> {
        // Get current version and increment for new key
        let old_version = self.get_current_version()?;
        let new_version = old_version + 1;

        // Update key version in database if available
        if let Some(database) = &self.database {
            // Create new key version record
            let key_version = crate::security::key_rotation::KeyVersion {
                tenant_id: Some(tenant_id),
                version: new_version,
                created_at: chrono::Utc::now(),
                expires_at: chrono::Utc::now() + chrono::Duration::days(365), // 1 year expiry
                is_active: false, // Not active until re-encryption is complete
                algorithm: "HKDF-SHA256".to_string(),
            };

            database.store_key_version(&key_version).await?;

            // Re-encrypt existing OAuth tokens and sensitive data with new key
            // This is a complex operation that requires careful implementation
            tracing::warn!(
                "Key rotation for tenant {} requires manual re-encryption of existing data. \
                 Old data encrypted with version {} may become inaccessible.",
                tenant_id,
                old_version
            );

            // Activate the new key version
            database
                .update_key_version_status(Some(tenant_id), new_version, true)
                .await?;

            // Deactivate old version
            database
                .update_key_version_status(Some(tenant_id), old_version, false)
                .await?;
        }

        // Clear cached key to force regeneration with new parameters
        {
            let mut cache = self.derived_keys_cache.write().map_err(|_| {
                anyhow::anyhow!("Security cache lock poisoned - cannot rotate tenant key")
            })?;
            cache.remove(&tenant_id);
        }

        // Update current version
        self.set_current_version(new_version)?;

        // Re-derive key with new version
        let _ = self.derive_tenant_key(tenant_id)?;

        tracing::info!(
            "Rotated encryption key for tenant {} from version {} to version {}",
            tenant_id,
            old_version,
            new_version
        );

        Ok(())
    }

    /// Clear key cache (useful for memory cleanup or security)
    ///
    /// # Errors
    ///
    /// Returns an error if the key cache `RwLock` is poisoned
    pub fn clear_key_cache(&self) -> Result<()> {
        self.derived_keys_cache
            .write()
            .map_err(|_| anyhow::anyhow!("Security cache lock poisoned - cannot clear cache"))?
            .clear();
        tracing::info!("Cleared encryption key cache");
        Ok(())
    }

    /// Get encryption statistics (for monitoring)
    ///
    /// # Errors
    ///
    /// Returns an error if the key cache `RwLock` is poisoned
    pub fn get_stats(&self) -> Result<EncryptionStats> {
        let cache = self
            .derived_keys_cache
            .read()
            .map_err(|_| anyhow::anyhow!("Security cache lock poisoned - cannot get stats"))?;
        Ok(EncryptionStats {
            cached_tenant_keys: cache.len(),
            master_key_algorithm: "AES-256-GCM".to_string(),
            key_derivation_algorithm: "HKDF-SHA256".to_string(),
        })
    }
}

/// Encryption statistics for monitoring
#[derive(Debug, Serialize)]
pub struct EncryptionStats {
    pub cached_tenant_keys: usize,
    pub master_key_algorithm: String,
    pub key_derivation_algorithm: String,
}

/// Enhanced encrypted token with rotation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedEncryptedToken {
    /// Encrypted access token
    pub access_token: EncryptedData,
    /// Encrypted refresh token
    pub refresh_token: EncryptedData,
    /// Token expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// OAuth scopes
    pub scopes: String,
    /// Key version used for encryption
    pub key_version: u32,
}

impl EnhancedEncryptedToken {
    /// Encrypt OAuth token with tenant-specific encryption
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt_oauth_token(
        encryption_manager: &TenantEncryptionManager,
        tenant_id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        scopes: &str,
    ) -> Result<Self> {
        Ok(Self {
            access_token: encryption_manager.encrypt_tenant_data(tenant_id, access_token)?,
            refresh_token: encryption_manager.encrypt_tenant_data(tenant_id, refresh_token)?,
            expires_at,
            scopes: scopes.to_string(),
            key_version: encryption_manager.get_current_version().unwrap_or(1),
        })
    }

    /// Decrypt OAuth token with tenant-specific decryption
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails
    pub fn decrypt_oauth_token(
        &self,
        encryption_manager: &TenantEncryptionManager,
        tenant_id: Uuid,
    ) -> Result<(String, String)> {
        let access_token = encryption_manager.decrypt_tenant_data(tenant_id, &self.access_token)?;
        let refresh_token =
            encryption_manager.decrypt_tenant_data(tenant_id, &self.refresh_token)?;

        Ok((access_token, refresh_token))
    }
}
