// ABOUTME: Unit tests for crypto keys functionality
// ABOUTME: Validates crypto keys behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use base64::{engine::general_purpose, Engine};
use pierre_mcp_server::crypto::keys::A2AKeyManager;

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
