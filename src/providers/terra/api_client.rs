// ABOUTME: Terra REST API client for authentication, user management, and historical data
// ABOUTME: Handles API key auth, widget sessions, and on-demand data requests
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Terra REST API client
//!
//! This module provides a client for interacting with Terra's REST API endpoints.
//! While Terra primarily uses webhooks for data delivery, the REST API is used for:
//! - Generating authentication widget sessions
//! - Requesting historical data
//! - Managing user connections
//! - Deauthenticating users

use crate::providers::errors::ProviderError;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Terra API configuration
#[derive(Debug, Clone)]
pub struct TerraApiConfig {
    /// Terra API key (from dashboard)
    pub api_key: String,
    /// Terra dev ID (from dashboard)
    pub dev_id: String,
    /// Base URL for Terra API
    pub base_url: String,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for TerraApiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            dev_id: String::new(),
            base_url: "https://api.tryterra.co/v2".to_owned(),
            timeout: Duration::from_secs(30),
        }
    }
}

/// Terra API client for REST operations
pub struct TerraApiClient {
    config: TerraApiConfig,
    client: Client,
}

/// Response from widget session generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSessionResponse {
    /// Status of the request
    pub status: String,
    /// Session ID for the widget
    pub session_id: Option<String>,
    /// URL to redirect user to for authentication
    pub url: Option<String>,
    /// Error message if any
    pub message: Option<String>,
}

/// Response from user deauthentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeauthResponse {
    /// Status of the request
    pub status: String,
    /// Message
    pub message: Option<String>,
}

/// Response from user info request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoResponse {
    /// Status
    pub status: String,
    /// User info
    pub user: Option<TerraUserInfo>,
}

/// Terra user info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraUserInfo {
    /// Terra user ID
    pub user_id: String,
    /// Provider name
    pub provider: String,
    /// Last webhook update
    pub last_webhook_update: Option<String>,
    /// Reference ID
    pub reference_id: Option<String>,
    /// Scopes granted
    pub scopes: Option<String>,
}

/// Historical data request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataRequest {
    /// Terra user ID
    pub user_id: String,
    /// Start date for data range
    pub start_date: DateTime<Utc>,
    /// End date for data range
    pub end_date: DateTime<Utc>,
    /// Whether to send data to webhook (true) or return in response (false)
    pub to_webhook: bool,
    /// Data types to fetch (activity, sleep, body, daily, nutrition)
    pub data_types: Vec<String>,
}

/// Response from historical data request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataResponse {
    /// Status
    pub status: String,
    /// Message
    pub message: Option<String>,
    /// Data (if `to_webhook` = false and data is small enough)
    pub data: Option<serde_json::Value>,
}

/// Response from list users/subscriptions request
#[derive(Debug, Clone, Deserialize)]
struct SubscriptionsResponse {
    users: Vec<TerraUserInfo>,
}

impl TerraApiClient {
    /// Create a new Terra API client
    #[must_use]
    pub fn new(config: TerraApiConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Generate a widget session URL for user authentication
    ///
    /// # Arguments
    /// * `reference_id` - Your system's user ID to associate with the Terra user
    /// * `providers` - Optional list of providers to show (e.g., `["GARMIN", "FITBIT"]`)
    /// * `auth_success_redirect_url` - URL to redirect after successful auth
    /// * `auth_failure_redirect_url` - URL to redirect after failed auth
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails
    pub async fn generate_widget_session(
        &self,
        reference_id: &str,
        providers: Option<Vec<&str>>,
        auth_success_redirect_url: Option<&str>,
        auth_failure_redirect_url: Option<&str>,
    ) -> Result<WidgetSessionResponse, ProviderError> {
        let url = format!("{}/auth/generateWidgetSession", self.config.base_url);

        let mut body = serde_json::json!({
            "reference_id": reference_id,
        });

        if let Some(provs) = providers {
            body["providers"] = serde_json::json!(provs);
        }

        if let Some(success_url) = auth_success_redirect_url {
            body["auth_success_redirect_url"] = serde_json::json!(success_url);
        }

        if let Some(failure_url) = auth_failure_redirect_url {
            body["auth_failure_redirect_url"] = serde_json::json!(failure_url);
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("dev-id", &self.config.dev_id)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::ApiError {
                provider: "terra".to_owned(),
                status_code: status.as_u16(),
                message: text,
                retryable: status.is_server_error(),
            });
        }

        serde_json::from_str(&text).map_err(|e| ProviderError::ParseError {
            provider: "terra".to_owned(),
            field: "widget_session_response",
            source: e,
        })
    }

