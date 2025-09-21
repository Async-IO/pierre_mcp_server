// ABOUTME: Two-tier key management system for secure database encryption and secret storage
// ABOUTME: Implements MEK (Master Encryption Key) from environment and DEK (Database Encryption Key) stored encrypted

use crate::database_plugins::DatabaseProvider;
use anyhow::{anyhow, Result};
use base64::Engine;
use std::env;
use tracing::{info, warn};

/// Master Encryption Key (MEK) - Tier 1
/// Loaded from environment variable or external key management system
pub struct MasterEncryptionKey {
    key: [u8; 32],
}

/// Database Encryption Key (DEK) - Tier 2
/// Stored encrypted in database, used for actual data encryption
pub struct DatabaseEncryptionKey {
    key: [u8; 32],
}

impl MasterEncryptionKey {
    /// Create MEK from raw key bytes - primarily for testing
    #[must_use]
    pub const fn from_bytes(key: [u8; 32]) -> Self {
        Self { key }
    }
    /// Load MEK from environment variable or generate for development
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The environment variable contains invalid base64 encoding
    /// - The decoded key is not exactly 32 bytes
    /// - Random key generation fails in development mode
    pub fn load_or_generate() -> Result<Self> {
        // Try to load from environment first
        if let Ok(encoded_key) = env::var("PIERRE_MASTER_ENCRYPTION_KEY") {
            return Self::load_from_environment(&encoded_key);
        }

        // Development mode: generate and log warning
        Ok(Self::generate_for_development())
    }

    /// Load MEK from base64-encoded environment variable
    ///
    /// # Errors
    /// Returns error if decoding fails or key is wrong length
    fn load_from_environment(encoded_key: &str) -> Result<Self> {
        info!("Loading Master Encryption Key from environment variable");
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded_key)
            .map_err(|e| {
                anyhow!(
                    "Invalid base64 encoding in PIERRE_MASTER_ENCRYPTION_KEY: {}",
                    e
                )
            })?;

        if key_bytes.len() != 32 {
            return Err(anyhow!(
                "Master encryption key must be exactly 32 bytes, got {} bytes",
                key_bytes.len()
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Self { key })
    }

    /// Generate temporary MEK for development with appropriate warnings
    ///
    /// Generate a new key for development (never fails)
    fn generate_for_development() -> Self {
        Self::log_development_warnings();
        let key = crate::database::generate_encryption_key();
        Self::log_generated_key(&key);
        Self { key }
    }

    /// Log warning messages for development key generation
    fn log_development_warnings() {
        warn!("PIERRE_MASTER_ENCRYPTION_KEY not found in environment");
        warn!("Generating temporary MEK for development - NOT SECURE FOR PRODUCTION");
    }

    /// Log the generated key for development use
    fn log_generated_key(key: &[u8; 32]) {
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);
        warn!(
            "Generated MEK (save for production): PIERRE_MASTER_ENCRYPTION_KEY={}",
            encoded
        );
        warn!("Add this to your environment variables for production deployment");
    }

    /// Get the raw key bytes for encryption operations
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    /// Encrypt data with the MEK (used to encrypt DEK)
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::aead::generic_array::GenericArray;
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
        use rand::RngCore;

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&self.key));

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = GenericArray::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data with the MEK (used to decrypt DEK)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The encrypted data is too short to contain a nonce
    /// - Decryption fails due to invalid data or wrong key
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::aead::generic_array::GenericArray;
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};

        if encrypted_data.len() < 12 {
            return Err(anyhow!("Encrypted data too short"));
        }

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&self.key));

        // Extract nonce and ciphertext
        let nonce = GenericArray::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        // Decrypt the data
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }
}

impl DatabaseEncryptionKey {
    /// Create a new random DEK
    #[must_use]
    pub fn generate() -> Self {
        let key = crate::database::generate_encryption_key();
        Self { key }
    }

