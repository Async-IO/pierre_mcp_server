// ABOUTME: OAuth 2.0 data models for client registration and token exchange
// ABOUTME: Implements RFC 7591 and OAuth 2.0 request/response structures
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// OAuth 2.0 Client Registration Request (RFC 7591)
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    /// Redirect URIs for authorization code flow
    pub redirect_uris: Vec<String>,
    /// Optional client name for display
    pub client_name: Option<String>,
    /// Optional client URI for information
    pub client_uri: Option<String>,
    /// Grant types the client can use
    pub grant_types: Option<Vec<String>>,
    /// Response types the client can use
    pub response_types: Option<Vec<String>>,
    /// Scopes the client can request
    pub scope: Option<String>,
}

/// OAuth 2.0 Client Registration Response (RFC 7591)
#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    /// Unique client identifier
    pub client_id: String,
    /// Client secret for authentication
    pub client_secret: String,
    /// When the client registration expires (optional)
    pub client_id_issued_at: Option<i64>,
    /// When the client secret expires (optional)
    pub client_secret_expires_at: Option<i64>,
    /// Redirect URIs registered for this client
    pub redirect_uris: Vec<String>,
    /// Grant types allowed for this client
    pub grant_types: Vec<String>,
    /// Response types allowed for this client
    pub response_types: Vec<String>,
    /// Client name
    pub client_name: Option<String>,
    /// Client URI
    pub client_uri: Option<String>,
    /// Scopes this client can request
    pub scope: Option<String>,
}

/// OAuth 2.0 Authorization Request
#[derive(Debug, Deserialize, Clone)]
pub struct AuthorizeRequest {
    /// Response type (code, token)
    pub response_type: String,
    /// Client identifier
    pub client_id: String,
    /// Redirect URI for response
    pub redirect_uri: String,
    /// Requested scopes
    pub scope: Option<String>,
    /// State parameter for CSRF protection
    pub state: Option<String>,
    /// PKCE code challenge (RFC 7636)
    pub code_challenge: Option<String>,
    /// PKCE code challenge method (plain or S256)
    pub code_challenge_method: Option<String>,
}

/// OAuth 2.0 Authorization Response
#[derive(Debug, Serialize)]
pub struct AuthorizeResponse {
    /// Authorization code
    pub code: String,
    /// State parameter (if provided in request)
    pub state: Option<String>,
}

/// OAuth 2.0 Token Request
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    /// Grant type (`authorization_code`, `client_credentials`, `refresh_token`)
    pub grant_type: String,
    /// Authorization code (for `authorization_code` grant)
    pub code: Option<String>,
    /// Redirect URI (must match registration)
    pub redirect_uri: Option<String>,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Requested scopes (for `client_credentials` grant)
    pub scope: Option<String>,
    /// Refresh token (for `refresh_token` grant)
    pub refresh_token: Option<String>,
    /// PKCE code verifier (RFC 7636, for `authorization_code` grant)
    pub code_verifier: Option<String>,
}

/// OAuth 2.0 Token Response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Access token (JWT)
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Expires in seconds
    pub expires_in: i64,
    /// Scopes granted
    pub scope: Option<String>,
    /// Refresh token (optional)
    pub refresh_token: Option<String>,
}

/// OAuth 2.0 Error Response
#[derive(Debug, Serialize)]
pub struct OAuth2Error {
    /// Error code
    pub error: String,
    /// Human-readable error description
    pub error_description: Option<String>,
    /// URI for error information
    pub error_uri: Option<String>,
}

impl OAuth2Error {
    /// Create an `invalid_request` error
    #[must_use]
    pub fn invalid_request(description: &str) -> Self {
        Self {
            error: "invalid_request".to_owned(),
            error_description: Some(description.to_owned()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.2.1".to_owned(),
            ),
        }
    }

    /// Create an `invalid_client` error
    #[must_use]
    pub fn invalid_client() -> Self {
        Self {
            error: "invalid_client".to_owned(),
            error_description: Some("Client authentication failed".to_owned()),
            error_uri: Some("https://datatracker.ietf.org/doc/html/rfc6749#section-5.2".to_owned()),
        }
    }

    /// Create an `invalid_grant` error
    #[must_use]
    pub fn invalid_grant(description: &str) -> Self {
        Self {
            error: "invalid_grant".to_owned(),
            error_description: Some(description.to_owned()),
            error_uri: Some("https://datatracker.ietf.org/doc/html/rfc6749#section-5.2".to_owned()),
        }
    }

    /// Create an `unsupported_grant_type` error
    #[must_use]
    pub fn unsupported_grant_type() -> Self {
        Self {
            error: "unsupported_grant_type".to_owned(),
            error_description: Some("Grant type not supported".to_owned()),
            error_uri: Some("https://datatracker.ietf.org/doc/html/rfc6749#section-5.2".to_owned()),
        }
    }

    /// Create an `unauthorized_client` error (RFC 6749 Section 4.1.2.1)
    /// Used when a client attempts to use a `grant_type` or `response_type` it was not registered for
    #[must_use]
    pub fn unauthorized_client(description: &str) -> Self {
        Self {
            error: "unauthorized_client".to_owned(),
            error_description: Some(description.to_owned()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.2.1".to_owned(),
            ),
        }
    }

    /// Create an `invalid_scope` error (RFC 6749 Section 4.1.2.1)
    /// Used when a client requests scopes beyond what it was registered for
    #[must_use]
    pub fn invalid_scope(description: &str) -> Self {
        Self {
            error: "invalid_scope".to_owned(),
            error_description: Some(description.to_owned()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.2.1".to_owned(),
            ),
        }
    }
}

// Database persistence types re-exported from pierre-core for unified type identity
pub use pierre_core::models::{OAuth2AuthCode, OAuth2Client, OAuth2RefreshToken, OAuth2State};

/// OAuth 2.0 Access Token
#[derive(Debug, Clone)]
pub struct OAuth2AccessToken {
    /// The access token value
    pub token: String,
    /// Client ID that owns this token
    pub client_id: String,
    /// User ID if user-authorized (None for `client_credentials` grant)
    pub user_id: Option<Uuid>,
    /// Space-separated list of granted scopes
    pub scope: Option<String>,
    /// When this token expires
    pub expires_at: DateTime<Utc>,
    /// When this token was created
    pub created_at: DateTime<Utc>,
}

/// Validate and Refresh Request
#[derive(Debug, Deserialize)]
pub struct ValidateRefreshRequest {
    /// Optional refresh token to use if access token is expired but refreshable
    pub refresh_token: Option<String>,
}

/// Validation status for token
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    /// Token is valid and can be used
    Valid,
    /// Token was refreshed, use new tokens
    Refreshed,
    /// Token is invalid, requires full re-authentication
    Invalid,
}

/// Validate and Refresh Response
#[derive(Debug, Serialize)]
pub struct ValidateRefreshResponse {
    /// Validation status
    pub status: ValidationStatus,

    /// Seconds until expiration (only for Valid status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,

    /// New access token (only for Refreshed status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,

    /// New refresh token (only for Refreshed status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Token type (only for Refreshed status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,

    /// Reason for invalidity (only for Invalid status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Whether full re-authentication is required (only for Invalid status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_full_reauth: Option<bool>,
}
