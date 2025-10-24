// ABOUTME: JWKS (JSON Web Key Set) endpoint implementation for asymmetric JWT validation
// ABOUTME: Provides RS256/ES256 key generation, rotation, and public key distribution via JWKS endpoint
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright (c) 2025 Async-IO.org

//! JWKS (JSON Web Key Set) Management
//!
//! This module provides:
//! - RSA key pair generation for RS256 JWT signing
//! - JWKS JSON format for public key distribution
//! - Automatic key rotation with configurable intervals
//! - Key versioning for rolling key updates
//!
//! ## Security Model
//!
//! - Private keys never leave the server
//! - Public keys distributed via `/.well-known/jwks.json`
//! - Multiple keys supported for graceful rotation
//! - Old keys retained during rotation window
//!
//! ## Example
//!
//! ```rust,no_run
//! use pierre_mcp_server::admin::jwks::JwksManager;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut manager = JwksManager::new();
//!
//! // Generate initial RSA key pair
//! manager.generate_rsa_key_pair("key_2025_01")?;
//!
//! // Get JWKS for public distribution
//! let jwks_json = manager.get_jwks_json()?;
//!
//! // Rotate keys (keeps old keys for validation)
//! manager.rotate_keys()?;
//! # Ok(())
//! # }
//! ```

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey};
use rsa::{
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey},
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RSA key size in bits for RS256 (2048 bits minimum, 4096 bits recommended)
const RSA_KEY_SIZE: usize = 4096;

/// Key rotation interval in days
const KEY_ROTATION_DAYS: i64 = 90;

/// Number of historical keys to retain for validation
const MAX_HISTORICAL_KEYS: usize = 3;

/// JWK (JSON Web Key) representation for JWKS endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKey {
    /// Key type (always "RSA" for RS256)
    pub kty: String,
    /// Public key use (always "sig" for signature)
    #[serde(rename = "use")]
    pub key_use: String,
    /// Key ID for rotation tracking
    pub kid: String,
    /// Algorithm (RS256)
    pub alg: String,
    /// RSA modulus (base64url encoded)
    pub n: String,
    /// RSA exponent (base64url encoded)
    pub e: String,
}

/// JWKS (JSON Web Key Set) container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKeySet {
    /// Array of public keys
    pub keys: Vec<JsonWebKey>,
}

/// RSA key pair with metadata
#[derive(Clone)]
pub struct RsaKeyPair {
    /// Unique key identifier
    pub kid: String,
    /// Private key for signing
    pub private_key: RsaPrivateKey,
    /// Public key for verification
    pub public_key: RsaPublicKey,
    /// Key creation timestamp
    pub created_at: DateTime<Utc>,
    /// Whether this is the currently active signing key
    pub is_active: bool,
}

impl RsaKeyPair {
    /// Generate new RSA key pair with production-grade 4096-bit key size
    ///
    /// # Errors
    /// Returns error if key generation fails
    pub fn generate(kid: &str) -> Result<Self> {
        Self::generate_with_key_size(kid, RSA_KEY_SIZE)
    }

    /// Generate RSA key pair with configurable key size
    ///
    /// Use 2048 bits for faster test execution, 4096 bits for production security.
    ///
    /// # Errors
    /// Returns error if key generation fails
    pub fn generate_with_key_size(kid: &str, key_size_bits: usize) -> Result<Self> {
        use rand::rngs::OsRng;

        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, key_size_bits)
            .map_err(|e| anyhow!("Failed to generate RSA private key: {e}"))?;

        let public_key = RsaPublicKey::from(&private_key);

