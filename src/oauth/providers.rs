// ABOUTME: OAuth provider configurations and endpoint management
// ABOUTME: Defines OAuth2 provider settings for Strava, Fitbit, and other platforms
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth Provider Implementations
//!
//! Concrete implementations of OAuth providers for different fitness platforms.

use super::{AuthorizationResponse, OAuthError, OAuthProvider, TokenData};
use crate::config::environment::OAuthProviderConfig;
use crate::utils::oauth::{
    exchange_authorization_code, refresh_access_token, revoke_access_token, validate_token_expiry,
    AuthMethod, OAuthConfig,
};
use anyhow::Result;
use serde::Deserialize;
use uuid::Uuid;

/// Strava OAuth provider
pub struct StravaOAuthProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

/// Strava token response format
#[derive(Debug, Deserialize)]
struct StravaTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    scope: Option<String>,
}

impl StravaOAuthProvider {
    /// Create a new Strava OAuth provider from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client ID is not configured in the provided config
    /// - Client secret is not configured in the provided config
    /// - Configuration parameters are invalid
    pub fn from_config(config: &OAuthProviderConfig) -> Result<Self, OAuthError> {
        let client_id = config
            .client_id
            .as_ref()
            .ok_or_else(|| {
                OAuthError::ConfigurationError("Strava client_id not configured".into())
            })?
            .clone();

        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| {
                OAuthError::ConfigurationError("Strava client_secret not configured".into())
            })?
            .clone();

        let redirect_uri = config
            .redirect_uri
            .clone()
            .unwrap_or_else(crate::constants::env_config::strava_redirect_uri);

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

#[async_trait::async_trait]
impl OAuthProvider for StravaOAuthProvider {
    fn name(&self) -> &'static str {
        "strava"
    }

    /// Generate authorization URL for Strava OAuth flow
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL encoding fails for any parameter
    /// - Base authorization URL is malformed
    /// - State parameter is invalid
    async fn generate_auth_url(
        &self,
        _user_id: Uuid,
        state: String,
    ) -> Result<AuthorizationResponse, OAuthError> {
        let scope = "read,activity:read_all";

        let auth_base_url = crate::constants::env_config::strava_auth_url();
        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            auth_base_url,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(&state)
        );

        Ok(AuthorizationResponse {
            authorization_url: auth_url,
            state,
            provider: "strava".into(),
            instructions: "Visit the authorization URL to connect your Strava account. Complete the OAuth flow through your web browser.".into(),
            expires_in_minutes: 10,
        })
    }

    async fn exchange_code(&self, code: &str, _state: &str) -> Result<TokenData, OAuthError> {
        let config = OAuthConfig {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            redirect_uri: self.redirect_uri.clone(),
            token_url: crate::constants::env_config::strava_token_url(),
            provider_name: "strava".into(),
            auth_method: AuthMethod::FormParams,
        };

        exchange_authorization_code(
            &config,
            code,
            |token_response: StravaTokenResponse, provider| {
                let expires_at =
                    chrono::DateTime::<chrono::Utc>::from_timestamp(token_response.expires_at, 0)
                        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(6));

                TokenData {
                    access_token: token_response.access_token,
                    refresh_token: token_response.refresh_token,
                    expires_at,
                    scopes: token_response
                        .scope
                        .unwrap_or_else(|| "read,activity:read_all".into()),
                    provider: provider.into(),
                }
            },
        )
        .await
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenData, OAuthError> {
        let config = OAuthConfig {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            redirect_uri: self.redirect_uri.clone(),
            token_url: crate::constants::env_config::strava_token_url(),
            provider_name: "strava".into(),
            auth_method: AuthMethod::FormParams,
        };

        refresh_access_token(
            &config,
            refresh_token,
            |token_response: StravaTokenResponse, provider| {
                let expires_at =
                    chrono::DateTime::<chrono::Utc>::from_timestamp(token_response.expires_at, 0)
                        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(6));

                TokenData {
                    access_token: token_response.access_token,
                    refresh_token: token_response.refresh_token,
                    expires_at,
                    scopes: token_response
                        .scope
                        .unwrap_or_else(|| "read,activity:read_all".into()),
                    provider: provider.into(),
                }
            },
        )
        .await
    }

    async fn revoke_token(&self, access_token: &str) -> Result<(), OAuthError> {
        revoke_access_token(
            &crate::constants::env_config::strava_deauthorize_url(),
            access_token,
            &AuthMethod::FormParams,
            &self.client_id,
            &self.client_secret,
        )
        .await
    }

    async fn validate_token(&self, token: &TokenData) -> Result<bool, OAuthError> {
        validate_token_expiry(token, 5)
    }
}

