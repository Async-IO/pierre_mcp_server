// ABOUTME: Request and response types for authentication routes
// ABOUTME: Defines DTOs for registration, login, OAuth, and user management endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Authentication request and response types
//!
//! This module contains all DTOs (Data Transfer Objects) used by the authentication
//! routes for serialization and deserialization of API requests and responses.

use serde::{Deserialize, Serialize};

/// User registration request
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    /// User's email address
    pub email: String,
    /// User's password (will be hashed)
    pub password: String,
    /// Optional display name for the user
    pub display_name: Option<String>,
}

/// User registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    /// Unique identifier for the newly created user
    pub user_id: String,
    /// Success message for the registration
    pub message: String,
}

/// User login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// User's email address
    pub email: String,
    /// User's password
    pub password: String,
}

/// Firebase login request - authenticate with Firebase ID token
#[derive(Debug, Deserialize)]
pub struct FirebaseLoginRequest {
    /// Firebase ID token from client-side Firebase SDK
    pub id_token: String,
}

/// User info for login response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    /// Unique identifier for the user
    pub user_id: String,
    /// User's email address
    pub email: String,
    /// User's display name if set
    pub display_name: Option<String>,
    /// Whether the user has admin privileges
    pub is_admin: bool,
    /// User role for permission system (`super_admin`, `admin`, `user`)
    pub role: String,
    /// User account status (`pending`, `active`, `suspended`)
    pub user_status: String,
}

/// User login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// JWT authentication token (optional, set in httpOnly cookie)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_token: Option<String>,
    /// CSRF token for request validation (client must include in X-CSRF-Token header)
    pub csrf_token: String,
    /// When the token expires (ISO 8601 format)
    pub expires_at: String,
    /// User information
    pub user: UserInfo,
}

/// User profile update request
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    /// New display name for the user
    pub display_name: String,
}

/// User profile update response
#[derive(Debug, Serialize)]
pub struct UpdateProfileResponse {
    /// Success message
    pub message: String,
    /// Updated user information
    pub user: UserInfo,
}

/// Change password request for authenticated users
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    /// Current password for verification
    pub current_password: String,
    /// New password to set
    pub new_password: String,
}

/// Complete a password reset using a one-time reset token
///
/// The token is issued by an admin via the password reset endpoint.
/// Users present the token along with their chosen new password.
#[derive(Debug, Deserialize)]
pub struct CompleteResetRequest {
    /// One-time reset token provided by the admin
    pub reset_token: String,
    /// New password chosen by the user
    pub new_password: String,
}

/// Session restore response for authenticated users
///
/// Returned by `GET /api/auth/session` to restore sessions using httpOnly cookies.
/// Provides user info and a fresh JWT for WebSocket authentication.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    /// Authenticated user information
    pub user: UserInfo,
    /// Fresh JWT token for WebSocket authentication (not stored in cookies)
    pub access_token: String,
    /// Fresh CSRF token for request validation
    pub csrf_token: String,
}

/// User stats response for dashboard
#[derive(Debug, Serialize)]
pub struct UserStatsResponse {
    /// Number of connected fitness providers
    pub connected_providers: i64,
    /// Number of days the user has been active
    pub days_active: i64,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// Current JWT token to refresh
    pub token: String,
    /// User ID for validation
    pub user_id: String,
}

/// `OAuth2` ROPC (Resource Owner Password Credentials) token request
/// Per RFC 6749 Section 4.3 - uses form-encoded body
#[derive(Debug, Deserialize)]
pub struct OAuth2TokenRequest {
    /// Grant type - must be "password" for ROPC
    pub grant_type: String,
    /// User's email address (RFC calls this "username")
    pub username: String,
    /// User's password
    pub password: String,
    /// `OAuth2` client identifier (optional for first-party clients)
    pub client_id: Option<String>,
    /// `OAuth2` client secret (optional for public clients)
    pub client_secret: Option<String>,
    /// Requested `OAuth2` scopes (optional, space-separated)
    pub scope: Option<String>,
}

/// `OAuth2` token response per RFC 6749 Section 5.1
/// Extended with optional user info for frontend compatibility
#[derive(Debug, Serialize)]
pub struct OAuth2TokenResponse {
    /// The access token issued by the authorization server
    pub access_token: String,
    /// The type of the token issued (always "Bearer")
    pub token_type: String,
    /// The lifetime in seconds of the access token
    pub expires_in: i64,
    /// Optional refresh token for obtaining new access tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// The scope of the access token (space-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    // --- Pierre extensions (allowed per RFC 6749 Section 5.1) ---
    /// User information (Pierre extension for frontend compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
    /// CSRF token for web clients (Pierre extension)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csrf_token: Option<String>,
}

/// `OAuth2` error response per RFC 6749 Section 5.2
#[derive(Debug, Serialize)]
pub struct OAuth2ErrorResponse {
    /// Error code per RFC 6749 (e.g., `invalid_grant`, `invalid_client`)
    pub error: String,
    /// Human-readable description of the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

/// OAuth provider connection status
#[derive(Debug, Serialize)]
pub struct OAuthStatus {
    /// Name of the OAuth provider (e.g., "strava", "google")
    pub provider: String,
    /// Whether the user is currently connected to this provider
    pub connected: bool,
    /// When the last sync occurred (ISO 8601 format)
    pub last_sync: Option<String>,
}

/// OAuth authorization response for provider auth URLs
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationResponse {
    /// URL to redirect user to for OAuth authorization
    pub authorization_url: String,
    /// CSRF state token for validating callback
    pub state: String,
    /// Human-readable instructions for the user
    pub instructions: String,
    /// How long the authorization URL is valid (minutes)
    pub expires_in_minutes: i64,
}

/// Connection status for fitness providers
#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    /// Name of the fitness provider (e.g., "strava", "garmin")
    pub provider: String,
    /// Whether the user is connected to this provider
    pub connected: bool,
    /// How this provider was connected (e.g., "oauth", "synthetic", "manual")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<String>,
    /// When the connection expires (ISO 8601 format)
    pub expires_at: Option<String>,
    /// Space-separated list of granted OAuth scopes
    pub scopes: Option<String>,
}

/// Provider status for the unified providers endpoint
///
/// Returns all available providers (OAuth and non-OAuth) with their connection status.
/// This is the source of truth for frontend provider displays.
#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    /// Provider identifier (e.g., "strava", "synthetic")
    pub provider: String,
    /// Human-readable display name (e.g., "Strava", "Synthetic")
    pub display_name: String,
    /// Whether this provider requires OAuth authentication
    pub requires_oauth: bool,
    /// Whether the user is connected to this provider
    pub connected: bool,
    /// Provider capabilities (e.g., `["activities", "sleep"]`)
    pub capabilities: Vec<String>,
}

/// Response for the /api/providers endpoint
#[derive(Debug, Serialize)]
pub struct ProvidersStatusResponse {
    /// List of all available providers with their status
    pub providers: Vec<ProviderStatus>,
}
