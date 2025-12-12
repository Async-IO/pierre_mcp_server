// ABOUTME: Two-tier key management system for secure database encryption and secret storage
// ABOUTME: Implements MEK (Master Encryption Key) from environment and DEK (Database Encryption Key) stored encrypted
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use base64::Engine;
use std::env;
use tracing::info;

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
    /// Load MEK from environment variable (required)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The `PIERRE_MASTER_ENCRYPTION_KEY` environment variable is not set
    /// - The environment variable contains invalid base64 encoding
    /// - The decoded key is not exactly 32 bytes
    pub fn load_or_generate() -> AppResult<Self> {
        env::var("PIERRE_MASTER_ENCRYPTION_KEY").map_or_else(
            |_| {
                Err(AppError::config(
                    "PIERRE_MASTER_ENCRYPTION_KEY environment variable is required.\n\n\
                     This key is used to encrypt sensitive data (OAuth tokens, admin secrets, etc.).\n\
                     Without a persistent key, encrypted data becomes unreadable after server restart.\n\n\
                     To generate a key, run:\n\
                     \x20\x20openssl rand -base64 32\n\n\
                     Then set it in your environment:\n\
                     \x20\x20export PIERRE_MASTER_ENCRYPTION_KEY=\"<your-generated-key>\"\n\n\
                     Or add it to your .env file.",
                ))
            },
            |encoded_key| Self::load_from_environment(&encoded_key),
        )
    }

    /// Load MEK from base64-encoded environment variable
    ///
    /// # Errors
    /// Returns error if decoding fails or key is wrong length
    fn load_from_environment(encoded_key: &str) -> AppResult<Self> {
        info!("Loading Master Encryption Key from environment variable");
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded_key)
            .map_err(|e| {
                AppError::config(format!(
                    "Invalid base64 encoding in PIERRE_MASTER_ENCRYPTION_KEY: {e}"
                ))
            })?;

        if key_bytes.len() != 32 {
            return Err(AppError::config(format!(
                "Master encryption key must be exactly 32 bytes, got {} bytes",
                key_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Self { key })
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
    pub fn encrypt(&self, plaintext: &[u8]) -> AppResult<Vec<u8>> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        use rand::RngCore;

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::internal(format!("Invalid key length: {e}")))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AppError::internal(format!("Encryption failed: {e}")))?;

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
    pub fn decrypt(&self, encrypted_data: &[u8]) -> AppResult<Vec<u8>> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

        if encrypted_data.len() < 12 {
            return Err(AppError::invalid_input("Encrypted data too short"));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::internal(format!("Invalid key length: {e}")))?;

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        // Decrypt the data
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::internal(format!("Decryption failed: {e}")))?;

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
    pub fn encrypt_with_mek(&self, mek: &MasterEncryptionKey) -> AppResult<Vec<u8>> {
        mek.encrypt(&self.key)
    }

    /// Decrypt DEK from database using MEK
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - MEK decryption fails
    /// - Decrypted data is not exactly 32 bytes
    pub fn decrypt_with_mek(encrypted_dek: &[u8], mek: &MasterEncryptionKey) -> AppResult<Self> {
        let decrypted_bytes = mek.decrypt(encrypted_dek)?;

        if decrypted_bytes.len() != 32 {
            return Err(AppError::internal(format!(
                "Decrypted DEK has invalid length: expected 32 bytes, got {}",
                decrypted_bytes.len()
            )));
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
    pub fn bootstrap() -> AppResult<(Self, [u8; 32])> {
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

    /// Decode base64-encoded encrypted DEK
    fn decode_encrypted_dek(encrypted_dek_base64: &str) -> AppResult<Vec<u8>> {
        base64::engine::general_purpose::STANDARD
            .decode(encrypted_dek_base64)
            .map_err(|e| AppError::internal(format!("Invalid base64 encoding for stored DEK: {e}")))
    }

    /// Store encrypted DEK in database
    async fn store_dek(
        database: &crate::database_plugins::factory::Database,
        encrypted_dek: &[u8],
    ) -> AppResult<()> {
        let encrypted_dek_base64 = base64::engine::general_purpose::STANDARD.encode(encrypted_dek);
        database
            .update_system_secret("database_encryption_key", &encrypted_dek_base64)
            .await
    }

    /// Complete initialization after database is available
    ///
    /// This loads the existing DEK from the database (if any) and updates the database's
    /// encryption key to use the correct DEK. This is necessary because during bootstrap,
    /// a temporary DEK is used to initialize the database, but we need to use the actual
    /// stored DEK for encrypted data operations.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database operations fail
    /// - DEK encryption/decryption fails
    pub async fn complete_initialization(
        &mut self,
        database: &mut crate::database_plugins::factory::Database,
    ) -> AppResult<()> {
        info!("Completing two-tier key management initialization");

        if let Ok(encrypted_dek_base64) =
            database.get_system_secret("database_encryption_key").await
        {
            self.load_existing_dek(&encrypted_dek_base64)?;
            // Update the database's encryption key to use the loaded DEK
            // This is critical: the database was initialized with a temp key,
            // now we update it with the real key loaded from the database
            database.update_encryption_key(self.dek.as_bytes().to_vec());
            info!("Database encryption key updated to use loaded DEK");
        } else {
            self.store_new_dek(database).await?;
        }

        info!("Two-tier key management system fully initialized");
        Ok(())
    }

    fn load_existing_dek(&mut self, encrypted_dek_base64: &str) -> AppResult<()> {
        info!("Loading existing Database Encryption Key from database");
        let encrypted_dek = Self::decode_encrypted_dek(encrypted_dek_base64)?;
        self.dek =
            DatabaseEncryptionKey::decrypt_with_mek(&encrypted_dek, &self.mek).map_err(|e| {
                // Check if this is a decryption failure (key mismatch)
                if e.message.contains("Decryption failed") {
                    let database_url =
                        env::var("DATABASE_URL").unwrap_or_else(|_| "unknown".to_owned());
                    AppError::encryption_key_mismatch(&database_url)
                } else {
                    e
                }
            })?;
        info!("Existing Database Encryption Key loaded successfully");
        Ok(())
    }

    async fn store_new_dek(
        &self,
        database: &crate::database_plugins::factory::Database,
    ) -> AppResult<()> {
        info!("No existing DEK found, storing current Database Encryption Key");
        let encrypted_dek = self.dek.encrypt_with_mek(&self.mek)?;
        Self::store_dek(database, &encrypted_dek).await?;
        info!("Database Encryption Key stored successfully");
        Ok(())
    }

    /// Load DEK from database or generate new one
    async fn load_or_generate_dek(
        database: &crate::database_plugins::factory::Database,
        mek: &MasterEncryptionKey,
    ) -> AppResult<DatabaseEncryptionKey> {
        if let Ok(encrypted_dek_base64) =
            database.get_system_secret("database_encryption_key").await
        {
            info!("Loading existing Database Encryption Key from database");
            let encrypted_dek = Self::decode_encrypted_dek(&encrypted_dek_base64)?;
            return DatabaseEncryptionKey::decrypt_with_mek(&encrypted_dek, mek).map_err(|e| {
                // Check if this is a decryption failure (key mismatch)
                if e.message.contains("Decryption failed") {
                    let database_url =
                        env::var("DATABASE_URL").unwrap_or_else(|_| "unknown".to_owned());
                    AppError::encryption_key_mismatch(&database_url)
                } else {
                    e
                }
            });
        }

        info!("No existing DEK found, generating new Database Encryption Key");
        let dek = DatabaseEncryptionKey::generate();
        let encrypted_dek = dek.encrypt_with_mek(mek)?;
        Self::store_dek(database, &encrypted_dek).await?;
        info!("Generated and stored new Database Encryption Key");
        Ok(dek)
    }

    /// Initialize key manager with MEK from environment and DEK from database (for existing systems)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - MEK loading fails
    /// - Database operations fail
    /// - DEK encryption/decryption fails
    pub async fn initialize(
        database: &crate::database_plugins::factory::Database,
    ) -> AppResult<Self> {
        info!("Initializing two-tier key management system");

        let mek = MasterEncryptionKey::load_or_generate()?;
        let dek = Self::load_or_generate_dek(database, &mek).await?;

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
    ) -> AppResult<()> {
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
