// ABOUTME: OAuth2 client implementation for fitness platform authentication
// ABOUTME: Generic OAuth2 client supporting multiple fitness platform providers
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub use_pkce: bool,
}

/// `PKCE` (Proof Key for Code Exchange) parameters for enhanced `OAuth2` security
#[derive(Debug, Clone)]
pub struct PkceParams {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl PkceParams {
    /// Generate `PKCE` parameters with `S256` challenge method
    #[must_use]
    pub fn generate() -> Self {
        // Generate a cryptographically secure random code verifier (43-128 characters)
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut rng = rand::thread_rng();
        let code_verifier: String = (0..128)
            .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
            .collect();

        // Create S256 code challenge
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

impl OAuth2Token {
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    }

    #[must_use]
    pub fn will_expire_soon(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now() + Duration::minutes(5))
    }
}

pub struct OAuth2Client {
    config: OAuth2Config,
    client: reqwest::Client,
}

impl OAuth2Client {
    #[must_use]
    pub fn new(config: OAuth2Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Get authorization URL
    ///
    /// # Errors
    ///
    /// Returns an error if the authorization URL is malformed
    pub fn get_authorization_url(&self, state: &str) -> Result<String> {
        let mut url = Url::parse(&self.config.auth_url).context("Invalid auth URL")?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", state);

        Ok(url.to_string())
    }

    /// Get authorization `URL` with `PKCE` support
    ///
    /// # Errors
    ///
    /// Returns an error if the authorization URL is malformed
    pub fn get_authorization_url_with_pkce(
        &self,
        state: &str,
        pkce: &PkceParams,
    ) -> Result<String> {
        let mut url = Url::parse(&self.config.auth_url).context("Invalid auth URL")?;

        let mut query_pairs = url.query_pairs_mut();
        query_pairs
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", state);

        if self.config.use_pkce {
            query_pairs
                .append_pair("code_challenge", &pkce.code_challenge)
                .append_pair("code_challenge_method", &pkce.code_challenge_method);
        }

        drop(query_pairs);
        Ok(url.to_string())
    }

    /// Exchange authorization code for tokens
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails or response is invalid
    pub async fn exchange_code(&self, code: &str) -> Result<OAuth2Token> {
        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.config.redirect_uri.as_str()),
        ];

        let response: TokenResponse = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        Ok(Self::token_from_response(response))
    }

    /// Exchange authorization code with `PKCE` support
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails or response is invalid
    pub async fn exchange_code_with_pkce(
        &self,
        code: &str,
        pkce: &PkceParams,
    ) -> Result<OAuth2Token> {
        let mut params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.config.redirect_uri.as_str()),
        ];

        if self.config.use_pkce {
            params.push(("code_verifier", &pkce.code_verifier));
        }

        let response: TokenResponse = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        Ok(Self::token_from_response(response))
    }

    /// Refresh an expired access token
    ///
    /// # Errors
    ///
    /// Returns an error if the token refresh request fails or response is invalid
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuth2Token> {
        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response: TokenResponse = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        Ok(Self::token_from_response(response))
    }

    #[must_use]
    fn token_from_response(response: TokenResponse) -> OAuth2Token {
        let expires_at = response
            .expires_in
            .map(|seconds| Utc::now() + Duration::seconds(i64::try_from(seconds).unwrap_or(3600)));

        OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at,
            refresh_token: response.refresh_token,
            scope: response.scope,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

// Strava-specific OAuth2 extensions
pub mod strava {
    use super::{DateTime, Deserialize, OAuth2Token, PkceParams, Result, Utc};

    #[derive(Debug, Deserialize)]
    pub struct StravaTokenResponse {
        pub token_type: String,
        pub expires_at: i64,
        pub expires_in: i64,
        pub refresh_token: String,
        pub access_token: String,
        pub athlete: Option<StravaAthleteSummary>,
    }

    #[derive(Debug, Deserialize)]
    pub struct StravaAthleteSummary {
        pub id: i64,
        pub username: Option<String>,
        pub firstname: Option<String>,
        pub lastname: Option<String>,
    }

    /// Exchange Strava authorization code for tokens and athlete info
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails or response is invalid
    pub async fn exchange_strava_code(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        code: &str,
    ) -> Result<(OAuth2Token, Option<StravaAthleteSummary>)> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
        ];

        let response: StravaTokenResponse = client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let token = OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at: Some(
                DateTime::from_timestamp(response.expires_at, 0).unwrap_or_else(Utc::now),
            ),
            refresh_token: Some(response.refresh_token),
            scope: None,
        };

        Ok((token, response.athlete))
    }

    /// Exchange Strava authorization code with `PKCE` support
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails or response is invalid
    pub async fn exchange_strava_code_with_pkce(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        code: &str,
        pkce: &PkceParams,
    ) -> Result<(OAuth2Token, Option<StravaAthleteSummary>)> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("code_verifier", &pkce.code_verifier),
        ];

        let response: StravaTokenResponse = client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let token = OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at: Some(
                DateTime::from_timestamp(response.expires_at, 0).unwrap_or_else(Utc::now),
            ),
            refresh_token: Some(response.refresh_token),
            scope: None,
        };

        Ok((token, response.athlete))
    }

    /// Refresh Strava access token
    ///
    /// # Errors
    ///
    /// Returns an error if the token refresh request fails or response is invalid
    pub async fn refresh_strava_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuth2Token> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response: StravaTokenResponse = client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        Ok(OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at: Some(
                DateTime::from_timestamp(response.expires_at, 0).unwrap_or_else(Utc::now),
            ),
            refresh_token: Some(response.refresh_token),
            scope: None,
        })
    }
}

