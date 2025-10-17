// ABOUTME: Strava API integration and data fetching
// ABOUTME: Handles Strava authentication, activity retrieval, and data transformation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::{AuthData, FitnessProvider};
use crate::config::FitnessConfig;
use crate::constants::{api_provider_limits, env_config};
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use crate::oauth2_client::PkceParams;
use crate::pagination::{CursorPage, PaginationParams};
use crate::utils::http_client::api_client;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::sync::OnceLock;
use tracing::{error, info};

/// Configuration for Strava API integration
#[derive(Debug, Clone)]
pub struct StravaConfig {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret  
    pub client_secret: String,
    /// API base URL
    pub base_url: String,
    /// Auth URL
    pub auth_url: String,
    /// Token URL
    pub token_url: String,
}

impl Default for StravaConfig {
    fn default() -> Self {
        // OAuth credentials are now tenant-based, not environment-based
        // Use empty strings for client credentials - they should be provided via tenant configuration
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            base_url: env_config::strava_api_base(),
            auth_url: env_config::strava_auth_url(),
            token_url: env_config::strava_token_url(),
        }
    }
}

/// Global Strava configuration singleton
static STRAVA_CONFIG: OnceLock<StravaConfig> = OnceLock::new();

impl StravaConfig {
    /// Get the global Strava configuration
    pub fn global() -> &'static Self {
        STRAVA_CONFIG.get_or_init(Self::default)
    }
}

pub struct StravaProvider {
    client: Client,
    config: &'static StravaConfig,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

impl Default for StravaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl StravaProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: api_client(),
            config: StravaConfig::global(),
            access_token: None,
            refresh_token: None,
        }
    }

    #[must_use]
    pub fn with_config(config: &'static StravaConfig) -> Self {
        Self {
            client: api_client(),
            config,
            access_token: None,
            refresh_token: None,
        }
    }

    /// Get the authorization URL for Strava OAuth flow
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client ID is not configured or empty
    /// - Auth URL is malformed and cannot be parsed
    /// - URL encoding fails for any parameter
    pub fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String> {
        if self.config.client_id.is_empty() {
            return Err(anyhow::anyhow!("Client ID not configured"));
        }

        let mut url = url::Url::parse(&self.config.auth_url)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", crate::constants::oauth::STRAVA_DEFAULT_SCOPES)
            .append_pair("state", state);

        Ok(url.into())
    }

    /// Get authorization URL with PKCE support for enhanced security
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client ID is not configured or empty
    /// - Auth URL is malformed and cannot be parsed
    /// - URL encoding fails for any parameter
    /// - PKCE parameters are invalid
    pub fn get_auth_url_with_pkce(
        &self,
        redirect_uri: &str,
        state: &str,
        pkce: &PkceParams,
    ) -> Result<String> {
        if self.config.client_id.is_empty() {
            return Err(anyhow::anyhow!("Client ID not configured"));
        }

        let mut url = url::Url::parse(&self.config.auth_url)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", crate::constants::oauth::STRAVA_DEFAULT_SCOPES)
            .append_pair("state", state)
            .append_pair("code_challenge", &pkce.code_challenge)
            .append_pair("code_challenge_method", &pkce.code_challenge_method);

        Ok(url.into())
    }

    /// Exchange authorization code for access and refresh tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client credentials are not configured
    /// - HTTP request to token endpoint fails
    /// - Token exchange API returns an error response
    /// - Response cannot be parsed as JSON
    /// - Strava API returns invalid token data
    pub async fn exchange_code(&mut self, code: &str) -> Result<(String, String)> {
        if self.config.client_id.is_empty() || self.config.client_secret.is_empty() {
            return Err(anyhow::anyhow!("Client credentials not configured"));
        }

        let (token, athlete) = crate::oauth2_client::strava::exchange_strava_code(
            &self.client,
            &self.config.client_id,
            &self.config.client_secret,
            code,
        )
        .await?;

        // Store tokens without unnecessary cloning
        self.access_token = Some(token.access_token.clone()); // Safe: String ownership for struct field storage
        self.refresh_token.clone_from(&token.refresh_token);

        if let Some(ref athlete) = athlete {
            info!(
                "Authenticated as Strava athlete: {} ({})",
                athlete.id,
                athlete.username.as_deref().unwrap_or("unknown")
            );
        }

        // Return tokens - handle missing refresh token properly
        let refresh_token = token.refresh_token.unwrap_or_else(|| {
            tracing::warn!("No refresh token provided by Strava");
            String::new()
        });

        Ok((token.access_token, refresh_token))
    }

    /// Exchange authorization code with PKCE support for enhanced security
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client credentials are not configured
    /// - HTTP request to token endpoint fails
    /// - Token exchange API returns an error response
    /// - Response cannot be parsed as JSON
    /// - PKCE verification fails
    /// - Strava API returns invalid token data
    pub async fn exchange_code_with_pkce(
        &mut self,
        code: &str,
        pkce: &PkceParams,
    ) -> Result<(String, String)> {
        if self.config.client_id.is_empty() || self.config.client_secret.is_empty() {
            return Err(anyhow::anyhow!("Client credentials not configured"));
        }

        let (token, athlete) = crate::oauth2_client::strava::exchange_strava_code_with_pkce(
            &self.client,
            &self.config.client_id,
            &self.config.client_secret,
            code,
            pkce,
        )
        .await?;

        self.access_token = Some(token.access_token.clone()); // Safe: String ownership for struct field storage
        self.refresh_token.clone_from(&token.refresh_token);

        if let Some(ref athlete) = athlete {
            info!(
                "Authenticated as Strava athlete with PKCE: {} ({})",
                athlete.id,
                athlete.username.as_deref().unwrap_or("unknown")
            );
        }

        // Return tokens - handle missing refresh token properly
        let refresh_token = token.refresh_token.unwrap_or_else(|| {
            tracing::warn!("No refresh token provided by Strava");
            String::new()
        });

        Ok((token.access_token, refresh_token))
    }

    /// Refresh the access token using the stored refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No refresh token is available
    /// - Client credentials are not configured
    /// - HTTP request to token endpoint fails
    /// - Token refresh API returns an error response
    /// - Response cannot be parsed as JSON
    /// - Strava API returns invalid token data
    pub async fn refresh_access_token(&mut self) -> Result<(String, String)> {
        let refresh_token = self
            .refresh_token
            .as_ref()
            .context("No refresh token available")?;

        if self.config.client_id.is_empty() || self.config.client_secret.is_empty() {
            return Err(anyhow::anyhow!("Client credentials not configured"));
        }

        let new_token = crate::oauth2_client::strava::refresh_strava_token(
            &self.client,
            &self.config.client_id,
            &self.config.client_secret,
            refresh_token,
        )
        .await?;

        self.access_token = Some(new_token.access_token.clone()); // Safe: String ownership for struct field storage
        self.refresh_token.clone_from(&new_token.refresh_token);

        info!("Token refreshed successfully");

        // Return tokens - handle missing refresh token properly
        let refresh_token = new_token.refresh_token.unwrap_or_else(|| {
            tracing::warn!("No refresh token provided by Strava");
            String::new()
        });

        Ok((new_token.access_token, refresh_token))
    }
}

