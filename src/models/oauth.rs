// ABOUTME: OAuth token models for secure credential storage and notifications
// ABOUTME: EncryptedToken, DecryptedToken, UserOAuthToken, UserOAuthApp, OAuthNotification, and session types
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};

/// Encrypted `OAuth` token storage
///
/// Tokens are encrypted at rest using AES-256-GCM encryption.
/// Only decrypted when needed for `API` calls.
/// Each encrypted token has its nonce prepended to the ciphertext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedToken {
    /// Encrypted access token with prepended nonce (base64 encoded: \[12-byte nonce\]\[ciphertext\])
    pub access_token: String,
    /// Encrypted refresh token with prepended nonce (base64 encoded: \[12-byte nonce\]\[ciphertext\])
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Token scope permissions
    pub scope: String,
}

impl EncryptedToken {
    /// Create a new encrypted token
    ///
    /// Encrypts both access and refresh tokens with independent nonces.
    /// Each nonce is prepended to its corresponding ciphertext for cryptographic independence.
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails or if the encryption key is invalid
    pub fn new(
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        scope: String,
        encryption_key: &[u8],
    ) -> AppResult<Self> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // Encrypt access token with its own nonce
        let mut access_nonce_bytes = [0u8; 12];
        rng.fill(&mut access_nonce_bytes)?;
        let access_nonce = Nonce::assume_unique_for_key(access_nonce_bytes);

        let unbound_key = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        let mut access_token_data = access_token.as_bytes().to_vec();
        key.seal_in_place_append_tag(access_nonce, Aad::empty(), &mut access_token_data)?;

        // Prepend nonce to ciphertext (modern pattern)
        let mut access_combined = access_nonce_bytes.to_vec();
        access_combined.extend(access_token_data);
        let encrypted_access = general_purpose::STANDARD.encode(access_combined);

        // Encrypt refresh token with its own independent nonce
        let mut refresh_nonce_bytes = [0u8; 12];
        rng.fill(&mut refresh_nonce_bytes)?;
        let refresh_nonce = Nonce::assume_unique_for_key(refresh_nonce_bytes);

        let unbound_key2 = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key2 = LessSafeKey::new(unbound_key2);

        let mut refresh_token_data = refresh_token.as_bytes().to_vec();
        key2.seal_in_place_append_tag(refresh_nonce, Aad::empty(), &mut refresh_token_data)?;

        // Prepend nonce to ciphertext (modern pattern)
        let mut refresh_combined = refresh_nonce_bytes.to_vec();
        refresh_combined.extend(refresh_token_data);
        let encrypted_refresh = general_purpose::STANDARD.encode(refresh_combined);

        Ok(Self {
            access_token: encrypted_access,
            refresh_token: encrypted_refresh,
            expires_at,
            scope,
        })
    }

    /// Decrypt the token for use
    ///
    /// Extracts nonces from the prepended ciphertext and decrypts each token independently.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails, nonce is invalid, or the encryption key is incorrect
    pub fn decrypt(&self, encryption_key: &[u8]) -> AppResult<DecryptedToken> {
        use base64::{engine::general_purpose, Engine as _};
        use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

        // Decrypt access token: extract nonce from prepended data
        let access_combined = general_purpose::STANDARD.decode(&self.access_token)?;
        if access_combined.len() < 12 {
            return Err(AppError::invalid_input("Invalid access token: too short"));
        }

        let (access_nonce_bytes, access_ciphertext) = access_combined.split_at(12);
        let access_nonce = Nonce::assume_unique_for_key(access_nonce_bytes.try_into()?);

        let unbound_key = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key = LessSafeKey::new(unbound_key);

        let mut access_data = access_ciphertext.to_vec();
        let access_plaintext = key.open_in_place(access_nonce, Aad::empty(), &mut access_data)?;
        let access_token = String::from_utf8(access_plaintext.to_vec())
            .map_err(|e| AppError::invalid_input(format!("Invalid UTF-8 in access token: {e}")))?;

        // Decrypt refresh token: extract nonce from prepended data
        let refresh_combined = general_purpose::STANDARD.decode(&self.refresh_token)?;
        if refresh_combined.len() < 12 {
            return Err(AppError::invalid_input("Invalid refresh token: too short"));
        }

        let (refresh_nonce_bytes, refresh_ciphertext) = refresh_combined.split_at(12);
        let refresh_nonce = Nonce::assume_unique_for_key(refresh_nonce_bytes.try_into()?);

        let unbound_key2 = UnboundKey::new(&AES_256_GCM, encryption_key)?;
        let key2 = LessSafeKey::new(unbound_key2);

        let mut refresh_data = refresh_ciphertext.to_vec();
        let refresh_plaintext =
            key2.open_in_place(refresh_nonce, Aad::empty(), &mut refresh_data)?;
        let refresh_token = String::from_utf8(refresh_plaintext.to_vec())
            .map_err(|e| AppError::invalid_input(format!("Invalid UTF-8 in refresh token: {e}")))?;

        Ok(DecryptedToken {
            access_token,
            refresh_token,
            expires_at: self.expires_at,
            scope: self.scope.clone(),
        })
    }
}

