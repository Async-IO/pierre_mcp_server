// ABOUTME: Firebase Authentication token validation module
// ABOUTME: Validates Firebase ID tokens using Google's public keys with automatic key caching
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Firebase Authentication Token Validation
//!
//! This module provides:
//! - Firebase ID token validation using Google's public keys
//! - Automatic key fetching and caching from Google's JWKS endpoint
//! - Token claims extraction (email, provider, etc.)
//!
//! ## Security Model
//!
//! - Public keys fetched from Google's official endpoint
//! - Keys cached based on Cache-Control header (typically 1 hour)
//! - Tokens validated for issuer, audience, and expiry
//! - Provider ID extracted from `firebase.sign_in_provider` claim
//!
//! ## Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::admin::firebase::FirebaseAuth;
//! use pierre_mcp_server::config::environment::FirebaseConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = FirebaseConfig {
//!     project_id: Some("my-project".to_string()),
//!     api_key: None,
//!     enabled: true,
//!     key_cache_ttl_secs: 3600,
//! };
//! let firebase = FirebaseAuth::new(config);
//!
//! // Validate a Firebase ID token
//! let claims = firebase.validate_token("eyJ...").await?;
//! println!("User email: {}", claims.email.unwrap_or_default());
//! println!("Provider: {}", claims.provider);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use x509_parser::prelude::*;

use crate::config::environment::FirebaseConfig;
use crate::errors::{AppError, AppResult};

/// Google's Firebase public key endpoint
const FIREBASE_CERTS_URL: &str =
    "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";

/// Firebase issuer URL template (includes project ID)
const FIREBASE_ISSUER_TEMPLATE: &str = "https://securetoken.google.com/";

/// Minimum cache TTL in seconds (5 minutes)
const MIN_CACHE_TTL_SECS: i64 = 300;

/// Default cache TTL in seconds if Cache-Control header is missing (1 hour)
const DEFAULT_CACHE_TTL_SECS: i64 = 3600;

/// Cached Firebase public keys
struct CachedKeys {
    /// Key ID to PEM-encoded public key mapping
    keys: HashMap<String, String>,
    /// When the cache expires
    expires_at: DateTime<Utc>,
}

/// Firebase ID token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirebaseClaims {
    /// Issuer (should be `https://securetoken.google.com/<project-id>`)
    pub iss: String,
    /// Audience (should be the Firebase project ID)
    pub aud: String,
    /// Subject (Firebase user UID)
    pub sub: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// User email (if available)
    pub email: Option<String>,
    /// Whether email is verified
    pub email_verified: Option<bool>,
    /// User display name (if available)
    pub name: Option<String>,
    /// User profile picture URL (if available)
    pub picture: Option<String>,
    /// Firebase-specific claims
    #[serde(default)]
    pub firebase: FirebaseSpecificClaims,
    /// Authentication provider extracted from `firebase.sign_in_provider`
    #[serde(skip)]
    pub provider: String,
}

/// Firebase-specific claims within the token
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FirebaseSpecificClaims {
    /// Sign-in provider (e.g., "google.com", "apple.com", "password")
    pub sign_in_provider: Option<String>,
    /// Identity claims from the provider
    pub identities: Option<HashMap<String, Value>>,
}

/// Firebase Authentication handler
///
/// Provides token validation with automatic key caching.
/// Thread-safe via `Arc<RwLock<_>>` for concurrent access.
pub struct FirebaseAuth {
    /// Firebase configuration
    config: FirebaseConfig,
    /// HTTP client for fetching public keys
    http_client: Client,
    /// Cached public keys (Arc for sharing across threads)
    cached_keys: Arc<RwLock<Option<CachedKeys>>>,
}

impl FirebaseAuth {
    /// Create a new Firebase authentication handler
    #[must_use]
    pub fn new(config: FirebaseConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            cached_keys: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if Firebase authentication is enabled and configured
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.config.is_configured()
    }

    /// Get the Firebase project ID
    #[must_use]
    pub fn project_id(&self) -> Option<&str> {
        self.config.project_id.as_deref()
    }