/// Fitbit OAuth provider
pub struct FitbitOAuthProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

/// Fitbit token response format
#[derive(Debug, Deserialize)]
struct FitbitTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    scope: String,
}

impl FitbitOAuthProvider {
    /// Create a new Fitbit OAuth provider from configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client ID is not configured in the provided config
    /// - Client secret is not configured in the provided config
    /// - Configuration parameters are invalid
    pub fn from_config(config: &OAuthProviderConfig) -> Result<Self, OAuthError> {
        let client_id = config
            .client_id
            .as_ref()
            .ok_or_else(|| {
                OAuthError::ConfigurationError("Fitbit client_id not configured".into())
            })?
            .clone();

        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| {
                OAuthError::ConfigurationError("Fitbit client_secret not configured".into())
            })?
            .clone();

        let redirect_uri = config
            .redirect_uri
            .clone()
            .unwrap_or_else(crate::constants::env_config::fitbit_redirect_uri);

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

#[async_trait::async_trait]
impl OAuthProvider for FitbitOAuthProvider {
    fn name(&self) -> &'static str {
        "fitbit"
    }

    async fn generate_auth_url(
        &self,
        _user_id: Uuid,
        state: String,
    ) -> Result<AuthorizationResponse, OAuthError> {
        let scope = "activity heartrate location nutrition profile settings sleep social weight";

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            crate::constants::env_config::fitbit_auth_url(),
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(&state)
        );

        Ok(AuthorizationResponse {
            authorization_url: auth_url,
            state,
            provider: "fitbit".into(),
            instructions: "Visit the authorization URL to connect your Fitbit account. Complete the OAuth flow through your web browser.".into(),
            expires_in_minutes: 10,
        })
    }

    async fn exchange_code(&self, code: &str, _state: &str) -> Result<TokenData, OAuthError> {
        let config = OAuthConfig {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            redirect_uri: self.redirect_uri.clone(),
            token_url: crate::constants::env_config::fitbit_token_url(),
            provider_name: "fitbit".into(),
            auth_method: AuthMethod::BasicAuth,
        };

        exchange_authorization_code(
            &config,
            code,
            |token_response: FitbitTokenResponse, provider| {
                let expires_at =
                    chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);

                TokenData {
                    access_token: token_response.access_token,
                    refresh_token: token_response.refresh_token,
                    expires_at,
                    scopes: token_response.scope,
                    provider: provider.into(),
                }
            },
        )
        .await
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenData, OAuthError> {
        let config = OAuthConfig {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            redirect_uri: self.redirect_uri.clone(),
            token_url: crate::constants::env_config::fitbit_token_url(),
            provider_name: "fitbit".into(),
            auth_method: AuthMethod::BasicAuth,
        };

        refresh_access_token(
            &config,
            refresh_token,
            |token_response: FitbitTokenResponse, provider| {
                let expires_at =
                    chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);

                TokenData {
                    access_token: token_response.access_token,
                    refresh_token: token_response.refresh_token,
                    expires_at,
                    scopes: token_response.scope,
                    provider: provider.into(),
                }
            },
        )
        .await
    }

    async fn revoke_token(&self, access_token: &str) -> Result<(), OAuthError> {
        revoke_access_token(
            &crate::constants::env_config::fitbit_revoke_url(),
            access_token,
            &AuthMethod::BasicAuth,
            &self.client_id,
            &self.client_secret,
        )
        .await
    }

    async fn validate_token(&self, token: &TokenData) -> Result<bool, OAuthError> {
        validate_token_expiry(token, 5)
    }
}
