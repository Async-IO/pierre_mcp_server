// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OAuth2Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

impl OAuth2Token {
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at <= Utc::now()
        } else {
            false
        }
    }
    
    #[allow(dead_code)]
    pub fn will_expire_soon(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at <= Utc::now() + Duration::minutes(5)
        } else {
            false
        }
    }
}

#[allow(dead_code)]
pub struct OAuth2Client {
    config: OAuth2Config,
    client: reqwest::Client,
}

impl OAuth2Client {
    pub fn new(config: OAuth2Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
    
    pub fn get_authorization_url(&self, state: &str) -> Result<String> {
        let mut url = Url::parse(&self.config.auth_url)
            .context("Invalid auth URL")?;
        
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", state);
        
        Ok(url.to_string())
    }
    
    pub async fn exchange_code(&self, code: &str) -> Result<OAuth2Token> {
        let params = [
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("code", &code.to_string()),
            ("grant_type", &"authorization_code".to_string()),
            ("redirect_uri", &self.config.redirect_uri),
        ];
        
        let response: TokenResponse = self.client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(self.token_from_response(response))
    }
    
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuth2Token> {
        let params = [
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("refresh_token", &refresh_token.to_string()),
            ("grant_type", &"refresh_token".to_string()),
        ];
        
        let response: TokenResponse = self.client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(self.token_from_response(response))
    }
    
    fn token_from_response(&self, response: TokenResponse) -> OAuth2Token {
        let expires_at = response.expires_in.map(|seconds| {
            Utc::now() + Duration::seconds(seconds as i64)
        });
        
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
#[allow(dead_code)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

// Strava-specific OAuth2 extensions
pub mod strava {
    use super::*;
    
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    pub struct StravaTokenResponse {
        pub token_type: String,
        pub expires_at: i64,
        pub expires_in: i64,
        pub refresh_token: String,
        pub access_token: String,
        pub athlete: Option<StravaAthleteSummary>,
    }
    
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    pub struct StravaAthleteSummary {
        pub id: i64,
        pub username: Option<String>,
        pub firstname: Option<String>,
        pub lastname: Option<String>,
    }
    
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
            expires_at: Some(DateTime::from_timestamp(response.expires_at, 0)
                .unwrap_or_else(Utc::now)),
            refresh_token: Some(response.refresh_token),
            scope: None,
        };
        
        Ok((token, response.athlete))
    }
    
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
            expires_at: Some(DateTime::from_timestamp(response.expires_at, 0)
                .unwrap_or_else(Utc::now)),
            refresh_token: Some(response.refresh_token),
            scope: None,
        })
    }
}