        Ok(Self {
            kid: kid.to_string(),
            private_key,
            public_key,
            created_at: Utc::now(),
            is_active: true,
        })
    }

    /// Convert public key to JWK format
    ///
    /// # Errors
    /// Returns error if key serialization fails
    pub fn to_jwk(&self) -> Result<JsonWebKey> {
        // Extract RSA modulus and exponent
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use rsa::traits::PublicKeyParts;

        let n = self.public_key.n();
        let e = self.public_key.e();

        // Convert to base64url
        let n_bytes = n.to_bytes_be();
        let e_bytes = e.to_bytes_be();

        let n_b64 = URL_SAFE_NO_PAD.encode(&n_bytes);
        let e_b64 = URL_SAFE_NO_PAD.encode(&e_bytes);

        Ok(JsonWebKey {
            kty: "RSA".to_string(),
            key_use: "sig".to_string(),
            kid: self.kid.clone(),
            alg: "RS256".to_string(),
            n: n_b64,
            e: e_b64,
        })
    }

    /// Export private key as PEM
    ///
    /// # Errors
    /// Returns error if PEM encoding fails
    pub fn export_private_key_pem(&self) -> Result<String> {
        self.private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .map(|pem| pem.to_string())
            .map_err(|e| anyhow!("Failed to export private key as PEM: {e}"))
    }

    /// Export public key as PEM
    ///
    /// # Errors
    /// Returns error if PEM encoding fails
    pub fn export_public_key_pem(&self) -> Result<String> {
        self.public_key
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .map_err(|e| anyhow!("Failed to export public key as PEM: {e}"))
    }

    /// Import private key from PEM
    ///
    /// # Errors
    /// Returns error if PEM parsing fails
    pub fn import_private_key_pem(kid: &str, pem: &str) -> Result<Self> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(pem)
            .map_err(|e| anyhow!("Failed to parse private key PEM: {e}"))?;

        let public_key = RsaPublicKey::from(&private_key);

        Ok(Self {
            kid: kid.to_string(),
            private_key,
            public_key,
            created_at: Utc::now(),
            is_active: false, // Imported keys start inactive
        })
    }

    /// Get encoding key for JWT signing
    ///
    /// # Panics
    /// Panics if PEM export or encoding key creation fails (should never happen with valid RSA keys)
    #[must_use]
    pub fn encoding_key(&self) -> EncodingKey {
        // Export to PEM and create encoding key
        let pem = self
            .export_private_key_pem()
            .expect("Failed to export private key");
        EncodingKey::from_rsa_pem(pem.as_bytes()).expect("Failed to create encoding key")
    }

    /// Get decoding key for JWT verification
    ///
    /// # Panics
    /// Panics if PEM export or decoding key creation fails (should never happen with valid RSA keys)
    #[must_use]
    pub fn decoding_key(&self) -> DecodingKey {
        // Export to PEM and create decoding key
        let pem = self
            .export_public_key_pem()
            .expect("Failed to export public key");
        DecodingKey::from_rsa_pem(pem.as_bytes()).expect("Failed to create decoding key")
    }
}

/// JWKS manager for key lifecycle management
pub struct JwksManager {
    /// All keys (active and historical)
    keys: HashMap<String, RsaKeyPair>,
    /// Currently active key ID for signing
    active_key_id: Option<String>,
}

