// ABOUTME: Generic OAuth utility functions to eliminate code duplication
// ABOUTME: Provides common OAuth token exchange and refresh patterns for all providers
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::oauth::{OAuthError, TokenData};
use crate::utils::http_client::oauth_client;
use base64::{engine::general_purpose, Engine as _};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

/// OAuth authentication method for token requests
pub enum AuthMethod {
    /// Send client credentials as form parameters
    FormParams,
    /// Send client credentials as Basic Authorization header  
    BasicAuth,
}

/// Configuration for generic OAuth operations
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub token_url: String,
    pub provider_name: String,
    pub auth_method: AuthMethod,
}

/// Generic OAuth token exchange function
///
/// # Errors
///
/// Returns an error if:
/// - HTTP request fails
/// - Token endpoint returns error response
/// - JSON parsing fails
/// - Provider-specific token conversion fails
pub async fn exchange_authorization_code<T>(
    config: &OAuthConfig,
    authorization_code: &str,
    convert_token: impl Fn(T, &str) -> TokenData,
) -> Result<TokenData, OAuthError>
where
    T: DeserializeOwned,
{
    let client = oauth_client();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", authorization_code);

    let mut request = client.post(&config.token_url);

    match config.auth_method {
        AuthMethod::FormParams => {
            params.insert("client_id", &config.client_id);
            params.insert("client_secret", &config.client_secret);
            request = request.form(&params);
        }
        AuthMethod::BasicAuth => {
            let auth_header = general_purpose::STANDARD
                .encode(format!("{}:{}", config.client_id, config.client_secret));
            params.insert("redirect_uri", &config.redirect_uri);
            request = request
                .header("Authorization", format!("Basic {auth_header}"))
                .form(&params);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

    let token_response: T = serde_json::from_str(&response_text)
        .map_err(|e| OAuthError::TokenExchangeFailed(format!("Parse error: {e}")))?;

    Ok(convert_token(token_response, &config.provider_name))
}

/// Generic OAuth token refresh function
///
/// # Errors
///
/// Returns an error if:
/// - HTTP request fails  
/// - Token endpoint returns error response
/// - JSON parsing fails
/// - Provider-specific token conversion fails
pub async fn refresh_access_token<T>(
    config: &OAuthConfig,
    refresh_token: &str,
    convert_token: impl Fn(T, &str) -> TokenData,
) -> Result<TokenData, OAuthError>
where
    T: DeserializeOwned,
{
    let client = oauth_client();

    let mut params = HashMap::new();
    params.insert("grant_type", "refresh_token");
    params.insert("refresh_token", refresh_token);

    let mut request = client.post(&config.token_url);

    match config.auth_method {
        AuthMethod::FormParams => {
            params.insert("client_id", &config.client_id);
            params.insert("client_secret", &config.client_secret);
            request = request.form(&params);
        }
        AuthMethod::BasicAuth => {
            let auth_header = general_purpose::STANDARD
                .encode(format!("{}:{}", config.client_id, config.client_secret));
            request = request
                .header("Authorization", format!("Basic {auth_header}"))
                .form(&params);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

    let token_response: T = serde_json::from_str(&response_text)
        .map_err(|e| OAuthError::TokenRefreshFailed(format!("Parse error: {e}")))?;

    Ok(convert_token(token_response, &config.provider_name))
}

/// Generic OAuth token revocation function
///
/// # Errors
///
/// Returns an error if:
/// - HTTP request fails
/// - Revocation endpoint returns error response  
pub async fn revoke_access_token(
    revoke_url: &str,
    access_token: &str,
    auth_method: &AuthMethod,
    client_id: &str,
    client_secret: &str,
) -> Result<(), OAuthError> {
    let client = oauth_client();

    let mut request = client.post(revoke_url);

    match auth_method {
        AuthMethod::FormParams => {
            request = request.form(&[
                ("access_token", access_token),
                ("client_id", client_id),
                ("client_secret", client_secret),
            ]);
        }
        AuthMethod::BasicAuth => {
            let auth_header =
                general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
            request = request
                .header("Authorization", format!("Basic {auth_header}"))
                .form(&[("token", access_token)]);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?;

    if !response.status().is_success() {
        return Err(OAuthError::TokenRefreshFailed(
            "Failed to revoke token".into(),
        ));
    }

    Ok(())
}

/// Generic token validation with configurable buffer time
///
/// # Errors
///
/// This function does not return errors in normal operation
pub fn validate_token_expiry(token: &TokenData, buffer_minutes: i64) -> Result<bool, OAuthError> {
    let now = chrono::Utc::now();
    let buffer = chrono::Duration::minutes(buffer_minutes);

    Ok(token.expires_at > (now + buffer))
}
