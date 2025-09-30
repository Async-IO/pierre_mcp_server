// ABOUTME: OAuth module organizing authentication and provider management
// ABOUTME: Centralizes OAuth2 flows, token management, and provider integrations
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth Management Module
//!
//! Unified OAuth handling for all fitness providers across both MCP servers.
//! Provides a consistent interface for OAuth flows regardless of provider.

pub mod providers;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// OAuth token data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub scopes: String,
    pub provider: String,
}

/// OAuth authorization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResponse {
    pub authorization_url: String,
    pub state: String,
    pub provider: String,
    pub instructions: String,
    pub expires_in_minutes: u32,
}

/// OAuth callback response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackResponse {
    pub user_id: String,
    pub provider: String,
    pub expires_at: String,
    pub scopes: String,
    pub success: bool,
    pub message: String,
}

/// OAuth error types
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("Provider not supported: {0}")]
    UnsupportedProvider(String),

    #[error("Invalid authorization code")]
    InvalidCode,

    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("Invalid state parameter")]
    InvalidState,

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Trait for OAuth provider implementations
#[async_trait::async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Generate authorization URL
    async fn generate_auth_url(
        &self,
        user_id: Uuid,
        state: String,
    ) -> Result<AuthorizationResponse, OAuthError>;

    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str, state: &str) -> Result<TokenData, OAuthError>;

    /// Refresh access token
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenData, OAuthError>;

    /// Revoke access token
    async fn revoke_token(&self, access_token: &str) -> Result<(), OAuthError>;

    /// Validate token and check if refresh needed
    async fn validate_token(&self, token: &TokenData) -> Result<bool, OAuthError>;
}

/// OAuth provider registry
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn OAuthProvider>>,
}

impl ProviderRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a new OAuth provider
    pub fn register_provider(&mut self, provider: Box<dyn OAuthProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// Get provider by name
    #[must_use]
    pub fn get_provider(&self, name: &str) -> Option<&dyn OAuthProvider> {
        self.providers.get(name).map(std::convert::AsRef::as_ref)
    }

    /// List all registered providers
    #[must_use]
    pub fn list_providers(&self) -> Vec<&str> {
        self.providers
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
