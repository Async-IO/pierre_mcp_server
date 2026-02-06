// ABOUTME: Strava API integration and data fetching
// ABOUTME: Handles Strava authentication, activity retrieval, and data transformation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::{ActivityQueryParams, AuthData, FitnessProvider};
use crate::config::FitnessConfig;
use crate::constants::api_provider_limits;
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use crate::oauth2_client::client::PkceParams;
use crate::pagination::{CursorPage, PaginationParams};
use crate::providers::errors::ProviderError;
use crate::utils::http_client::api_client;
use crate::errors::{AppError, AppResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::sync::OnceLock;
use tracing::{debug, error, info, trace, warn};

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
        let config = crate::constants::get_server_config();
        // OAuth credentials are now tenant-based, not environment-based
        // Use empty strings for client credentials - they should be provided via tenant configuration
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            base_url: config.map_or_else(
                || "https://www.strava.com/api/v3".to_owned(),
                |c| c.external_services.strava_api.base_url.clone()
            ),
            auth_url: config.map_or_else(
                || "https://www.strava.com/oauth/authorize".to_owned(),
                |c| c.external_services.strava_api.auth_url.clone()
            ),
            token_url: config.map_or_else(
                || "https://www.strava.com/oauth/token".to_owned(),
                |c| c.external_services.strava_api.token_url.clone()
            ),
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

/// Strava API provider implementation
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
    /// Creates a new Strava provider with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: api_client(),
            config: StravaConfig::global(),
            access_token: None,
            refresh_token: None,
        }
    }

    /// Creates a new Strava provider with custom configuration
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
    pub fn get_auth_url(&self, redirect_uri: &str, state: &str) -> AppResult<String> {
        if self.config.client_id.is_empty() {
            return Err(ProviderError::ConfigurationError {
                provider: "strava".into(),
                details: "Client ID not configured".into(),
            }
            .into());
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
    ) -> AppResult<String> {
        if self.config.client_id.is_empty() {
            return Err(ProviderError::ConfigurationError {
                provider: "strava".into(),
                details: "Client ID not configured".into(),
            }
            .into());
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
    pub async fn exchange_code(&mut self, code: &str) -> AppResult<(String, String)> {
        if self.config.client_id.is_empty() || self.config.client_secret.is_empty() {
            return Err(ProviderError::ConfigurationError {
                provider: "strava".into(),
                details: "Client credentials not configured".into(),
            }
            .into());
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
            warn!("No refresh token provided by Strava");
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
    ) -> AppResult<(String, String)> {
        if self.config.client_id.is_empty() || self.config.client_secret.is_empty() {
            return Err(ProviderError::ConfigurationError {
                provider: "strava".into(),
                details: "Client credentials not configured".into(),
            }
            .into());
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
            warn!("No refresh token provided by Strava");
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
    pub async fn refresh_access_token(&mut self) -> AppResult<(String, String)> {
        let refresh_token = self.refresh_token.as_ref().ok_or_else(|| {
            ProviderError::TokenRefreshFailed {
                provider: "strava".into(),
                details: "No refresh token available".into(),
            }
        })?;

        if self.config.client_id.is_empty() || self.config.client_secret.is_empty() {
            return Err(ProviderError::ConfigurationError {
                provider: "strava".into(),
                details: "Client credentials not configured".into(),
            }
            .into());
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
            warn!("No refresh token provided by Strava");
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
    async fn authenticate(&mut self, auth_data: AuthData) -> AppResult<()> {
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
            AuthData::ApiKey(_) => Err(ProviderError::AuthenticationFailed {
                provider: "strava".into(),
                reason: "Strava requires OAuth2 authentication".into(),
            }
            .into()),
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
    #[tracing::instrument(skip(self), fields(provider = "strava", api_call = "get_athlete"))]
    async fn get_athlete(&self) -> AppResult<Athlete> {
        let token = self.access_token.as_ref().ok_or_else(|| {
            ProviderError::AuthenticationFailed {
                provider: "strava".into(),
                reason: "Not authenticated".into(),
            }
        })?;

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
                debug!("No username provided by Strava for athlete {}", response.id);
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
    #[tracing::instrument(
        skip(self, params),
        fields(
            provider = "strava",
            api_call = "get_activities",
            limit = ?params.limit,
            offset = ?params.offset,
        )
    )]
    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>> {
        let token = self.access_token.as_ref().ok_or_else(|| {
            ProviderError::AuthenticationFailed {
                provider: "strava".into(),
                reason: "Not authenticated".into(),
            }
        })?;

        // Build query parameters without unnecessary allocations
        let per_page = params
            .limit
            .unwrap_or(api_provider_limits::strava::DEFAULT_ACTIVITIES_PER_PAGE);
        let page = params.offset.map_or(1, |o| o / per_page + 1);

        // Build query with optional before/after timestamps
        let mut query: Vec<(&str, String)> = vec![
            ("per_page", per_page.to_string()),
            ("page", page.to_string()),
        ];

        // Add timestamp filters if provided (Strava native pagination)
        if let Some(before) = params.before {
            query.push(("before", before.to_string()));
        }
        if let Some(after) = params.after {
            query.push(("after", after.to_string()));
        }

        let url = format!("{}/athlete/activities", &self.config.base_url);
        info!("Fetching activities from: {} with query: {:?}", url, query);

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .query(&query)
            .send()
            .await
            .map_err(|e| ProviderError::Reqwest {
                provider: "strava".into(),
                source: e,
            })?;

        let status = response.status();
        info!("Strava API response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|e| {
                warn!("Failed to read error response body: {}", e);
                "Unable to read error response".into()
            });
            error!("Strava API error response: status={}, body_length={} bytes", status, error_text.len());

            // Check if it's an authentication error and we have a refresh token
            if status == 401 && self.refresh_token.is_some() {
                info!("Access token expired, attempting to refresh...");
                // Note: This would require mutable self to refresh the token
                return Err(ProviderError::AuthenticationFailed {
                    provider: "strava".into(),
                    reason: format!("Access token expired. Strava API error: {status} - {error_text}"),
                }
                .into());
            }

            return Err(ProviderError::ApiError {
                provider: "strava".into(),
                status_code: status.as_u16(),
                message: error_text,
                retryable: status.is_server_error(),
            }
            .into());
        }

        // Get response text for parsing
        let response_text = response.text().await.map_err(|e| {
            ProviderError::NetworkError(format!("Failed to read response body: {e}"))
        })?;

        info!("Strava API response length: {} bytes", response_text.len());
        trace!("Strava API response body: [redacted, {} bytes]", response_text.len());

        // Try to parse JSON
        let activities: Vec<StravaActivity> =
            serde_json::from_str(&response_text).map_err(|e| ProviderError::ParseError {
                provider: "strava".into(),
                field: "activities",
                source: e,
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
    ) -> AppResult<CursorPage<Activity>> {
        // Strava API uses numeric pagination - delegate to offset-based approach
        let query_params = ActivityQueryParams::with_pagination(Some(params.limit), None);
        let activities = self.get_activities_with_params(&query_params).await?;
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
    #[tracing::instrument(skip(self), fields(provider = "strava", api_call = "get_activity", activity_id = %id))]
    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let token = self.access_token.as_ref().ok_or_else(|| {
            ProviderError::AuthenticationFailed {
                provider: "strava".into(),
                reason: "Not authenticated".into(),
            }
        })?;

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
    #[tracing::instrument(skip(self), fields(provider = "strava", api_call = "get_stats"))]
    async fn get_stats(&self) -> AppResult<Stats> {
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
    #[tracing::instrument(skip(self), fields(provider = "strava", api_call = "get_personal_records"))]
    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Personal records require analysis of activities to determine bests
        // This would involve fetching activities and calculating personal bests
        // Strava API does not provide direct PR endpoints - requires activity analysis
        debug!("Personal records require activity analysis - returning empty set");
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
    async fn get_strava_athlete_stats(&self) -> AppResult<Stats> {
        let token = self.access_token.as_ref().ok_or_else(|| {
            ProviderError::AuthenticationFailed {
                provider: "strava".into(),
                reason: "Not authenticated".into(),
            }
        })?;

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

    // Location fields
    location_city: Option<String>,
    location_state: Option<String>,
    location_country: Option<String>,

    // Activity classification
    workout_type: Option<u32>,
    sport_type: Option<String>,

    // Performance metrics
    average_cadence: Option<f32>,
    average_watts: Option<f32>,
    weighted_average_watts: Option<f32>,
    max_watts: Option<u32>,
    device_watts: Option<bool>,
    kilojoules: Option<f32>,
    calories: Option<f32>,
    suffer_score: Option<u32>,

    // Segment data
    segment_efforts: Option<Vec<StravaSegmentEffort>>,
}

/// Strava segment effort data
#[derive(Debug, Deserialize)]
struct StravaSegmentEffort {
    id: u64,
    name: String,
    elapsed_time: u64,
    moving_time: Option<u64>,
    start_date: DateTime<Utc>,
    distance: f64,
    average_heartrate: Option<f32>,
    max_heartrate: Option<f32>,
    average_cadence: Option<f32>,
    average_watts: Option<f32>,
    kom_rank: Option<u32>,
    pr_rank: Option<u32>,
    segment: Option<StravaSegment>,
}

/// Strava segment metadata
#[derive(Debug, Deserialize)]
struct StravaSegment {
    climb_category: Option<u32>,
    average_grade: Option<f32>,
    elevation_high: Option<f64>,
    elevation_low: Option<f64>,
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
            calories: strava.calories.and_then(|c| {
                if c.is_finite() && c >= 0.0 {
                    Some(c.round() as u32)
                } else {
                    None
                }
            }),
            steps: None,            // Strava doesn't provide step data
            heart_rate_zones: None, // Strava doesn't provide zone breakdown data

            // Power metrics - now extracted from Strava
            average_power: strava.average_watts.and_then(|w| {
                if w.is_finite() && w >= 0.0 {
                    Some(w.round() as u32)
                } else {
                    None
                }
            }),
            max_power: strava.max_watts,
            normalized_power: strava.weighted_average_watts.and_then(|w| {
                if w.is_finite() && w >= 0.0 {
                    Some(w.round() as u32)
                } else {
                    None
                }
            }),
            power_zones: None,
            ftp: None,

            // Cadence metrics - now extracted from Strava
            average_cadence: strava.average_cadence.and_then(|c| {
                if c.is_finite() && c >= 0.0 {
                    Some(c.round() as u32)
                } else {
                    None
                }
            }),
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
            suffer_score: strava.suffer_score,
            time_series_data: None,

            start_latitude,
            start_longitude,
            city: strava.location_city,
            region: strava.location_state,
            country: strava.location_country,
            trail_name: None,

            // New fields
            workout_type: strava.workout_type,
            sport_type_detail: strava.sport_type,
            segment_efforts: strava.segment_efforts.map(|efforts| {
                efforts.into_iter().map(|effort| {
                    let segment_elevation_gain = effort.segment.as_ref().and_then(|seg| {
                        match (seg.elevation_high, seg.elevation_low) {
                            (Some(high), Some(low)) => Some(high - low),
                            _ => None,
                        }
                    });

                    crate::models::SegmentEffort {
                        id: effort.id.to_string(),
                        name: effort.name,
                        elapsed_time: effort.elapsed_time,
                        moving_time: effort.moving_time,
                        start_date: effort.start_date,
                        distance: effort.distance,
                        average_heart_rate: effort.average_heartrate.and_then(|hr| {
                            if hr.is_finite() && hr >= 0.0 {
                                Some(hr.round() as u32)
                            } else {
                                None
                            }
                        }),
                        max_heart_rate: effort.max_heartrate.and_then(|hr| {
                            if hr.is_finite() && hr >= 0.0 {
                                Some(hr.round() as u32)
                            } else {
                                None
                            }
                        }),
                        average_cadence: effort.average_cadence.and_then(|c| {
                            if c.is_finite() && c >= 0.0 {
                                Some(c.round() as u32)
                            } else {
                                None
                            }
                        }),
                        average_watts: effort.average_watts.and_then(|w| {
                            if w.is_finite() && w >= 0.0 {
                                Some(w.round() as u32)
                            } else {
                                None
                            }
                        }),
                        kom_rank: effort.kom_rank,
                        pr_rank: effort.pr_rank,
                        climb_category: effort.segment.as_ref().and_then(|seg| seg.climb_category),
                        average_grade: effort.segment.as_ref().and_then(|seg| seg.average_grade),
                        elevation_gain: segment_elevation_gain,
                    }
                }).collect()
            }),

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