/// Decrypted `OAuth` token for `API` calls
///
/// This is never stored - only exists in memory during `API` requests.
#[derive(Debug, Clone)]
pub struct DecryptedToken {
    /// Plain text access token
    pub access_token: String,
    /// Plain text refresh token
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Token scope permissions
    pub scope: String,
}

/// User OAuth token for tenant-provider combination
///
/// Stores user's personal OAuth tokens for accessing fitness providers
/// within their tenant's application context. Each user can have one token
/// per tenant-provider combination (e.g., user's Strava token in tenant A).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthToken {
    /// Unique identifier for this token record
    pub id: String,
    /// User who owns this token
    pub user_id: Uuid,
    /// Tenant context for this token
    pub tenant_id: String,
    /// Provider name (strava, fitbit, etc.)
    pub provider: String,
    /// Encrypted OAuth access token
    pub access_token: String,
    /// Encrypted OAuth refresh token (optional for some providers)
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// When the access token expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Granted OAuth scopes
    pub scope: Option<String>,
    /// When this token was first stored
    pub created_at: DateTime<Utc>,
    /// When this token was last updated
    pub updated_at: DateTime<Utc>,
}

impl UserOAuthToken {
    /// Create a new user OAuth token
    #[must_use]
    pub fn new(
        user_id: Uuid,
        tenant_id: String,
        provider: String,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            tenant_id,
            provider,
            access_token,
            refresh_token,
            token_type: "Bearer".to_owned(),
            expires_at,
            scope,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the access token is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| Utc::now() > expires_at)
    }

    /// Check if token needs refresh (expires within 5 minutes)
    #[must_use]
    pub fn needs_refresh(&self) -> bool {
        self.expires_at.is_some_and(|expires_at| {
            let refresh_threshold = Utc::now() + chrono::Duration::minutes(5);
            refresh_threshold >= expires_at
        })
    }

    /// Update token with new values
    pub fn update_token(
        &mut self,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) {
        self.access_token = access_token;
        self.refresh_token = refresh_token;
        self.expires_at = expires_at;
        self.scope = scope;
        self.updated_at = Utc::now();
    }
}

/// User OAuth app credentials for cloud deployment
///
/// Each user can configure their own OAuth application credentials
/// for each provider (Strava, Fitbit, etc.) to work in cloud deployments
/// where server-wide environment variables won't work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthApp {
    /// Unique identifier for this OAuth app configuration
    pub id: String,
    /// User who owns this OAuth app configuration
    pub user_id: Uuid,
    /// OAuth provider name (strava, fitbit, etc.)
    pub provider: String,
    /// OAuth client ID from the provider
    pub client_id: String,
    /// OAuth client secret from the provider (encrypted)
    pub client_secret: String,
    /// OAuth redirect URI configured with the provider
    pub redirect_uri: String,
    /// When this configuration was created
    pub created_at: DateTime<Utc>,
    /// When this configuration was last updated
    pub updated_at: DateTime<Utc>,
}

