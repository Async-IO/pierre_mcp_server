// ABOUTME: OAuth response types for use across modules
// ABOUTME: Shared types to avoid layering inversions (core modules should not depend on HTTP routes)
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use serde::Serialize;

/// OAuth callback response returned after successful OAuth authorization
///
/// This type is used by both HTTP route handlers and core OAuth flow managers.
/// Placed in types module to avoid layering inversion where core modules
/// would otherwise import from HTTP route definitions.
#[derive(Debug, Serialize)]
pub struct OAuthCallbackResponse {
    /// User ID for the connected account
    pub user_id: String,
    /// Name of the OAuth provider
    pub provider: String,
    /// When the OAuth token expires (ISO 8601 format)
    pub expires_at: String,
    /// Space-separated list of granted OAuth scopes
    pub scopes: String,
    /// Optional mobile redirect URL from OAuth state (for mobile app flows)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile_redirect_url: Option<String>,
}