    /// Create DEK from existing key bytes
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key: bytes }
    }

    /// Get the raw key bytes for database encryption
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    /// Encrypt DEK using MEK for storage in database
    ///
    /// # Errors
    ///
    /// Returns an error if MEK encryption fails
    pub fn encrypt_with_mek(&self, mek: &MasterEncryptionKey) -> Result<Vec<u8>> {
        mek.encrypt(&self.key)
    }

    /// Decrypt DEK from database using MEK
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - MEK decryption fails
    /// - Decrypted data is not exactly 32 bytes
    pub fn decrypt_with_mek(encrypted_dek: &[u8], mek: &MasterEncryptionKey) -> Result<Self> {
        let decrypted_bytes = mek.decrypt(encrypted_dek)?;

        if decrypted_bytes.len() != 32 {
            return Err(anyhow!(
                "Decrypted DEK has invalid length: expected 32 bytes, got {}",
                decrypted_bytes.len()
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&decrypted_bytes);

        Ok(Self { key })
    }
}

/// Two-tier key management system
pub struct KeyManager {
    mek: MasterEncryptionKey,
    dek: DatabaseEncryptionKey,
}

impl KeyManager {
    /// Bootstrap initialization: Load MEK and generate temporary DEK for database initialization
    ///
    /// # Errors
    ///
    /// Returns an error if MEK loading fails
    pub fn bootstrap() -> Result<(Self, [u8; 32])> {
        info!("Bootstrapping two-tier key management system");

        // Load MEK from environment
        let mek = MasterEncryptionKey::load_or_generate()?;

        // Generate temporary DEK for database initialization
        let temp_dek = DatabaseEncryptionKey::generate();
        let database_key = *temp_dek.as_bytes();

        let manager = Self { mek, dek: temp_dek };

        info!("Bootstrap key management system initialized");

        Ok((manager, database_key))
    }

    /// Complete initialization after database is available
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operations fail
    /// - DEK encryption/decryption fails
    pub async fn complete_initialization(
        &mut self,
        database: &crate::database_plugins::factory::Database,
    ) -> Result<()> {
        info!("Completing two-tier key management initialization");

        // Try to load existing DEK from database
        if let Ok(encrypted_dek_base64) =
            database.get_system_secret("database_encryption_key").await
        {
            info!("Loading existing Database Encryption Key from database");

            // Decode from base64
            let encrypted_dek = base64::engine::general_purpose::STANDARD
                .decode(&encrypted_dek_base64)
                .map_err(|e| anyhow!("Invalid base64 encoding for stored DEK: {}", e))?;

            // Decrypt with MEK and replace temporary DEK
            self.dek = DatabaseEncryptionKey::decrypt_with_mek(&encrypted_dek, &self.mek)?;

            info!("Existing Database Encryption Key loaded successfully");
        } else {
            info!("No existing DEK found, storing current Database Encryption Key");

            // Encrypt current DEK with MEK
            let encrypted_dek = self.dek.encrypt_with_mek(&self.mek)?;

            // Store encrypted DEK in database
            let encrypted_dek_base64 =
                base64::engine::general_purpose::STANDARD.encode(&encrypted_dek);
            database
                .update_system_secret("database_encryption_key", &encrypted_dek_base64)
                .await?;

            info!("Database Encryption Key stored successfully");
        }

        info!("Two-tier key management system fully initialized");
        Ok(())
    }

    /// Initialize key manager with MEK from environment and DEK from database (for existing systems)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - MEK loading fails
    /// - Database operations fail
    /// - DEK encryption/decryption fails
    pub async fn initialize(database: &crate::database_plugins::factory::Database) -> Result<Self> {
        info!("Initializing two-tier key management system");

        // Load MEK from environment
        let mek = MasterEncryptionKey::load_or_generate()?;

        // Try to load existing DEK from database
        let dek = if let Ok(encrypted_dek_base64) =
            database.get_system_secret("database_encryption_key").await
        {
            info!("Loading existing Database Encryption Key from database");

            // Decode from base64
            let encrypted_dek = base64::engine::general_purpose::STANDARD
                .decode(&encrypted_dek_base64)
                .map_err(|e| anyhow!("Invalid base64 encoding for stored DEK: {}", e))?;

            // Decrypt with MEK
            DatabaseEncryptionKey::decrypt_with_mek(&encrypted_dek, &mek)?
        } else {
            info!("No existing DEK found, generating new Database Encryption Key");

            // Generate new DEK
            let dek = DatabaseEncryptionKey::generate();

            // Encrypt DEK with MEK
            let encrypted_dek = dek.encrypt_with_mek(&mek)?;

            // Store encrypted DEK in database
            let encrypted_dek_base64 =
                base64::engine::general_purpose::STANDARD.encode(&encrypted_dek);
            database
                .update_system_secret("database_encryption_key", &encrypted_dek_base64)
                .await?;

            info!("Generated and stored new Database Encryption Key");
            dek
        };

        info!("Two-tier key management system initialized successfully");

        Ok(Self { mek, dek })
    }

    /// Get the DEK for database operations (what we previously called "encryption key")
    #[must_use]
    pub const fn database_key(&self) -> &[u8; 32] {
        self.dek.as_bytes()
    }

    /// Get the MEK for key encryption operations
    #[must_use]
    pub const fn master_key(&self) -> &MasterEncryptionKey {
        &self.mek
    }

    /// Rotate the DEK (generate new one, encrypt with MEK, store in database)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - DEK encryption fails
    /// - Database storage fails
    pub async fn rotate_database_key(
        &mut self,
        database: &crate::database_plugins::factory::Database,
    ) -> Result<()> {
        info!("Rotating Database Encryption Key");

        // Generate new DEK
        self.dek = DatabaseEncryptionKey::generate();

        // Encrypt new DEK with MEK
        let encrypted_dek = self.dek.encrypt_with_mek(&self.mek)?;

        // Store encrypted DEK in database
        let encrypted_dek_base64 = base64::engine::general_purpose::STANDARD.encode(&encrypted_dek);
        database
            .update_system_secret("database_encryption_key", &encrypted_dek_base64)
            .await?;

        info!("Database Encryption Key rotated successfully");

        Ok(())
    }
}