impl JwksManager {
    /// Create new JWKS manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            active_key_id: None,
        }
    }

    /// Generate and register new RSA key pair with production-grade 4096-bit key size
    ///
    /// # Errors
    /// Returns error if key generation or registration fails
    pub fn generate_rsa_key_pair(&mut self, kid: &str) -> Result<()> {
        self.generate_rsa_key_pair_with_size(kid, RSA_KEY_SIZE)
    }

    /// Generate and register RSA key pair with configurable key size for testing
    ///
    /// Use 2048 bits for faster test execution, 4096 bits for production security.
    ///
    /// # Errors
    /// Returns error if key generation or registration fails
    pub fn generate_rsa_key_pair_with_size(
        &mut self,
        kid: &str,
        key_size_bits: usize,
    ) -> Result<()> {
        let key_pair = RsaKeyPair::generate_with_key_size(kid, key_size_bits)?;

        // Deactivate previous active key if exists
        if let Some(prev_active_kid) = &self.active_key_id {
            if let Some(prev_key) = self.keys.get_mut(prev_active_kid) {
                prev_key.is_active = false;
            }
        }

        // Set new key as active
        self.active_key_id = Some(kid.to_string());
        self.keys.insert(kid.to_string(), key_pair);

        Ok(())
    }

    /// Get active signing key
    ///
    /// # Errors
    /// Returns error if no active key exists
    pub fn get_active_key(&self) -> Result<&RsaKeyPair> {
        let kid = self
            .active_key_id
            .as_ref()
            .ok_or_else(|| anyhow!("No active signing key"))?;

        self.keys
            .get(kid)
            .ok_or_else(|| anyhow!("Active key not found: {kid}"))
    }

    /// Get key by ID
    #[must_use]
    pub fn get_key(&self, kid: &str) -> Option<&RsaKeyPair> {
        self.keys.get(kid)
    }

    /// Get all keys (for validation)
    #[must_use]
    pub fn get_all_keys(&self) -> Vec<&RsaKeyPair> {
        self.keys.values().collect()
    }

    /// Register an existing RSA key pair from PEM format (for database loading)
    ///
    /// # Errors
    /// Returns error if key import or registration fails
    pub fn register_keypair_from_pem(
        &mut self,
        kid: &str,
        private_key_pem: &str,
        created_at: DateTime<Utc>,
        is_active: bool,
    ) -> Result<()> {
        let mut key_pair = RsaKeyPair::import_private_key_pem(kid, private_key_pem)?;
        key_pair.created_at = created_at;
        key_pair.is_active = is_active;

        // If this key is marked active, deactivate current active key
        if is_active {
            if let Some(prev_active_kid) = &self.active_key_id {
                if let Some(prev_key) = self.keys.get_mut(prev_active_kid) {
                    prev_key.is_active = false;
                }
            }
            self.active_key_id = Some(kid.to_string());
        }

        self.keys.insert(kid.to_string(), key_pair);
        Ok(())
    }

    /// Load keys from database tuples
    ///
    /// # Errors
    /// Returns error if key import fails
    pub fn load_keys_from_database(
        &mut self,
        keypairs: Vec<(String, String, String, DateTime<Utc>, bool)>,
    ) -> Result<()> {
        for (kid, private_key_pem, _public_key_pem, created_at, is_active) in keypairs {
            self.register_keypair_from_pem(&kid, &private_key_pem, created_at, is_active)?;
        }
        Ok(())
    }

    /// Generate JWKS JSON for public key distribution
    ///
    /// # Errors
    /// Returns error if JWK serialization fails
    pub fn get_jwks_json(&self) -> Result<String> {
        let jwks = self.get_jwks()?;
        serde_json::to_string_pretty(&jwks).map_err(|e| anyhow!("Failed to serialize JWKS: {e}"))
    }

    /// Get JWKS structure
    ///
    /// # Errors
    /// Returns error if JWK conversion fails
    pub fn get_jwks(&self) -> Result<JsonWebKeySet> {
        let mut keys = Vec::new();

        for key_pair in self.keys.values() {
            keys.push(key_pair.to_jwk()?);
        }

        Ok(JsonWebKeySet { keys })
    }

    /// Rotate keys - generate new key and mark old key as historical
    ///
    /// # Errors
    /// Returns error if key generation fails
    pub fn rotate_keys(&mut self) -> Result<String> {
        self.rotate_keys_with_size(RSA_KEY_SIZE)
    }

    /// Rotate keys with custom key size - generate new key and mark old key as historical
    ///
    /// # Errors
    /// Returns error if key generation fails
    pub fn rotate_keys_with_size(&mut self, key_size_bits: usize) -> Result<String> {
        let new_kid = format!("key_{}", Utc::now().format("%Y%m%d_%H%M%S"));

        self.generate_rsa_key_pair_with_size(&new_kid, key_size_bits)?;

        // Clean up old keys (keep only MAX_HISTORICAL_KEYS)
        self.cleanup_old_keys();

        Ok(new_kid)
    }

    /// Remove old keys beyond retention limit
    fn cleanup_old_keys(&mut self) {
        if self.keys.len() <= MAX_HISTORICAL_KEYS {
            return;
        }

        // Sort keys by creation time, with kid as tiebreaker for deterministic behavior
        // This ensures consistent ordering on systems with low timestamp resolution (Windows)
        let mut sorted_keys: Vec<_> = self
            .keys
            .iter()
            .map(|(kid, key)| (kid.clone(), key.created_at))
            .collect();

        sorted_keys.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        // Remove oldest keys beyond limit
        let to_remove = sorted_keys.len() - MAX_HISTORICAL_KEYS;
        for (kid, _) in sorted_keys.iter().take(to_remove) {
            if Some(kid) != self.active_key_id.as_ref() {
                self.keys.remove(kid);
            }
        }
    }

    /// Check if key rotation is needed
    #[must_use]
    pub fn should_rotate_keys(&self) -> bool {
        if let Some(active_kid) = &self.active_key_id {
            if let Some(active_key) = self.keys.get(active_kid) {
                let age = Utc::now() - active_key.created_at;
                return age.num_days() >= KEY_ROTATION_DAYS;
            }
        }
        true // Rotate if no active key
    }

    /// Sign admin token claims using RS256
    ///
    /// # Errors
    /// Returns error if no active key exists or signing fails
    pub fn sign_admin_token<T: Serialize>(&self, claims: &T) -> Result<String> {
        use jsonwebtoken::{encode, Header};

        let active_key = self.get_active_key()?;

        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(active_key.kid.clone());

        let encoding_key = active_key.encoding_key();

        encode(&header, claims, &encoding_key)
            .map_err(|e| anyhow!("Failed to encode RS256 admin JWT: {e}"))
    }

    /// Verify admin token and extract claims
    ///
    /// # Errors
    /// Returns error if token verification fails or claims cannot be decoded
    pub fn verify_admin_token<T: for<'de> Deserialize<'de>>(&self, token: &str) -> Result<T> {
        use jsonwebtoken::{decode, decode_header, Validation};

        // Extract kid from header
        let header =
            decode_header(token).map_err(|e| anyhow!("Failed to decode JWT header: {e}"))?;

        let kid = header
            .kid
            .ok_or_else(|| anyhow!("JWT header missing kid"))?;

        // Get corresponding key
        let key_pair = self
            .get_key(&kid)
            .ok_or_else(|| anyhow!("Unknown key ID: {kid}"))?;

        let decoding_key = key_pair.decoding_key();

        // Set up validation
        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[crate::constants::service_names::ADMIN_API]);
        validation.set_issuer(&[crate::constants::service_names::PIERRE_MCP_SERVER]);

        // Verify and decode
        let token_data = decode::<T>(token, &decoding_key, &validation)
            .map_err(|e| anyhow!("Failed to verify RS256 admin JWT: {e}"))?;

        Ok(token_data.claims)
    }
}

impl Default for JwksManager {
    fn default() -> Self {
        Self::new()
    }
}