    /// Validate a Firebase ID token
    ///
    /// # Arguments
    ///
    /// * `token` - The Firebase ID token to validate
    ///
    /// # Returns
    ///
    /// * `Ok(FirebaseClaims)` - The validated token claims
    /// * `Err(AppError)` - If validation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Firebase is not configured
    /// - Token header cannot be decoded
    /// - Public key cannot be found for the token's key ID
    /// - Token signature is invalid
    /// - Token is expired or not yet valid
    /// - Issuer or audience doesn't match
    pub async fn validate_token(&self, token: &str) -> AppResult<FirebaseClaims> {
        // Check if Firebase is configured
        let project_id =
            self.config.project_id.as_ref().ok_or_else(|| {
                AppError::invalid_input("Firebase authentication is not configured")
            })?;

        if !self.config.enabled {
            return Err(AppError::invalid_input(
                "Firebase authentication is disabled",
            ));
        }

        // Decode the token header to get the key ID
        let header = decode_header(token).map_err(|e| {
            debug!(error = %e, "Failed to decode Firebase token header");
            AppError::auth_invalid("Invalid token format")
        })?;

        let kid = header.kid.ok_or_else(|| {
            debug!("Firebase token missing key ID (kid) in header");
            AppError::auth_invalid("Token missing key ID")
        })?;

        // Get the public key for this key ID
        let pem_key = self.get_public_key(&kid).await?;

        // Create the decoding key from the PEM
        let decoding_key = DecodingKey::from_rsa_pem(pem_key.as_bytes()).map_err(|e| {
            warn!(error = %e, kid = %kid, "Failed to create decoding key from PEM");
            AppError::internal(format!("Invalid public key: {e}"))
        })?;

        // Set up validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[project_id]);
        validation.set_issuer(&[format!("{FIREBASE_ISSUER_TEMPLATE}{project_id}")]);

        // Decode and validate the token
        let token_data =
            decode::<FirebaseClaims>(token, &decoding_key, &validation).map_err(|e| {
                debug!(error = %e, "Firebase token validation failed");
                match e.kind() {
                    ErrorKind::ExpiredSignature => AppError::auth_expired(),
                    ErrorKind::InvalidAudience => AppError::auth_invalid("Invalid token audience"),
                    ErrorKind::InvalidIssuer => AppError::auth_invalid("Invalid token issuer"),
                    _ => AppError::auth_invalid("Invalid token"),
                }
            })?;

        // Extract the provider from firebase.sign_in_provider
        let mut claims = token_data.claims;
        claims.provider = claims
            .firebase
            .sign_in_provider
            .clone()
            .unwrap_or_else(|| "unknown".to_owned());

        info!(
            user_id = %claims.sub,
            provider = %claims.provider,
            "Firebase token validated successfully"
        );
        debug!(
            user_id = %claims.sub,
            email = claims.email.as_deref().unwrap_or("(none)"),
            "Firebase token claims detail"
        );

        Ok(claims)
    }

    /// Get the public key for a given key ID
    ///
    /// Fetches keys from cache or Google's endpoint if cache is expired.
    async fn get_public_key(&self, kid: &str) -> AppResult<String> {
        // Check if we have a valid cached key
        if let Some(key) = self.try_get_cached_key(kid).await {
            return Ok(key);
        }

        // Fetch fresh keys
        self.refresh_keys().await?;

        // Try to get the key from the refreshed cache
        self.get_cached_key_or_error(kid).await
    }

    /// Try to get a key from the cache if valid
    async fn try_get_cached_key(&self, kid: &str) -> Option<String> {
        let result = {
            let cache = self.cached_keys.read().await;
            cache.as_ref().and_then(|cached| {
                if cached.expires_at > Utc::now() {
                    cached.keys.get(kid).cloned()
                } else {
                    None
                }
            })
        };
        if result.is_some() {
            debug!(kid = %kid, "Using cached Firebase public key");
        }
        result
    }

    /// Get a key from cache or return error
    async fn get_cached_key_or_error(&self, kid: &str) -> AppResult<String> {
        let result = {
            let cache = self.cached_keys.read().await;
            cache
                .as_ref()
                .ok_or_else(|| AppError::internal("Failed to fetch Firebase public keys"))
                .and_then(|cached| {
                    cached.keys.get(kid).cloned().ok_or_else(|| {
                        debug!(kid = %kid, "Firebase public key not found for kid");
                        AppError::auth_invalid("Unknown token signing key")
                    })
                })
        };
        result
    }

