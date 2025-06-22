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
    pub fn create_public_key_info(keypair: &A2AKeypair) -> A2APublicKey {
        A2APublicKey {
            public_key: keypair.public_key.clone(),
            key_type: "ed25519".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Sign data with private key
    pub fn sign_data(private_key: &str, data: &[u8]) -> Result<String> {
        let secret_bytes = general_purpose::STANDARD.decode(private_key)?;
        let signing_key = SigningKey::from_bytes(secret_bytes.as_slice().try_into()?);

        let signature = signing_key.sign(data);
        Ok(general_purpose::STANDARD.encode(signature.to_bytes()))
    }

    /// Verify signature with public key
    pub fn verify_signature(public_key: &str, data: &[u8], signature: &str) -> Result<bool> {
        let public_bytes = general_purpose::STANDARD.decode(public_key)?;
        let verifying_key = VerifyingKey::from_bytes(public_bytes.as_slice().try_into()?)?;

        let sig_bytes = general_purpose::STANDARD.decode(signature)?;
        let signature = Signature::from_bytes(sig_bytes.as_slice().try_into()?);

        match verifying_key.verify(data, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Generate a challenge for client verification
    pub fn generate_challenge() -> String {
        use rand::Rng;
        let mut rng = OsRng;
        let challenge: [u8; 32] = rng.gen();
        general_purpose::STANDARD.encode(challenge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = A2AKeyManager::generate_keypair().unwrap();

        // Verify the keys are base64 encoded
        assert!(general_purpose::STANDARD
            .decode(&keypair.public_key)
            .is_ok());
        assert!(general_purpose::STANDARD
            .decode(&keypair.private_key)
            .is_ok());

        // Verify public key info creation
        let public_info = A2AKeyManager::create_public_key_info(&keypair);
        assert_eq!(public_info.key_type, "ed25519");
        assert_eq!(public_info.public_key, keypair.public_key);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = A2AKeyManager::generate_keypair().unwrap();
        let test_data = b"Hello, A2A authentication!";

        // Sign data
        let signature = A2AKeyManager::sign_data(&keypair.private_key, test_data).unwrap();

        // Verify signature
        let is_valid =
            A2AKeyManager::verify_signature(&keypair.public_key, test_data, &signature).unwrap();
        assert!(is_valid);

        // Verify with wrong data should fail
        let wrong_data = b"Wrong data";
        let is_invalid =
            A2AKeyManager::verify_signature(&keypair.public_key, wrong_data, &signature).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_challenge_generation() {
        let challenge1 = A2AKeyManager::generate_challenge();
        let challenge2 = A2AKeyManager::generate_challenge();

        // Challenges should be different
        assert_ne!(challenge1, challenge2);

        // Should be valid base64
        assert!(general_purpose::STANDARD.decode(&challenge1).is_ok());
        assert!(general_purpose::STANDARD.decode(&challenge2).is_ok());
    }
}
