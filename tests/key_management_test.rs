// ABOUTME: Tests for key management functionality including MEK/DEK encryption and decryption
// ABOUTME: Validates master encryption key operations and database encryption key handling

use pierre_mcp_server::key_management::{DatabaseEncryptionKey, MasterEncryptionKey};

#[test]
fn test_mek_encrypt_decrypt() {
    let mek = MasterEncryptionKey::from_bytes([1u8; 32]);

    let data = b"test data to encrypt";
    let encrypted = mek.encrypt(data).unwrap();
    let decrypted = mek.decrypt(&encrypted).unwrap();

    assert_eq!(data.as_slice(), decrypted.as_slice());
}

#[test]
fn test_dek_encrypt_with_mek() {
    let mek = MasterEncryptionKey::from_bytes([1u8; 32]);

    let dek = DatabaseEncryptionKey::generate();
    let original_key = *dek.as_bytes();

    // Encrypt DEK with MEK
    let encrypted_dek = dek.encrypt_with_mek(&mek).unwrap();

    // Decrypt DEK with MEK
    let restored_dek = DatabaseEncryptionKey::decrypt_with_mek(&encrypted_dek, &mek).unwrap();

    assert_eq!(original_key, *restored_dek.as_bytes());
}