    /// Refresh the public key cache from Google's endpoint
    async fn refresh_keys(&self) -> AppResult<()> {
        info!("Fetching Firebase public keys from Google");

        let (certs, cache_ttl) = self.fetch_google_certificates().await?;
        let keys = convert_certs_to_keys(certs)?;
        self.update_cache(keys, cache_ttl).await;

        Ok(())
    }

    /// Fetch X.509 certificates from Google's endpoint
    async fn fetch_google_certificates(&self) -> AppResult<(HashMap<String, String>, i64)> {
        let response = self
            .http_client
            .get(FIREBASE_CERTS_URL)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to fetch Firebase public keys");
                AppError::internal(format!("Failed to fetch Firebase public keys: {e}"))
            })?;

        // Parse cache TTL from Cache-Control header
        let cache_ttl = response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_max_age)
            .unwrap_or(DEFAULT_CACHE_TTL_SECS)
            .max(MIN_CACHE_TTL_SECS);

        // Parse the response body as a map of kid -> X.509 certificate
        let certs: HashMap<String, String> = response.json().await.map_err(|e| {
            warn!(error = %e, "Failed to parse Firebase public keys response");
            AppError::internal(format!("Failed to parse Firebase public keys: {e}"))
        })?;

        Ok((certs, cache_ttl))
    }

    /// Update the key cache with new keys
    async fn update_cache(&self, keys: HashMap<String, String>, cache_ttl: i64) {
        let expires_at = Utc::now() + Duration::seconds(cache_ttl);

        info!(
            num_keys = keys.len(),
            cache_ttl_secs = cache_ttl,
            expires_at = %expires_at,
            "Firebase public keys cached"
        );

        let mut cache = self.cached_keys.write().await;
        *cache = Some(CachedKeys { keys, expires_at });
    }
}

/// Convert X.509 certificates to PEM-encoded public keys
fn convert_certs_to_keys(certs: HashMap<String, String>) -> AppResult<HashMap<String, String>> {
    let mut keys = HashMap::with_capacity(certs.len());
    for (kid, cert_pem) in certs {
        match extract_public_key_from_cert(&cert_pem) {
            Ok(public_key_pem) => {
                keys.insert(kid, public_key_pem);
            }
            Err(e) => {
                warn!(kid = %kid, error = %e, "Failed to extract public key from certificate");
            }
        }
    }

    if keys.is_empty() {
        return Err(AppError::internal("No valid Firebase public keys found"));
    }

    Ok(keys)
}

/// Parse max-age value from Cache-Control header
///
/// Example: "public, max-age=3600, must-revalidate" -> 3600
fn parse_max_age(cache_control: &str) -> Option<i64> {
    cache_control
        .split(',')
        .map(str::trim)
        .find(|s| s.starts_with("max-age="))
        .and_then(|s| s.strip_prefix("max-age="))
        .and_then(|s| s.parse().ok())
}

/// Extract the public key from an X.509 certificate in PEM format
///
/// Firebase returns X.509 certificates, but we need the RSA public key
/// for JWT validation.
fn extract_public_key_from_cert(cert_pem: &str) -> AppResult<String> {
    // Parse the PEM-encoded certificate
    let (_, pem) = parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| AppError::internal(format!("Failed to parse X.509 PEM: {e}")))?;

    // Parse the X.509 certificate
    let (_, cert) = X509Certificate::from_der(&pem.contents)
        .map_err(|e| AppError::internal(format!("Failed to parse X.509 certificate: {e}")))?;

    // Get the subject public key info (SPKI)
    let spki = cert.public_key();

    // Convert SPKI to PEM format
    // The SPKI is already in DER format, we just need to PEM-encode it
    let spki_der = spki.raw;
    let pem_encoded = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        STANDARD
            .encode(spki_der)
            .chars()
            .collect::<Vec<_>>()
            .chunks(64)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(pem_encoded)
}