    /// Deauthenticate a user from Terra
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails
    pub async fn deauthenticate_user(
        &self,
        user_id: &str,
    ) -> Result<DeauthResponse, ProviderError> {
        let url = format!("{}/auth/deauthenticateUser", self.config.base_url);

        let response = self
            .client
            .delete(&url)
            .header("x-api-key", &self.config.api_key)
            .header("dev-id", &self.config.dev_id)
            .query(&[("user_id", user_id)])
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::ApiError {
                provider: "terra".to_owned(),
                status_code: status.as_u16(),
                message: text,
                retryable: status.is_server_error(),
            });
        }

        serde_json::from_str(&text).map_err(|e| ProviderError::ParseError {
            provider: "terra".to_owned(),
            field: "deauth_response",
            source: e,
        })
    }

    /// Get user info from Terra
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails
    pub async fn get_user_info(&self, user_id: &str) -> Result<UserInfoResponse, ProviderError> {
        let url = format!("{}/userInfo", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.config.api_key)
            .header("dev-id", &self.config.dev_id)
            .query(&[("user_id", user_id)])
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::ApiError {
                provider: "terra".to_owned(),
                status_code: status.as_u16(),
                message: text,
                retryable: status.is_server_error(),
            });
        }

        serde_json::from_str(&text).map_err(|e| ProviderError::ParseError {
            provider: "terra".to_owned(),
            field: "user_info_response",
            source: e,
        })
    }

    /// Request historical data for a user
    ///
    /// For date ranges > 28 days, Terra sends data asynchronously via webhook
    /// regardless of the `to_webhook` setting.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails
    pub async fn request_historical_data(
        &self,
        request: &HistoricalDataRequest,
    ) -> Result<HistoricalDataResponse, ProviderError> {
        // Build the URL with data type
        // Terra uses separate endpoints for each data type
        let data_type = request
            .data_types
            .first()
            .map_or("activity", String::as_str);
        let url = format!("{}/{}", self.config.base_url, data_type);

        let start_date_str = request.start_date.format("%Y-%m-%d").to_string();
        let end_date_str = request.end_date.format("%Y-%m-%d").to_string();

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.config.api_key)
            .header("dev-id", &self.config.dev_id)
            .query(&[
                ("user_id", &request.user_id),
                ("start_date", &start_date_str),
                ("end_date", &end_date_str),
                ("to_webhook", &request.to_webhook.to_string()),
            ])
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            // Handle rate limiting
            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimitExceeded {
                    provider: "terra".to_owned(),
                    retry_after_secs: 60,
                    limit_type: "API rate limit".to_owned(),
                });
            }

            return Err(ProviderError::ApiError {
                provider: "terra".to_owned(),
                status_code: status.as_u16(),
                message: text,
                retryable: status.is_server_error(),
            });
        }

        serde_json::from_str(&text).map_err(|e| ProviderError::ParseError {
            provider: "terra".to_owned(),
            field: "historical_data_response",
            source: e,
        })
    }

    /// List all users connected to your Terra app
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails
    pub async fn list_users(&self) -> Result<Vec<TerraUserInfo>, ProviderError> {
        let url = format!("{}/subscriptions", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.config.api_key)
            .header("dev-id", &self.config.dev_id)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ProviderError::ApiError {
                provider: "terra".to_owned(),
                status_code: status.as_u16(),
                message: text,
                retryable: status.is_server_error(),
            });
        }

        let parsed: SubscriptionsResponse =
            serde_json::from_str(&text).map_err(|e| ProviderError::ParseError {
                provider: "terra".to_owned(),
                field: "subscriptions_response",
                source: e,
            })?;

        Ok(parsed.users)
    }
}
