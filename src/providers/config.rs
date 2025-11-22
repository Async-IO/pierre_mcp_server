// ABOUTME: Generic provider configuration structures
// ABOUTME: OAuth credentials and endpoint configuration for any provider type
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OAuth 2.0 credentials and token management
///
/// Stores OAuth tokens, refresh tokens, and expiration information.
/// Generic enough to work with any OAuth provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Credentials {
    /// OAuth client ID from provider
    pub client_id: String,
    /// OAuth client secret from provider
    pub client_secret: String,
    /// Current access token
    pub access_token: Option<String>,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
    /// When the access token expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Granted OAuth scopes
    pub scopes: Vec<String>,
}

/// Provider configuration containing all necessary endpoints and settings
///
/// Generic configuration structure that works for any OAuth-enabled provider.
/// Contains endpoint URLs, API base URL, and default scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "strava", "fitbit", "garmin", "synthetic")
    pub name: String,
    /// OAuth authorization endpoint URL
    pub auth_url: String,
    /// OAuth token endpoint URL
    pub token_url: String,
    /// Base URL for provider API calls
    pub api_base_url: String,
    /// Optional token revocation endpoint URL
    pub revoke_url: Option<String>,
    /// Default OAuth scopes to request
    pub default_scopes: Vec<String>,
}