#[async_trait]
impl FitnessProvider for StravaProvider {
    /// Authenticate with the provided OAuth2 credentials
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Auth data is not OAuth2 format (Strava requires OAuth2)
    /// - Token storage fails
    async fn authenticate(&mut self, auth_data: AuthData) -> Result<()> {
        match auth_data {
            AuthData::OAuth2 {
                access_token,
                refresh_token,
                ..
            } => {
                // Only store the tokens - client credentials come from config
                self.access_token = access_token;
                self.refresh_token = refresh_token;
                Ok(())
            }
            AuthData::ApiKey(_) => Err(anyhow::anyhow!("Strava requires OAuth2 authentication")),
        }
    }

    /// Get the authenticated athlete's profile information
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated (no access token)
    /// - HTTP request to Strava API fails
    /// - API returns error response (e.g., 401 Unauthorized)
    /// - Response cannot be parsed as JSON
    /// - Strava API returns malformed athlete data
    async fn get_athlete(&self) -> Result<Athlete> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        let response: StravaAthlete = self
            .client
            .get(format!("{}/athlete", &self.config.base_url))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        Ok(Athlete {
            id: response.id.to_string(),
            username: response.username.unwrap_or_else(|| {
                tracing::debug!("No username provided by Strava for athlete {}", response.id);
                String::new()
            }),
            firstname: response.firstname,
            lastname: response.lastname,
            profile_picture: response.profile,
            provider: "strava".into(),
        })
    }

    /// Get a list of activities from Strava
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated (no access token)
    /// - HTTP request to Strava API fails
    /// - API returns error response (e.g., 401 Unauthorized, 429 Rate Limited)
    /// - Response cannot be parsed as JSON
    /// - Strava API returns malformed activity data
    /// - Network connection fails
    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        // Build query parameters without unnecessary allocations
        let per_page = limit.unwrap_or(api_provider_limits::strava::DEFAULT_ACTIVITIES_PER_PAGE);
        let page = offset.map_or(1, |o| o / per_page + 1);

        let query = [
            ("per_page", per_page.to_string()),
            ("page", page.to_string()),
        ];

        let url = format!("{}/athlete/activities", &self.config.base_url);
        info!("Fetching activities from: {} with query: {:?}", url, query);

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .query(&query)
            .send()
            .await
            .context("Failed to send request to Strava API")?;

        let status = response.status();
        info!("Strava API response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to read error response body: {}", e);
                "Unable to read error response".into()
            });
            error!("Strava API error response: {} - {}", status, error_text);

            // Check if it's an authentication error and we have a refresh token
            if status == 401 && self.refresh_token.is_some() {
                info!("Access token expired, attempting to refresh...");
                // Note: This would require mutable self to refresh the token
                return Err(anyhow::anyhow!(
                    "Access token expired. Strava API error: {} - {}",
                    status,
                    error_text
                ));
            }

            return Err(anyhow::anyhow!(
                "Strava API returned error: {} - {}",
                status,
                error_text
            ));
        }

        // Get response text first for debugging
        let response_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        info!("Strava API response length: {} bytes", response_text.len());
        if response_text.len() < crate::constants::logging::MAX_RESPONSE_BODY_LOG_SIZE {
            info!("Strava API response body: {}", response_text);
        } else {
            info!(
                "Strava API response body (truncated): {}...",
                &response_text[..500]
            );
        }

        // Try to parse JSON
        let activities: Vec<StravaActivity> =
            serde_json::from_str(&response_text).with_context(|| {
                format!(
                    "Failed to parse Strava activities JSON. Response: {}",
                    if response_text.len() < 500 {
                        &response_text
                    } else {
                        &response_text[..500]
                    }
                )
            })?;

        info!(
            "Successfully parsed {} activities from Strava",
            activities.len()
        );

        Ok(activities
            .into_iter()
            .map(std::convert::Into::into)
            .collect())
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> Result<CursorPage<Activity>> {
        // Stub implementation: delegate to offset-based pagination
        let activities = self.get_activities(Some(params.limit), None).await?;
        Ok(CursorPage::new(activities, None, None, false))
    }

    /// Get a specific activity by ID from Strava
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated (no access token)
    /// - HTTP request to Strava API fails
    /// - API returns error response (e.g., 401 Unauthorized, 404 Not Found)
    /// - Response cannot be parsed as JSON
    /// - Strava API returns malformed activity data
    /// - Activity ID is invalid or inaccessible
    async fn get_activity(&self, id: &str) -> Result<Activity> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        let response: StravaActivity = self
            .client
            .get(format!("{}/activities/{}", &self.config.base_url, id))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        Ok(response.into())
    }

    /// Get athlete statistics from Strava
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated (no access token)
    /// - HTTP request to Strava API fails
    /// - API returns error response (e.g., 401 Unauthorized)
    /// - Response cannot be parsed as JSON
    /// - Both athlete stats API and activities fallback fail
    /// - Strava API returns malformed stats data
    async fn get_stats(&self) -> Result<Stats> {
        // Try Strava's athlete stats endpoint first
        if let Ok(strava_stats) = self.get_strava_athlete_stats().await {
            return Ok(strava_stats);
        }

        // Fallback: Calculate from recent activities (limited to avoid rate limits)
        let activities = self.get_activities(Some(100), None).await?;

        let total_activities = activities.len() as u64;
        let total_distance = activities.iter().filter_map(|a| a.distance_meters).sum();
        let total_duration = activities.iter().map(|a| a.duration_seconds).sum();
        let total_elevation_gain = activities.iter().filter_map(|a| a.elevation_gain).sum();

        Ok(Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        })
    }

    /// Get personal records from Strava
    ///
    /// # Errors
    ///
    /// Returns errors for:
    /// - Authentication failures
    /// - API communication issues  
    /// - Data parsing errors
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        // Personal records require analysis of activities to determine bests
        // This would involve fetching activities and calculating personal bests
        // Strava API does not provide direct PR endpoints - requires activity analysis
        tracing::debug!("Personal records require activity analysis - returning empty set");
        Ok(vec![])
    }

    fn provider_name(&self) -> &'static str {
        "Strava"
    }
}

