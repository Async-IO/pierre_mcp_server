// ABOUTME: Cryptographic key management and encryption utilities
// ABOUTME: Provides secure key generation, storage, and data encryption/decryption
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Cryptographic key management for A2A clients

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Ed25519 keypair for A2A client authentication
#[derive(Debug, Clone)]
pub struct A2AKeypair {
    pub public_key: String,  // Base64 encoded
    pub private_key: String, // Base64 encoded (stored securely)
}

/// Public key information for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APublicKey {
    pub public_key: String, // Base64 encoded
    pub key_type: String,   // "ed25519"
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Key generation and management for A2A clients
pub struct A2AKeyManager;

impl A2AKeyManager {
    /// Generate a new Ed25519 keypair for A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if key generation fails
    pub fn generate_keypair() -> Result<A2AKeypair> {
        use rand::RngCore;

        let mut rng = OsRng;
        let mut secret_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        let public_key = general_purpose::STANDARD.encode(verifying_key.as_bytes());
        let private_key = general_purpose::STANDARD.encode(signing_key.as_bytes());

        Ok(A2AKeypair {
            public_key,
            private_key,
        })
    }

    /// Create public key info from keypair
    #[must_use]
    pub fn create_public_key_info(keypair: &A2AKeypair) -> A2APublicKey {
        A2APublicKey {
            public_key: keypair.public_key.clone(),
            key_type: "ed25519".into(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Sign data with private key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Private key decoding fails
    /// - Key format is invalid
    pub fn sign_data(private_key: &str, data: &[u8]) -> Result<String> {
        let secret_bytes = general_purpose::STANDARD.decode(private_key)?;
        let signing_key = SigningKey::from_bytes(secret_bytes.as_slice().try_into()?);

        let signature = signing_key.sign(data);
        Ok(general_purpose::STANDARD.encode(signature.to_bytes()))
    }

    /// Verify signature with public key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Public key or signature decoding fails
    /// - Key format is invalid
    pub fn verify_signature(public_key: &str, data: &[u8], signature: &str) -> Result<bool> {
        let public_bytes = general_purpose::STANDARD.decode(public_key)?;
        let verifying_key = VerifyingKey::from_bytes(public_bytes.as_slice().try_into()?)?;

        let sig_bytes = general_purpose::STANDARD.decode(signature)?;
        let signature = Signature::from_bytes(sig_bytes.as_slice().try_into()?);

        match verifying_key.verify(data, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Generate a challenge for client verification
    #[must_use]
    pub fn generate_challenge() -> String {
        use rand::Rng;
        let mut rng = OsRng;
        let challenge: [u8; 32] = rng.gen();
        general_purpose::STANDARD.encode(challenge)
    }
}