impl UserOAuthApp {
    /// Create a new user OAuth app configuration
    #[must_use]
    pub fn new(
        user_id: Uuid,
        provider: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            provider,
            client_id,
            client_secret,
            redirect_uri,
            created_at: now,
            updated_at: now,
        }
    }
}

/// User session for `MCP` protocol authentication
///
/// Contains `JWT` token and user context for secure `MCP` communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// User `ID` this session belongs to
    pub user_id: Uuid,
    /// `JWT` token for authentication
    pub jwt_token: String,
    /// When the session expires
    pub expires_at: DateTime<Utc>,
    /// User's email for display
    pub email: String,
    /// Available fitness providers for this user
    pub available_providers: Vec<String>,
}

/// Authentication request for `MCP` protocol
///
/// Clients send this to authenticate with the `MCP` server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// `JWT` token for authentication
    pub token: String,
}

/// Authentication response for `MCP` protocol
///
/// Server responds with user context and available capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Whether authentication was successful
    pub authenticated: bool,
    /// User `ID` if authenticated
    pub user_id: Option<Uuid>,
    /// Error message if authentication failed
    pub error: Option<String>,
    /// Available fitness providers for this user
    pub available_providers: Vec<String>,
}

/// Type of provider connection
///
/// Distinguishes how a provider was connected to enable type-specific behavior
/// (e.g., OAuth connections have tokens that expire, synthetic connections do not).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    /// Connected via OAuth 2.0 token exchange
    OAuth,
    /// Connected via synthetic/test data seeding
    Synthetic,
    /// Connected via manual configuration
    Manual,
}

impl ConnectionType {
    /// Convert from database string representation
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the string is not a valid connection type.
    pub fn from_str_value(s: &str) -> AppResult<Self> {
        match s {
            "oauth" => Ok(Self::OAuth),
            "synthetic" => Ok(Self::Synthetic),
            "manual" => Ok(Self::Manual),
            other => Err(AppError::invalid_input(format!(
                "Unknown connection type: {other}"
            ))),
        }
    }

    /// Convert to database string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OAuth => "oauth",
            Self::Synthetic => "synthetic",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Provider connection record: single source of truth for provider connectivity
///
/// Tracks whether a provider (OAuth, synthetic, or manual) is connected for a user.
/// All provider types register in this table, eliminating the need to query
/// separate tables (`user_oauth_tokens`, `synthetic_activities`) for connection status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConnection {
    /// Unique identifier for this connection record
    pub id: String,
    /// User who owns this connection
    pub user_id: Uuid,
    /// Tenant context for multi-tenant isolation
    pub tenant_id: String,
    /// Provider name (e.g., "strava", "garmin", "synthetic")
    pub provider: String,
    /// How this provider was connected
    pub connection_type: ConnectionType,
    /// When the connection was established
    pub connected_at: DateTime<Utc>,
    /// Optional JSON metadata (e.g., {"source": "seed-synthetic-activities"})
    pub metadata: Option<String>,
}

impl ProviderConnection {
    /// Create a new provider connection record
    #[must_use]
    pub fn new(
        user_id: Uuid,
        tenant_id: String,
        provider: String,
        connection_type: ConnectionType,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            tenant_id,
            provider,
            connection_type,
            connected_at: Utc::now(),
            metadata: None,
        }
    }

    /// Create a new provider connection with metadata
    #[must_use]
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// OAuth notification data structure for tracking OAuth flow completion events
///
/// Used to deliver asynchronous notifications to users about OAuth connection
/// status changes (success/failure of provider connections).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthNotification {
    /// Unique notification ID
    pub id: String,
    /// User ID this notification belongs to
    pub user_id: String,
    /// Provider name (e.g., "strava", "fitbit")
    pub provider: String,
    /// Whether OAuth flow succeeded
    pub success: bool,
    /// Notification message text
    pub message: String,
    /// Optional expiration timestamp as ISO 8601 string
    pub expires_at: Option<String>,
    /// When the notification was created
    pub created_at: DateTime<Utc>,
    /// When the notification was read (if read)
    pub read_at: Option<DateTime<Utc>>,
}
