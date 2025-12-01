<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 5: Cryptographic Key Management

> **Learning Objectives**: Master Pierre's two-tier key management system (MEK + DEK), understand RSA key generation for JWT signing, learn JWKS management, and use the `zeroize` crate for secure memory cleanup.
>
> **Prerequisites**: Chapters 1-4, basic understanding of encryption concepts
>
> **Estimated Time**: 2-3 hours

---

## Introduction

Cryptography in production requires careful key management. Pierre implements a **two-tier key system**:

1. **MEK (Master Encryption Key)** - Tier 1, from environment
2. **DEK (Database Encryption Key)** - Tier 2, encrypted with MEK

Plus **RSA key pairs** for JWT RS256 signing and **Ed25519** for A2A authentication.

This chapter teaches secure key generation, storage, and the Rust patterns that prevent key leakage.

---

## Two-Tier Key Management System

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│              Two-Tier Key Management                    │
└─────────────────────────────────────────────────────────┘

Tier 1: MEK (Master Encryption Key)
├─ Source: PIERRE_MASTER_ENCRYPTION_KEY environment variable
├─ Size: 32 bytes (256 bits)
├─ Usage: Encrypts DEK before storage
└─ Lifetime: Never stored in database

         ↓ Encrypts

Tier 2: DEK (Database Encryption Key)
├─ Source: Generated randomly, stored encrypted
├─ Size: 32 bytes (256 bits)
├─ Usage: Encrypts sensitive database fields (tokens, secrets)
└─ Storage: Database, encrypted with MEK

         ↓ Encrypts

User Data
├─ OAuth tokens
├─ API keys
└─ Sensitive user information
```

**Why two tiers?**
1. **MEK rotation** doesn't require re-encrypting all data
2. **DEK** can be rotated independently
3. **Separation of concerns**: MEK from ops, DEK from code
4. **Key hierarchy**: Industry standard (AWS KMS, GCP KMS use similar)

**Reference**: [AWS KMS Concepts](https://docs.aws.amazon.com/kms/latest/developerguide/concepts.html#master_keys)

---

## Master Encryption Key MEK

**Source**: `src/key_management.rs:14-188`

### MEK Structure

```rust
/// Master Encryption Key (MEK) - Tier 1
pub struct MasterEncryptionKey {
    key: [u8; 32],  // Fixed-size array (256 bits)
}
```

**Rust Idioms Explained**:

1. **Fixed-size array `[u8; 32]`**
   - Exactly 32 bytes, known at compile time
   - Stack-allocated (no heap)
   - Implements `Copy` (cheap to pass around)
   - More secure than `Vec<u8>` (can't be resized accidentally)

2. **Private field** - `key` is private
   - Can't access directly from outside module
   - Forces use of safe accessor methods
   - Prevents accidental copying

### Loading MEK from Environment

**Source**: `src/key_management.rs:40-75`

```rust
impl MasterEncryptionKey {
    /// Load MEK from environment variable or generate for development
    pub fn load_or_generate() -> Result<Self> {
        // Try to load from environment first
        if let Ok(encoded_key) = env::var("PIERRE_MASTER_ENCRYPTION_KEY") {
            return Self::load_from_environment(&encoded_key);
        }

        // Development mode: generate and log warning
        Ok(Self::generate_for_development())
    }

    fn load_from_environment(encoded_key: &str) -> Result<Self> {
        info!("Loading Master Encryption Key from environment variable");

        // Decode base64
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded_key)
            .map_err(|e| AppError::config(
                format!("Invalid base64 encoding in PIERRE_MASTER_ENCRYPTION_KEY: {e}")
            ))?;

        // Validate length
        if key_bytes.len() != 32 {
            return Err(AppError::config(format!(
                "Master encryption key must be exactly 32 bytes, got {} bytes",
                key_bytes.len()
            )).into());
        }