// Fitbit-specific OAuth2 extensions
pub mod fitbit {
    use super::{Deserialize, Duration, OAuth2Token, PkceParams, Result, Utc};

    #[derive(Debug, Deserialize)]
    pub struct FitbitTokenResponse {
        pub access_token: String,
        pub expires_in: i64,
        pub refresh_token: String,
        pub scope: String,
        pub token_type: String,
        pub user_id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct FitbitUserInfo {
        pub user_id: String,
    }

    /// Exchange Fitbit authorization code for tokens
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails or response is invalid
    pub async fn exchange_fitbit_code(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<(OAuth2Token, Option<FitbitUserInfo>)> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];

        let response: FitbitTokenResponse = client
            .post("https://api.fitbit.com/oauth2/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let token = OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at: Some(Utc::now() + Duration::seconds(response.expires_in)),
            refresh_token: Some(response.refresh_token),
            scope: Some(response.scope),
        };

        let user_info = FitbitUserInfo {
            user_id: response.user_id,
        };

        Ok((token, Some(user_info)))
    }

    /// Exchange Fitbit authorization code with `PKCE` support
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails or response is invalid
    pub async fn exchange_fitbit_code_with_pkce(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: &str,
        pkce: &PkceParams,
    ) -> Result<(OAuth2Token, Option<FitbitUserInfo>)> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
            ("code_verifier", &pkce.code_verifier),
        ];

        let response: FitbitTokenResponse = client
            .post("https://api.fitbit.com/oauth2/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let token = OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at: Some(Utc::now() + Duration::seconds(response.expires_in)),
            refresh_token: Some(response.refresh_token),
            scope: Some(response.scope),
        };

        let user_info = FitbitUserInfo {
            user_id: response.user_id,
        };

        Ok((token, Some(user_info)))
    }

    /// Refresh Fitbit access token
    ///
    /// # Errors
    ///
    /// Returns an error if the token refresh request fails or response is invalid
    pub async fn refresh_fitbit_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuth2Token> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response: FitbitTokenResponse = client
            .post("https://api.fitbit.com/oauth2/token")
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        Ok(OAuth2Token {
            access_token: response.access_token,
            token_type: response.token_type,
            expires_at: Some(Utc::now() + Duration::seconds(response.expires_in)),
            refresh_token: Some(response.refresh_token),
            scope: Some(response.scope),
        })
    }
}
