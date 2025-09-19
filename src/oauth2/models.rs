// ABOUTME: OAuth 2.0 data models for client registration and token exchange
// ABOUTME: Implements RFC 7591 and OAuth 2.0 request/response structures

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
    /// Grant type (`authorization_code`, `client_credentials`)
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
    #[must_use]
    pub fn invalid_request(description: &str) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.to_string()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.2.1".to_string(),
            ),
        }
    }

    #[must_use]
    pub fn invalid_client() -> Self {
        Self {
            error: "invalid_client".to_string(),
            error_description: Some("Client authentication failed".to_string()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-5.2".to_string(),
            ),
        }
    }

    #[must_use]
    pub fn invalid_grant(description: &str) -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some(description.to_string()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-5.2".to_string(),
            ),
        }
    }

    #[must_use]
    pub fn unsupported_grant_type() -> Self {
        Self {
            error: "unsupported_grant_type".to_string(),
            error_description: Some("Grant type not supported".to_string()),
            error_uri: Some(
                "https://datatracker.ietf.org/doc/html/rfc6749#section-5.2".to_string(),
            ),
        }
    }
}

/// Stored OAuth 2.0 Client
#[derive(Debug, Clone)]
pub struct OAuth2Client {
    pub id: String,
    pub client_id: String,
    pub client_secret_hash: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    pub client_name: Option<String>,
    pub client_uri: Option<String>,
    pub scope: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// OAuth 2.0 Authorization Code
#[derive(Debug, Clone)]
pub struct OAuth2AuthCode {
    pub code: String,
    pub client_id: String,
    pub user_id: Uuid,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

/// OAuth 2.0 Access Token
#[derive(Debug, Clone)]
pub struct OAuth2AccessToken {
    pub token: String,
    pub client_id: String,
    pub user_id: Option<Uuid>, // None for client_credentials
    pub scope: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