impl StravaProvider {
    /// Try to get stats from Strava's athlete stats endpoint
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated (no access token)
    /// - HTTP request to get athlete profile fails
    /// - HTTP request to get athlete stats fails
    /// - API returns error response (e.g., 401 Unauthorized, 403 Forbidden)
    /// - Response cannot be parsed as JSON
    /// - Strava API returns malformed athlete or stats data
    async fn get_strava_athlete_stats(&self) -> Result<Stats> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        // Get athlete ID first
        let athlete: StravaAthlete = self
            .client
            .get(format!("{}/athlete", &self.config.base_url))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        // Get athlete stats
        let response: StravaAthleteStats = self
            .client
            .get(format!(
                "{}/athletes/{}/stats",
                &self.config.base_url,
                athlete.id
            ))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        // Convert Strava stats to our format
        Ok(Stats {
            total_activities: response.all_ride_totals.count + response.all_run_totals.count,
            total_distance: response.all_ride_totals.distance + response.all_run_totals.distance,
            total_duration: response.all_ride_totals.moving_time
                + response.all_run_totals.moving_time,
            total_elevation_gain: response.all_ride_totals.elevation_gain
                + response.all_run_totals.elevation_gain,
        })
    }
}

#[derive(Debug, Deserialize)]
struct StravaAthlete {
    id: u64,
    username: Option<String>,
    firstname: Option<String>,
    lastname: Option<String>,
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StravaActivity {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    activity_type: String,
    start_date: DateTime<Utc>,
    elapsed_time: u64,
    distance: Option<f64>,
    total_elevation_gain: Option<f64>,
    average_heartrate: Option<f32>,
    max_heartrate: Option<f32>,
    average_speed: Option<f64>,
    max_speed: Option<f64>,
    start_latlng: Option<Vec<f64>>, // [latitude, longitude]
}

impl From<StravaActivity> for Activity {
    fn from(strava: StravaActivity) -> Self {
        // Use default fitness config for sport type mapping
        let fitness_config = FitnessConfig::default();

        // Extract GPS coordinates from start_latlng array
        let (start_latitude, start_longitude) =
            strava.start_latlng.map_or((None, None), |coords| {
                if coords.len() >= 2 {
                    (Some(coords[0]), Some(coords[1]))
                } else {
                    (None, None)
                }
            });

        Self {
            id: strava.id.to_string(),
            name: strava.name,
            sport_type: SportType::from_provider_string(&strava.activity_type, &fitness_config),
            start_date: strava.start_date,
            duration_seconds: strava.elapsed_time,
            distance_meters: strava.distance,
            elevation_gain: strava.total_elevation_gain,
            average_heart_rate: strava.average_heartrate.and_then(|hr| {
                if hr.is_finite() && (0.0..=f64::from(crate::constants::physiology::MAX_NORMAL_HR)).contains(&hr) {
                    let rounded = hr.round();
                    let hr_string = format!("{rounded:.0}");
                    hr_string.parse::<u32>().ok()
                } else {
                    None
                }
            }),
            max_heart_rate: strava.max_heartrate.and_then(|hr| {
                if hr.is_finite() && (0.0..=f64::from(crate::constants::physiology::MAX_NORMAL_HR)).contains(&hr) {
                    let rounded = hr.round();
                    let hr_string = format!("{rounded:.0}");
                    hr_string.parse::<u32>().ok()
                } else {
                    None
                }
            }),
            average_speed: strava.average_speed,
            max_speed: strava.max_speed,
            calories: None,
            steps: None,            // Strava doesn't provide step data
            heart_rate_zones: None, // Strava doesn't provide zone breakdown data

            // Advanced metrics - all None for basic Strava data
            average_power: None,
            max_power: None,
            normalized_power: None,
            power_zones: None,
            ftp: None,
            average_cadence: None,
            max_cadence: None,
            hrv_score: None,
            recovery_heart_rate: None,
            temperature: None,
            humidity: None,
            average_altitude: None,
            wind_speed: None,
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            running_power: None,
            breathing_rate: None,
            spo2: None,
            training_stress_score: None,
            intensity_factor: None,
            suffer_score: None,
            time_series_data: None,

            start_latitude,
            start_longitude,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: "strava".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct StravaAthleteStats {
    all_ride_totals: StravaTotals,
    all_run_totals: StravaTotals,
}

#[derive(Debug, Deserialize)]
struct StravaTotals {
    count: u64,
    distance: f64,
    moving_time: u64,
    elevation_gain: f64,
}
