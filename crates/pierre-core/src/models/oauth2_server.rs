// ABOUTME: OAuth 2.0 server persistence models for clients, auth codes, tokens, and state
// ABOUTME: Used by DatabaseProvider trait for OAuth 2.0 server storage operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Stored OAuth 2.0 Client
#[derive(Debug, Clone)]
pub struct OAuth2Client {
    /// Internal database ID
    pub id: String,
    /// OAuth 2.0 client identifier
    pub client_id: String,
    /// Hashed client secret for authentication
    pub client_secret_hash: String,
    /// Registered redirect URIs for authorization code flow
    pub redirect_uris: Vec<String>,
    /// Allowed OAuth 2.0 grant types (`authorization_code`, `client_credentials`, etc.)
    pub grant_types: Vec<String>,
    /// Allowed OAuth 2.0 response types (code, token, etc.)
    pub response_types: Vec<String>,
    /// Human-readable client name
    pub client_name: Option<String>,
    /// Client's home page URL
    pub client_uri: Option<String>,
    /// Space-separated list of allowed scopes
    pub scope: Option<String>,
    /// When this client was created
    pub created_at: DateTime<Utc>,
    /// Optional expiration time for the client registration
    pub expires_at: Option<DateTime<Utc>>,
}

/// OAuth 2.0 Authorization Code
#[derive(Debug, Clone)]
pub struct OAuth2AuthCode {
    /// The authorization code value
    pub code: String,
    /// Client ID that requested this code
    pub client_id: String,
    /// User who authorized the code
    pub user_id: Uuid,
    /// Tenant ID for multi-tenancy
    pub tenant_id: String,
    /// Redirect URI that must match during token exchange
    pub redirect_uri: String,
    /// Space-separated list of granted scopes
    pub scope: Option<String>,
    /// When this authorization code expires
    pub expires_at: DateTime<Utc>,
    /// Whether this code has been exchanged for a token
    pub used: bool,
    /// Client-generated state for CSRF protection (RFC 6749 Section 10.12)
    pub state: Option<String>,
    /// PKCE code challenge (RFC 7636)
    pub code_challenge: Option<String>,
    /// PKCE code challenge method (plain or S256)
    pub code_challenge_method: Option<String>,
}

/// OAuth 2.0 Refresh Token
#[derive(Debug, Clone)]
pub struct OAuth2RefreshToken {
    /// The refresh token value
    pub token: String,
    /// Client application identifier
    pub client_id: String,
    /// User identifier who owns this token
    pub user_id: Uuid,
    /// Tenant identifier for multi-tenant support
    pub tenant_id: String,
    /// Optional space-separated list of granted scopes
    pub scope: Option<String>,
    /// Timestamp when this refresh token expires
    pub expires_at: DateTime<Utc>,
    /// Timestamp when this refresh token was created
    pub created_at: DateTime<Utc>,
    /// Whether this refresh token has been revoked
    pub revoked: bool,
}

/// OAuth 2.0 State for CSRF Protection
#[derive(Debug, Clone)]
pub struct OAuth2State {
    /// Unique state value for CSRF protection
    pub state: String,
    /// Client application identifier
    pub client_id: String,
    /// Optional user identifier if authenticated
    pub user_id: Option<Uuid>,
    /// Optional tenant identifier for multi-tenant support
    pub tenant_id: Option<String>,
    /// URI to redirect to after authorization
    pub redirect_uri: String,
    /// Optional space-separated list of requested scopes
    pub scope: Option<String>,
    /// Optional PKCE code challenge for enhanced security
    pub code_challenge: Option<String>,
    /// Method used for PKCE code challenge (S256 or plain)
    pub code_challenge_method: Option<String>,
    /// Timestamp when this state was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when this state expires
    pub expires_at: DateTime<Utc>,
    /// Whether this state has been consumed
    pub used: bool,
}