        // Copy into fixed-size array
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Self { key })
    }
}
```

**Rust Idioms Explained**:

1. **`.copy_from_slice()` method**
   - Copies `Vec<u8>` into `[u8; 32]`
   - Panics if lengths don't match (we validate first)
   - More efficient than looping

2. **Early return pattern**
   - `if let Ok(...) { return ... }`
   - Avoids deep nesting
   - Clear error handling path

3. **Error context with `.map_err()`**
   - Wraps underlying error with helpful message
   - User sees "Invalid base64" not "DecodeError"

### MEK encryption/decryption

**Source**: `src/key_management.rs:130-187`

```rust
impl MasterEncryptionKey {
    /// Encrypt data with the MEK (used to encrypt DEK)
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        use rand::RngCore;

        // Create AES-GCM cipher
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::internal(format!("Invalid key length: {e}")))?;

        // Generate random nonce (12 bytes for AES-GCM)
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AppError::internal(format!("Encryption failed: {e}")))?;

        // Prepend nonce to ciphertext (needed for decryption)
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

        if encrypted_data.len() < 12 {
            return Err(AppError::invalid_input("Encrypted data too short").into());
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
```

**Cryptography Explained**:

1. **AES-256-GCM** - Authenticated encryption
   - **AES-256**: Symmetric encryption (256-bit key)
   - **GCM**: Galois/Counter Mode (authenticated, prevents tampering)
   - Industry standard (used by TLS, IPsec, etc.)

2. **Nonce (Number Once)**
   - 12 bytes random value
   - Must be unique for each encryption
   - Stored alongside ciphertext
   - Prevents identical plaintexts producing same ciphertext

3. **Prepending nonce to ciphertext**
   - Common pattern: `[nonce || ciphertext]`
   - Decryption extracts first 12 bytes
   - Alternative: separate storage (more complex)

**Reference**: [NIST AES-GCM Spec](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)

### Development Key Generation

**Source**: `src/key_management.rs:77-122`

```rust
fn generate_for_development() -> Self {
    Self::log_development_warnings();
    let key = crate::database::generate_encryption_key();
    Self::log_generated_key(&key);
    Self { key }
}

fn log_generated_key(key: &[u8; 32]) {
    // Only log MEK in debug builds with explicit environment flag
    #[cfg(debug_assertions)]
    {
        if std::env::var("PIERRE_LOG_MEK").unwrap_or_default() == "true" {
            let encoded = base64::engine::general_purpose::STANDARD.encode(key);
            warn!(
                "Generated MEK (save for production): PIERRE_MASTER_ENCRYPTION_KEY={}",
                encoded
            );
        } else {
            warn!("Generated MEK - Set PIERRE_LOG_MEK=true to display");
        }
    }

    // In release builds, never log the key
    #[cfg(not(debug_assertions))]
    {
        let _ = key; // Silence unused warning
        warn!("Generated temporary MEK - Use env var for production");
    }
}
```

**Rust Idioms Explained**:

1. **`#[cfg(debug_assertions)]`** - Conditional compilation
   - Code only compiled in debug builds
   - Production builds (release) exclude this code entirely
   - Prevents accidental key logging in production

2. **`#[cfg(not(debug_assertions))]`** - Release builds
   - Opposite of debug
   - `let _ = key;` consumes variable (prevents unused warning)

3. **Double protection**
   - Debug build + `PIERRE_LOG_MEK=true` required
   - Defense in depth: can't accidentally log keys

---

## RSA Keys for JWT Signing

Pierre uses **RS256** (RSA with SHA-256) for JWT signing, requiring RSA key pairs.

**Source**: `src/admin/jwks.rs:87-133`

### RSA Key Pair Structure

```rust
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
```

**Fields explained**:
- **`kid` (Key ID)**: Identifies key in JWKS (e.g., "key_2025_01")
- **`private_key`**: Used to sign JWTs (kept secret)
- **`public_key`**: Distributed via JWKS (anyone can verify)
- **`is_active`**: Only one active key at a time

### Generating RSA Keys

**Source**: `src/admin/jwks.rs:103-133`

```rust
impl RsaKeyPair {
    /// Generate RSA key pair with configurable key size
    pub fn generate_with_key_size(kid: &str, key_size_bits: usize) -> Result<Self> {
        use rand::rngs::OsRng;

        let mut rng = OsRng;  // Cryptographically secure RNG
        let private_key = RsaPrivateKey::new(&mut rng, key_size_bits)
            .map_err(|e| AppError::internal(
                format!("Failed to generate RSA private key: {e}")
            ))?;

        let public_key = RsaPublicKey::from(&private_key);

        Ok(Self {
            kid: kid.to_owned(),
            private_key,
            public_key,
            created_at: Utc::now(),
            is_active: true,
        })
    }
}
```

**Rust Idioms Explained**:

1. **`OsRng` - Operating system RNG**
   - Cryptographically secure random number generator
   - Uses OS entropy source (Linux: `/dev/urandom`, Windows: BCrypt)
   - **Never** use `rand::thread_rng()` for cryptographic keys

2. **`RsaPublicKey::from(&private_key)`**
   - Public key is mathematically derived from private key
   - No randomness needed
   - Implements `From` trait

3. **Key sizes**:
   - **2048 bits**: Minimum, fast generation (~250ms)
   - **4096 bits**: Recommended, slow generation (~10s)
   - Pierre uses 4096 in production, 2048 in tests

**Reference**: [RSA Key Sizes](https://www.keylength.com/en/4/)

### JWKS JSON Web Key Set)

**Source**: `src/admin/jwks.rs:62-85`

```rust
/// JWK (JSON Web Key) representation
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
    pub keys: Vec<JsonWebKey>,
}
```

**JWKS format example**:

```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "key_2025_01",
      "alg": "RS256",
      "n": "xGOr-H...(base64url)...",
      "e": "AQAB"
    }
  ]
}
```

**Fields explained**:
- **`kty`**: Key type (RSA, EC, oct)
- **`use`**: Key usage (sig=signature, enc=encryption)
- **`kid`**: Key identifier (for rotation)
- **`alg`**: Algorithm (RS256, ES256, etc.)
- **`n`**: RSA modulus (public)
- **`e`**: RSA exponent (usually 65537 = "AQAB" in base64url)

**Reference**: [RFC 7517 - JSON Web Key](https://tools.ietf.org/html/rfc7517)

### Converting to Jwk Format

**Source**: `src/admin/jwks.rs:135-162`

```rust
impl RsaKeyPair {
    pub fn to_jwk(&self) -> Result<JsonWebKey> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use rsa::traits::PublicKeyParts;

        // Extract RSA components
        let n = self.public_key.n();  // Modulus (BigUint)
        let e = self.public_key.e();  // Exponent (BigUint)

        // Convert to big-endian bytes
        let n_bytes = n.to_bytes_be();
        let e_bytes = e.to_bytes_be();

        // Encode as base64url (no padding)
        let n_b64 = URL_SAFE_NO_PAD.encode(&n_bytes);
        let e_b64 = URL_SAFE_NO_PAD.encode(&e_bytes);

        Ok(JsonWebKey {
            kty: "RSA".to_owned(),
            key_use: "sig".to_owned(),
            kid: self.kid.clone(),
            alg: "RS256".to_owned(),
            n: n_b64,
            e: e_b64,
        })
    }
}
```

**Cryptography Explained**:

1. **BigUint to bytes**
   - RSA components are very large integers
   - `.to_bytes_be()` = big-endian byte representation
   - Standard format for JWK

2. **Base64url encoding**
   - URL-safe variant (replaces `+/` with `-_`)
   - No padding (`=`) for cleaner URLs
   - Standard for JWT/JWKS

---

## Ed25519 for A2A Authentication

A2A protocol uses **Ed25519** (elliptic curve) for faster, smaller signatures.

**Source**: `src/crypto/keys.rs:16-66`

### Ed25519 Key Generation

```rust
/// Ed25519 keypair for A2A client authentication
#[derive(Debug, Clone)]
pub struct A2AKeypair {
    pub public_key: String,   // Base64 encoded
    pub private_key: String,  // Base64 encoded
}

impl A2AKeyManager {
    pub fn generate_keypair() -> Result<A2AKeypair> {
        use rand::RngCore;

        let mut rng = OsRng;
        let mut secret_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);

        // Security: Zeroize secret bytes to prevent memory exposure
        secret_bytes.zeroize();

        let verifying_key = signing_key.verifying_key();

        let public_key = general_purpose::STANDARD.encode(verifying_key.as_bytes());
        let private_key = general_purpose::STANDARD.encode(signing_key.as_bytes());

        Ok(A2AKeypair { public_key, private_key })
    }
}
```

**Ed25519 vs RSA**:

| Feature | Ed25519 | RSA-4096 |
|---------|---------|----------|
| **Key size** | 32 bytes | 512 bytes |
| **Signature size** | 64 bytes | 512 bytes |
| **Generation speed** | Fast (~1ms) | Slow (~10s) |
| **Verification speed** | Fast | Slower |
| **Use case** | Modern systems | Legacy compatibility |

**Why Pierre uses both?**:
- **RS256 (RSA)**: JWT standard, widely supported
- **Ed25519**: A2A only, modern, efficient

**Reference**: [Ed25519 Paper](https://ed25519.cr.yp.to/)

---

## Zeroize: Secure Memory Cleanup

The `zeroize` crate prevents key material from lingering in memory.

**Source**: `src/crypto/keys.rs:54`

### The Memory Leak Problem

```rust
// WITHOUT zeroize - INSECURE
fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rng.fill_bytes(&mut key);
    key
    // key bytes still in memory!
    // Could be swapped to disk, dumped in crash, etc.
}

// WITH zeroize - SECURE
fn generate_key() -> [u8; 32] {
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);

    let key = secret_bytes;  // Copy to return value
    secret_bytes.zeroize();  // Overwrite with zeros

    key
}
```

### Zeroize Usage

**Source**: `src/crypto/keys.rs:45-55`

```rust
use zeroize::Zeroize;

let mut secret_bytes = [0u8; 32];
rng.fill_bytes(&mut secret_bytes);

let signing_key = SigningKey::from_bytes(&secret_bytes);

// Overwrite secret_bytes with zeros
secret_bytes.zeroize();  // ← Critical security step

// secret_bytes memory now contains all zeros
// Prevents recovery via memory dumps
```

**Rust Idioms Explained**:

1. **`.zeroize()` method**
   - Overwrites memory with zeros
   - Compiler can't optimize away (volatile write)
   - Safe even if code panics (Drop implementation)

2. **`Zeroize` trait**
   - Implemented for arrays, Vecs, Strings
   - Can derive: `#[derive(Zeroize)]`
   - Automatic on drop with `ZeroizeOnDrop`

**Example with automatic zeroize**:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
struct SecretKey {
    key: [u8; 32],
}

fn use_key() {
    let secret = SecretKey { key: [1; 32] };
    // Use secret...
}  // ← Automatically zeroized on drop!
```

**Reference**: [zeroize crate docs](https://docs.rs/zeroize/)

---

## Practical Exercises

### Exercise 1: Implement AES Encryption

Implement encryption with AES-256-GCM:

```rust
use aes_gcm::{Aes256Gcm, KeyInit};

fn encrypt_data(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    // TODO:
    // 1. Create Aes256Gcm cipher
    // 2. Generate random nonce
    // 3. Encrypt plaintext
    // 4. Prepend nonce to ciphertext
}
```

### Exercise 2: Use Zeroize

Add secure cleanup to key generation:

```rust
use zeroize::Zeroize;

fn generate_api_key() -> String {
    let mut random_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random_bytes);

    let api_key = base64::encode(&random_bytes);

    // TODO: Zeroize random_bytes before returning

    api_key
}
```

**Solution**: Add `random_bytes.zeroize();` before return

---

## Key Takeaways

1. **Two-tier keys**: MEK from environment, DEK from database
2. **AES-256-GCM**: Authenticated encryption with nonces
3. **RSA for JWT**: 4096-bit keys for production security
4. **Ed25519 for A2A**: Smaller, faster elliptic curve signatures
5. **OsRng for crypto**: Never use weak RNGs for keys
6. **zeroize for cleanup**: Prevent key leakage in memory
7. **Conditional compilation**: `#[cfg(debug_assertions)]` for safe logging

---

## Next Chapter

[Chapter 6: JWT Authentication with RS256](./chapter-06-jwt-authentication.md) - Learn JWT token generation, validation, claims-based authorization, and the `jsonwebtoken` crate.
