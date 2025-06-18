// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth Provider Implementations
//!
//! Concrete implementations of OAuth providers for different fitness platforms.

use super::{AuthorizationResponse, OAuthError, OAuthProvider, TokenData};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
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
    pub fn new() -> Result<Self, OAuthError> {
        let client_id = std::env::var("STRAVA_CLIENT_ID")
            .map_err(|_| OAuthError::ConfigurationError("STRAVA_CLIENT_ID not set".to_string()))?;

        let client_secret = std::env::var("STRAVA_CLIENT_SECRET").map_err(|_| {
            OAuthError::ConfigurationError("STRAVA_CLIENT_SECRET not set".to_string())
        })?;

        let redirect_uri = std::env::var("STRAVA_REDIRECT_URI").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}/oauth/callback/strava",
                crate::constants::ports::DEFAULT_HTTP_PORT
            )
        });

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

#[async_trait::async_trait]
impl OAuthProvider for StravaOAuthProvider {
    fn name(&self) -> &str {
        "strava"
    }

    async fn generate_auth_url(
        &self,
        _user_id: Uuid,
        state: String,
    ) -> Result<AuthorizationResponse, OAuthError> {
        let scope = "read,activity:read_all";

        let auth_url = format!(
            "https://www.strava.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(&state)
        );

        Ok(AuthorizationResponse {
            authorization_url: auth_url,
            state,
            provider: "strava".to_string(),
            instructions: "Visit the authorization URL to connect your Strava account. Complete the OAuth flow through your web browser.".to_string(),
            expires_in_minutes: 10,
        })
    }

    async fn exchange_code(&self, code: &str, _state: &str) -> Result<TokenData, OAuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ];

        let response = client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

        let token_response: StravaTokenResponse = serde_json::from_str(&response_text)
            .map_err(|e| OAuthError::TokenExchangeFailed(format!("Parse error: {}", e)))?;

        let expires_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(token_response.expires_at, 0)
                .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(6));

        Ok(TokenData {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scopes: token_response
                .scope
                .unwrap_or_else(|| "read,activity:read_all".to_string()),
            provider: "strava".to_string(),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenData, OAuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response = client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

        let token_response: StravaTokenResponse = serde_json::from_str(&response_text)
            .map_err(|e| OAuthError::TokenRefreshFailed(format!("Parse error: {}", e)))?;

        let expires_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(token_response.expires_at, 0)
                .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(6));

        Ok(TokenData {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scopes: token_response
                .scope
                .unwrap_or_else(|| "read,activity:read_all".to_string()),
            provider: "strava".to_string(),
        })
    }

    async fn revoke_token(&self, access_token: &str) -> Result<(), OAuthError> {
        let client = reqwest::Client::new();

        let response = client
            .post("https://www.strava.com/oauth/deauthorize")
            .form(&[("access_token", access_token)])
            .send()
            .await
            .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OAuthError::TokenRefreshFailed(
                "Failed to revoke token".to_string(),
            ));
        }

        Ok(())
    }

    async fn validate_token(&self, token: &TokenData) -> Result<bool, OAuthError> {
        // Check if token is expired (with 5 minute buffer)
        let now = chrono::Utc::now();
        let buffer = chrono::Duration::minutes(5);

        Ok(token.expires_at > (now + buffer))
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
    pub fn new() -> Result<Self, OAuthError> {
        let client_id = std::env::var("FITBIT_CLIENT_ID")
            .map_err(|_| OAuthError::ConfigurationError("FITBIT_CLIENT_ID not set".to_string()))?;

        let client_secret = std::env::var("FITBIT_CLIENT_SECRET").map_err(|_| {
            OAuthError::ConfigurationError("FITBIT_CLIENT_SECRET not set".to_string())
        })?;

        let redirect_uri = std::env::var("FITBIT_REDIRECT_URI").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}/oauth/callback/fitbit",
                crate::constants::ports::DEFAULT_HTTP_PORT
            )
        });

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

#[async_trait::async_trait]
impl OAuthProvider for FitbitOAuthProvider {
    fn name(&self) -> &str {
        "fitbit"
    }

    async fn generate_auth_url(
        &self,
        _user_id: Uuid,
        state: String,
    ) -> Result<AuthorizationResponse, OAuthError> {
        let scope = "activity heartrate location nutrition profile settings sleep social weight";

        let auth_url = format!(
            "https://www.fitbit.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(&state)
        );

        Ok(AuthorizationResponse {
            authorization_url: auth_url,
            state,
            provider: "fitbit".to_string(),
            instructions: "Visit the authorization URL to connect your Fitbit account. Complete the OAuth flow through your web browser.".to_string(),
            expires_in_minutes: 10,
        })
    }

    async fn exchange_code(&self, code: &str, _state: &str) -> Result<TokenData, OAuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", self.client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("code", code),
        ];

        let auth_header =
            general_purpose::STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));

        let response = client
            .post("https://api.fitbit.com/oauth2/token")
            .header("Authorization", format!("Basic {}", auth_header))
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

        let token_response: FitbitTokenResponse = serde_json::from_str(&response_text)
            .map_err(|e| OAuthError::TokenExchangeFailed(format!("Parse error: {}", e)))?;

        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);

        Ok(TokenData {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scopes: token_response.scope,
            provider: "fitbit".to_string(),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenData, OAuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        let auth_header =
            general_purpose::STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));

        let response = client
            .post("https://api.fitbit.com/oauth2/token")
            .header("Authorization", format!("Basic {}", auth_header))
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

        let token_response: FitbitTokenResponse = serde_json::from_str(&response_text)
            .map_err(|e| OAuthError::TokenRefreshFailed(format!("Parse error: {}", e)))?;

        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);

        Ok(TokenData {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scopes: token_response.scope,
            provider: "fitbit".to_string(),
        })
    }

    async fn revoke_token(&self, access_token: &str) -> Result<(), OAuthError> {
        let client = reqwest::Client::new();

        let auth_header =
            general_purpose::STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));

        let response = client
            .post("https://api.fitbit.com/oauth2/revoke")
            .header("Authorization", format!("Basic {}", auth_header))
            .form(&[("token", access_token)])
            .send()
            .await
            .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OAuthError::TokenRefreshFailed(
                "Failed to revoke token".to_string(),
            ));
        }

        Ok(())
    }

    async fn validate_token(&self, token: &TokenData) -> Result<bool, OAuthError> {
        // Check if token is expired (with 5 minute buffer)
        let now = chrono::Utc::now();
        let buffer = chrono::Duration::minutes(5);

        Ok(token.expires_at > (now + buffer))
    }
}
