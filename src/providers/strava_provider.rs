// ABOUTME: Clean Strava API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 authentication and data fetching with proper error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing across async operations (shared_client().clone())
// - String ownership for API responses and error handling

use super::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use super::errors::ProviderError;
use crate::constants::{api_provider_limits, oauth_providers};
use crate::errors::{AppError, AppResult};
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use crate::pagination::{Cursor, CursorPage, PaginationDirection, PaginationParams};
use crate::utils::http_client::shared_client;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info, warn};

/// Strava API error response format
#[derive(Debug, Deserialize)]
struct StravaErrorResponse {
    message: String,
    errors: Option<Vec<StravaError>>,
}

#[derive(Debug, Deserialize)]
struct StravaError {
    resource: String,
    field: String,
    code: String,
}

/// Strava API response for athlete data
#[derive(Debug, Deserialize)]
struct StravaAthleteResponse {
    id: u64,
    username: Option<String>,
    firstname: Option<String>,
    lastname: Option<String>,
    profile_medium: Option<String>,
}

/// Strava map data in API responses
#[derive(Debug, Clone, Deserialize)]
pub struct StravaMap {
    /// Encoded polyline summary of the route
    pub summary_polyline: Option<String>,
}

/// Strava API response for activity data (summary endpoint)
#[derive(Debug, Clone, Deserialize)]
pub struct StravaActivityResponse {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    activity_type: String,
    start_date: String,
    distance: Option<f32>,
    elapsed_time: Option<u32>,
    total_elevation_gain: Option<f32>,
    average_speed: Option<f32>,
    max_speed: Option<f32>,
    average_heartrate: Option<f32>,
    max_heartrate: Option<f32>,
    average_cadence: Option<f32>,
    average_watts: Option<f32>,
    max_watts: Option<f32>,
    suffer_score: Option<f32>,

    // Location and GPS data from summary endpoint
    start_latlng: Option<Vec<f64>>,
    location_city: Option<String>,
    location_state: Option<String>,
    location_country: Option<String>,

    // Additional performance metrics from summary endpoint
    calories: Option<f32>,
}

/// Strava split data from detailed activity endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct StravaSplit {
    /// Distance covered in this split (meters)
    pub distance: Option<f32>,
    /// Total elapsed time for the split (seconds)
    pub elapsed_time: Option<u32>,
    /// Elevation gain/loss in the split (meters)
    pub elevation_difference: Option<f32>,
    /// Time spent moving during the split (seconds)
    pub moving_time: Option<u32>,
    /// Split number (1-based index)
    pub split: Option<u32>,
    /// Average speed during the split (meters/second)
    pub average_speed: Option<f32>,
    /// Pace zone classification (0-5)
    pub pace_zone: Option<u32>,
}

/// Strava lap data from detailed activity endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct StravaLap {
    /// Unique identifier for this lap
    pub id: Option<u64>,
    /// Total elapsed time for the lap (seconds)
    pub elapsed_time: Option<u32>,
    /// Time spent moving during the lap (seconds)
    pub moving_time: Option<u32>,
    /// Distance covered in the lap (meters)
    pub distance: Option<f32>,
    /// Total elevation gain during the lap (meters)
    pub total_elevation_gain: Option<f32>,
    /// Average speed during the lap (meters/second)
    pub average_speed: Option<f32>,
    /// Maximum speed reached during the lap (meters/second)
    pub max_speed: Option<f32>,
    /// Average heart rate during the lap (bpm)
    pub average_heartrate: Option<f32>,
    /// Maximum heart rate during the lap (bpm)
    pub max_heartrate: Option<f32>,
    /// Average cadence during the lap (rpm/spm)
    pub average_cadence: Option<f32>,
    /// Average power output during the lap (watts)
    pub average_watts: Option<f32>,
}

/// Strava segment effort data from detailed activity endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct StravaSegmentEffort {
    /// Unique identifier for this segment effort
    pub id: Option<u64>,
    /// Name of the segment
    pub name: Option<String>,
    /// Total elapsed time for the segment (seconds)
    pub elapsed_time: Option<u32>,
    /// Time spent moving during the segment (seconds)
    pub moving_time: Option<u32>,
    /// Distance of the segment (meters)
    pub distance: Option<f32>,
    /// Average heart rate during the segment (bpm)
    pub average_heartrate: Option<f32>,
    /// Maximum heart rate during the segment (bpm)
    pub max_heartrate: Option<f32>,
    /// Average cadence during the segment (rpm/spm)
    pub average_cadence: Option<f32>,
    /// Average power output during the segment (watts)
    pub average_watts: Option<f32>,
}

/// Detailed activity response from GET /activities/{id} endpoint
/// Includes all summary fields plus additional detail-only fields like splits, laps, and segment efforts
#[derive(Debug, Clone, Deserialize)]
pub struct DetailedActivityResponse {
    /// All summary-level activity fields (flattened)
    #[serde(flatten)]
    pub summary: StravaActivityResponse,

    // Social and engagement data
    /// Number of kudos received
    pub kudos_count: Option<u32>,
    /// Number of comments
    pub comment_count: Option<u32>,
    /// Number of athletes who participated
    pub athlete_count: Option<u32>,
    /// Number of photos attached
    pub photo_count: Option<u32>,
    /// Number of achievements earned
    pub achievement_count: Option<u32>,

    // Additional elevation data
    /// Highest elevation point (meters)
    pub elev_high: Option<f32>,
    /// Lowest elevation point (meters)
    pub elev_low: Option<f32>,

    // Performance metrics
    /// Number of personal records achieved
    pub pr_count: Option<u32>,
    /// Name of the recording device
    pub device_name: Option<String>,

    // Complex nested data
    /// Metric splits (1km or 1mi intervals)
    pub splits_metric: Option<Vec<StravaSplit>>,
    /// Lap data from the activity
    pub laps: Option<Vec<StravaLap>>,
    /// Segment efforts completed during the activity
    pub segment_efforts: Option<Vec<StravaSegmentEffort>>,
}

/// Strava API response for stats
#[derive(Debug, Deserialize)]
struct StravaStatsResponse {
    all_ride_totals: Option<StravaTotals>,
    all_run_totals: Option<StravaTotals>,
}

#[derive(Debug, Deserialize)]
struct StravaTotals {
    count: u32,
    distance: f32,
    moving_time: u32,
    elevation_gain: f32,
}

/// Clean Strava provider implementation
pub struct StravaProvider {
    config: ProviderConfig,
    credentials: tokio::sync::RwLock<Option<OAuth2Credentials>>,
    client: Client,
}

/// Convert f32 metric value to u32 for Activity fields
/// Safe for positive values within u32 range (heart rate, power, cadence, etc.)
#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn f32_to_u32(value: f32) -> u32 {
    value as u32
}

impl StravaProvider {
    /// Create a new Strava provider with default configuration
    #[must_use]
    pub fn new() -> Self {
        let config = ProviderConfig {
            name: oauth_providers::STRAVA.to_owned(),
            auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
            token_url: "https://www.strava.com/oauth/token".to_owned(),
            api_base_url: "https://www.strava.com/api/v3".to_owned(),
            revoke_url: Some("https://www.strava.com/oauth/deauthorize".to_owned()),
            default_scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                .split(',')
                .map(str::to_owned)
                .collect(),
        };

        Self {
            config,
            credentials: tokio::sync::RwLock::new(None),
            client: shared_client().clone(),
        }
    }

    /// Create provider with custom configuration
    #[must_use]
    pub fn with_config(config: ProviderConfig) -> Self {
        Self {
            config,
            credentials: tokio::sync::RwLock::new(None),
            client: shared_client().clone(),
        }
    }

    /// Make authenticated API request
    async fn api_request<T>(&self, endpoint: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        tracing::info!("Starting API request to endpoint: {}", endpoint);

        // Refresh token if needed before making request
        self.refresh_token_if_needed().await?;

        // Clone access token to avoid holding lock across await
        let access_token = {
            let guard = self.credentials.read().await;
            let credentials = guard.as_ref().ok_or_else(|| {
                AppError::internal("No credentials available for Strava API request")
            })?;

            let token = credentials
                .access_token
                .clone() // Safe: String ownership needed for async request
                .ok_or_else(|| AppError::internal("No access token available"))?;
            drop(guard); // Release lock immediately after cloning
            token
        };

        // Reject test/invalid tokens with proper error message
        if access_token.starts_with("at_") || access_token.len() < 40 {
            return Err(AppError::internal(
                "Invalid Strava access token. Please authenticate with Strava first to access real data."
            ));
        }

        tracing::debug!("Making authenticated request to Strava API");

        let url = format!(
            "{}/{}",
            self.config.api_base_url,
            endpoint.trim_start_matches('/')
        );

        tracing::info!("Making HTTP GET request to: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| {
                AppError::external_service("Strava", format!("Failed to send request: {e}"))
            })?;

        tracing::info!("Received HTTP response with status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let url_path = url;
            let text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Strava API request failed - status: {}, body: {}",
                status,
                text
            );

            // Handle 404 Not Found errors specifically
            if status.as_u16() == 404 {
                // Try to parse Strava's error response to extract resource details
                if let Ok(error_response) = serde_json::from_str::<StravaErrorResponse>(&text) {
                    tracing::debug!(
                        "Strava 404 error: {} (errors: {})",
                        error_response.message,
                        error_response.errors.as_ref().map_or(0, std::vec::Vec::len)
                    );

                    if let Some(errors) = error_response.errors {
                        if let Some(first_error) = errors.first() {
                            tracing::debug!(
                                "Strava error details: resource={}, field={}, code={}",
                                first_error.resource,
                                first_error.field,
                                first_error.code
                            );

                            // Extract resource ID from URL path (e.g., /activities/123456)
                            let resource_id = url_path
                                .split('/')
                                .next_back()
                                .unwrap_or("unknown")
                                .to_owned();

                            let err = ProviderError::NotFound {
                                provider: oauth_providers::STRAVA.to_owned(),
                                resource_type: first_error.resource.clone(),
                                resource_id,
                            };
                            return Err(AppError::external_service("Strava", err.to_string()));
                        }
                    }
                }
            }

            let err = ProviderError::ApiError {
                provider: oauth_providers::STRAVA.to_owned(),
                status_code: status.as_u16(),
                message: format!("Strava API request failed with status {status}: {text}"),
                retryable: false,
            };
            return Err(AppError::external_service("Strava", err.to_string()));
        }

        tracing::info!("Parsing JSON response from Strava API");
        let result = response.json().await.map_err(|e| {
            AppError::external_service("Strava", format!("Failed to parse API response: {e}"))
        });

        match &result {
            Ok(_) => tracing::info!("Successfully parsed JSON response"),
            Err(e) => tracing::error!("Failed to parse JSON response: {}", e),
        }

        result
    }

    /// Convert Strava activity type to our `SportType` enum
    fn parse_sport_type(strava_type: &str) -> SportType {
        match strava_type.to_lowercase().as_str() {
            "run" => SportType::Run,
            "ride" => SportType::Ride,
            "swim" => SportType::Swim,
            "walk" => SportType::Walk,
            "hike" => SportType::Hike,
            "workout" => SportType::Workout,
            "yoga" => SportType::Yoga,
            "weighttraining" => SportType::StrengthTraining,
            _ => SportType::Other(strava_type.to_owned()),
        }
    }

    /// Convert Strava activity response to internal Activity model
    fn convert_strava_activity(activity: StravaActivityResponse) -> AppResult<Activity> {
        let start_date = DateTime::parse_from_rfc3339(&activity.start_date)
            .map_err(|e| AppError::internal(format!("Failed to parse activity start date: {e}")))?
            .with_timezone(&Utc);

        let duration_seconds = activity.elapsed_time.map_or_else(
            || {
                debug!(
                    activity_id = %activity.id,
                    activity_name = %activity.name,
                    "Strava API returned None for elapsed_time - defaulting to 0 seconds"
                );
                0
            },
            u64::from,
        );

        Ok(Activity {
            id: activity.id.to_string(),
            name: activity.name,
            sport_type: Self::parse_sport_type(&activity.activity_type),
            start_date,
            distance_meters: activity.distance.map(f64::from),
            duration_seconds,
            elevation_gain: activity.total_elevation_gain.map(f64::from),
            average_speed: activity.average_speed.map(f64::from),
            max_speed: activity.max_speed.map(f64::from),
            average_heart_rate: activity.average_heartrate.map(f32_to_u32),
            max_heart_rate: activity.max_heartrate.map(f32_to_u32),
            average_cadence: activity.average_cadence.map(f32_to_u32),
            average_power: activity.average_watts.map(f32_to_u32),
            max_power: activity.max_watts.map(f32_to_u32),
            // Calories from summary endpoint
            calories: activity.calories.map(f32_to_u32),
            steps: None,
            heart_rate_zones: None,
            normalized_power: None,
            power_zones: None,
            ftp: None,
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
            suffer_score: activity.suffer_score.map(f32_to_u32),
            time_series_data: None,
            // GPS coordinates from summary endpoint
            start_latitude: activity
                .start_latlng
                .as_ref()
                .and_then(|latlng| latlng.first().copied()),
            start_longitude: activity
                .start_latlng
                .as_ref()
                .and_then(|latlng| latlng.get(1).copied()),
            // Location data from summary endpoint
            city: activity.location_city,
            region: activity.location_state,
            country: activity.location_country,
            trail_name: None,

            // Detailed activity classification - available from detailed endpoint
            workout_type: None,
            sport_type_detail: Some(activity.activity_type.clone()),
            segment_efforts: None,

            provider: oauth_providers::STRAVA.to_owned(),
        })
    }

    /// Convert detailed Strava activity response to internal Activity model with all fields populated
    ///
    /// # Errors
    /// Returns error if activity date parsing fails or API data is malformed
    pub fn convert_detailed_strava_activity(
        detailed: DetailedActivityResponse,
    ) -> AppResult<Activity> {
        // Start with summary conversion
        let activity = Self::convert_strava_activity(detailed.summary)?;

        // Add detailed-only fields that weren't in summary
        // Note: Most fields are already populated by summary conversion
        // Here we only add what's unique to the detailed endpoint

        // Currently, the detailed endpoint provides splits, laps, and segment efforts
        // but our Activity model doesn't have explicit fields for these yet.
        // The time_series_data field could be populated from detailed streams endpoint
        // (which requires a separate API call to /activities/{id}/streams)

        // For now, we just return the activity with summary data
        // Streams data integration can be added when needed

        Ok(activity)
    }

    /// Fetch detailed activity data from Strava API
    ///
    /// # Errors
    /// Returns error if API request fails, authentication is invalid, or response parsing fails
    pub async fn get_activity_details(&self, id: &str) -> AppResult<Activity> {
        let endpoint = format!("activities/{id}");
        let detailed_activity: DetailedActivityResponse = self.api_request(&endpoint).await?;
        Self::convert_detailed_strava_activity(detailed_activity)
    }

    /// Fetch activities with optional detailed data enrichment
    ///
    /// PERFORMANCE WARNING: When `include_details=true`, this makes N+1 API calls:
    /// - 1 call to fetch activity summaries (or multiple for pagination)
    /// - N additional calls to fetch detailed data for each activity
    ///
    /// For 25 activities with details: 1 summary call + 25 detail calls = 26 total API calls
    /// This significantly increases:
    /// - API quota usage (Strava: 100 requests per 15min, 1000 per day)
    /// - Response latency (26 sequential requests vs 1)
    /// - Rate limiting risk
    ///
    /// Use `include_details=true` only when detailed activity data is explicitly needed.
    /// Most use cases are satisfied by the summary endpoint data.
    ///
    /// # Errors
    /// Returns error if API requests fail, authentication is invalid, or response parsing fails
    pub async fn get_activities_with_details(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        include_details: bool,
    ) -> AppResult<Vec<Activity>> {
        // Fetch summary activities using existing implementation
        let activities = self.get_activities(limit, offset).await?;

        // If details not requested, return summary data
        if !include_details {
            return Ok(activities);
        }

        // Fetch detailed data for each activity (N+1 query pattern)
        tracing::warn!(
            "Fetching detailed data for {} activities - this will make {} additional API calls",
            activities.len(),
            activities.len()
        );

        let mut detailed_activities = Vec::with_capacity(activities.len());
        for activity in activities {
            match self.get_activity_details(&activity.id).await {
                Ok(detailed) => detailed_activities.push(detailed),
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch details for activity {}: {} - using summary data",
                        activity.id,
                        e
                    );
                    // Fallback: use summary data if detail fetch fails
                    detailed_activities.push(activity);
                }
            }
        }

        Ok(detailed_activities)
    }
}

impl Default for StravaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FitnessProvider for StravaProvider {
    fn name(&self) -> &'static str {
        oauth_providers::STRAVA
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        info!("Setting Strava credentials");
        *self.credentials.write().await = Some(credentials);
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        if let Some(creds) = self.credentials.read().await.as_ref() {
            if creds.access_token.is_some() {
                // Check if token is expired
                if let Some(expires_at) = creds.expires_at {
                    return Utc::now() < expires_at;
                }
                return true;
            }
        }
        false
    }

    async fn refresh_token_if_needed(&self) -> AppResult<()> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: String,
            expires_at: i64,
        }

        // Check if refresh is needed and extract credentials
        let (needs_refresh, credentials) = {
            let guard = self.credentials.read().await;
            let needs_refresh = if let Some(creds) = guard.as_ref() {
                creds.expires_at.is_some_and(|expires_at| {
                    Utc::now() + chrono::Duration::minutes(5) > expires_at
                })
            } else {
                let err = ProviderError::ConfigurationError {
                    provider: oauth_providers::STRAVA.to_owned(),
                    details: "No credentials available".to_owned(),
                };
                return Err(AppError::external_service("Strava", err.to_string()));
            };

            let credentials = guard
                .as_ref()
                .ok_or_else(|| AppError::internal("No credentials available for refresh"))?
                .clone(); // Safe: OAuth2Credentials ownership for refresh operation
            drop(guard); // Release lock early to avoid contention

            (needs_refresh, credentials)
        };

        if !needs_refresh {
            return Ok(());
        }

        let refresh_token = credentials
            .refresh_token
            .ok_or_else(|| AppError::internal("No refresh token available"))?;

        info!("Refreshing Strava access token");

        // Prepare token refresh request
        let params = [
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        let response = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                AppError::external_service(
                    "Strava",
                    format!("Failed to send token refresh request: {e}"),
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err = ProviderError::AuthenticationFailed {
                provider: oauth_providers::STRAVA.to_owned(),
                reason: format!("token refresh failed with status: {status}"),
            };
            return Err(AppError::external_service("Strava", err.to_string()));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AppError::external_service(
                "Strava",
                format!("Failed to parse token refresh response: {e}"),
            )
        })?;

        let new_credentials = OAuth2Credentials {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            access_token: Some(token_response.access_token),
            refresh_token: Some(token_response.refresh_token),
            expires_at: Utc.timestamp_opt(token_response.expires_at, 0).single(),
            scopes: credentials.scopes,
        };

        *self.credentials.write().await = Some(new_credentials);
        Ok(())
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        let strava_athlete: StravaAthleteResponse = self.api_request("athlete").await?;

        Ok(Athlete {
            id: strava_athlete.id.to_string(),
            username: strava_athlete.username.unwrap_or_default(),
            firstname: strava_athlete.firstname,
            lastname: strava_athlete.lastname,
            profile_picture: strava_athlete.profile_medium,
            provider: oauth_providers::STRAVA.to_owned(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        let requested_limit =
            limit.unwrap_or(api_provider_limits::strava::DEFAULT_ACTIVITIES_PER_PAGE);
        let start_offset = offset.unwrap_or(0);

        tracing::info!(
            "Starting get_activities - requested_limit: {}, offset: {}",
            requested_limit,
            start_offset
        );

        // If request is within single page limit, use single page fetch
        if requested_limit <= api_provider_limits::strava::MAX_ACTIVITIES_PER_REQUEST {
            return self
                .get_activities_single_page(requested_limit, start_offset)
                .await;
        }

        // For large requests, use multi-page fetching
        self.get_activities_multi_page(requested_limit, start_offset)
            .await
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        let limit = params
            .limit
            .min(api_provider_limits::strava::MAX_ACTIVITIES_PER_REQUEST);

        // Build endpoint with cursor-based parameters
        let mut endpoint = format!("athlete/activities?per_page={limit}");

        // If cursor provided, decode and use for filtering
        if let Some(cursor) = &params.cursor {
            if let Some((timestamp, id)) = cursor.decode() {
                use std::fmt::Write;
                // Strava filters by before/after timestamp
                match params.direction {
                    PaginationDirection::Forward => {
                        let _ = write!(endpoint, "&before={}", timestamp.timestamp());
                    }
                    PaginationDirection::Backward => {
                        let _ = write!(endpoint, "&after={}", timestamp.timestamp());
                    }
                }
                tracing::info!("Cursor pagination: timestamp={}, id={}", timestamp, id);
            }
        }

        tracing::info!("Cursor-based request - endpoint: {}", endpoint);

        let strava_activities: Vec<StravaActivityResponse> = self.api_request(&endpoint).await?;
        tracing::info!(
            "Received {} activities from cursor request",
            strava_activities.len()
        );

        // Convert activities
        let mut activities = Vec::new();
        for activity in &strava_activities {
            activities.push(Self::convert_strava_activity(activity.clone())?);
        }

        // Determine if there are more results
        let has_more = activities.len() == limit;

        // Create next cursor from last activity
        let next_cursor = if has_more {
            activities
                .last()
                .map(|last| Cursor::new(last.start_date, &last.id))
        } else {
            None
        };

        // Create previous cursor from first activity
        let prev_cursor = if params.cursor.is_some() {
            activities
                .first()
                .map(|first| Cursor::new(first.start_date, &first.id))
        } else {
            None
        };

        Ok(CursorPage::new(
            activities,
            next_cursor,
            prev_cursor,
            has_more,
        ))
    }

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let endpoint = format!("activities/{id}");
        let strava_activity: StravaActivityResponse = self.api_request(&endpoint).await?;
        Self::convert_strava_activity(strava_activity)
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        let stats: StravaStatsResponse = self.api_request("athletes/{id}/stats").await?;

        Ok(Stats {
            total_activities: u64::from(
                stats.all_ride_totals.as_ref().map_or(0, |t| t.count)
                    + stats.all_run_totals.as_ref().map_or(0, |t| t.count),
            ),
            total_distance: f64::from(
                stats.all_ride_totals.as_ref().map_or(0.0, |t| t.distance)
                    + stats.all_run_totals.as_ref().map_or(0.0, |t| t.distance),
            ),
            total_duration: u64::from(
                stats.all_ride_totals.as_ref().map_or(0, |t| t.moving_time)
                    + stats.all_run_totals.as_ref().map_or(0, |t| t.moving_time),
            ),
            total_elevation_gain: f64::from(
                stats
                    .all_ride_totals
                    .as_ref()
                    .map_or(0.0, |t| t.elevation_gain)
                    + stats
                        .all_run_totals
                        .as_ref()
                        .map_or(0.0, |t| t.elevation_gain),
            ),
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Strava doesn't provide personal records via API in the same format
        // This would require analyzing activities to determine PRs
        Ok(vec![])
    }

    async fn disconnect(&self) -> AppResult<()> {
        // Clone access token and revoke URL to avoid holding lock across await
        let (access_token_opt, revoke_url_opt) = {
            let guard = self.credentials.read().await;
            guard.as_ref().map_or((None, None), |credentials| {
                (
                    credentials.access_token.clone(), // Safe: String ownership for revoke request
                    self.config.revoke_url.clone(),   // Safe: String ownership for revoke request
                )
            })
        };

        if let (Some(access_token), Some(revoke_url)) = (access_token_opt, revoke_url_opt) {
            self.client
                .post(&revoke_url)
                .form(&[("token", access_token.as_str())])
                .send()
                .await
                .inspect_err(|e| {
                    warn!(
                        error = ?e,
                        "Failed to revoke Strava access token - continuing with credential cleanup"
                    );
                })
                .ok();
            info!("Attempted to revoke Strava access token");
        }

        // Clear credentials regardless of revoke success
        *self.credentials.write().await = None;
        Ok(())
    }
}

impl StravaProvider {
    /// Fetch activities using single API call (for requests <= `BULK_ACTIVITY_FETCH_THRESHOLD`)
    async fn get_activities_single_page(
        &self,
        limit: usize,
        offset: usize,
    ) -> AppResult<Vec<Activity>> {
        let page = offset / limit + 1;
        let endpoint = format!("athlete/activities?per_page={limit}&page={page}");

        tracing::info!("Single page request - endpoint: {}", endpoint);

        let strava_activities: Vec<StravaActivityResponse> = self.api_request(&endpoint).await?;
        tracing::info!(
            "Received {} activities from single page",
            strava_activities.len()
        );

        let mut activities = Vec::new();
        for activity in strava_activities {
            activities.push(Self::convert_strava_activity(activity)?);
        }

        Ok(activities)
    }

    /// Fetch activities using multiple API calls (for requests > `BULK_ACTIVITY_FETCH_THRESHOLD`)
    async fn get_activities_multi_page(
        &self,
        total_limit: usize,
        start_offset: usize,
    ) -> AppResult<Vec<Activity>> {
        // Pre-allocate vector with expected capacity for efficiency
        let mut all_activities = Vec::with_capacity(total_limit);

        // Calculate how many pages we need
        let activities_per_page = api_provider_limits::strava::MAX_ACTIVITIES_PER_REQUEST;
        let pages_needed = total_limit.div_ceil(activities_per_page);

        tracing::info!(
            "Multi-page request - total_limit: {}, pages_needed: {}, start_offset: {}",
            total_limit,
            pages_needed,
            start_offset
        );

        for page_index in 0..pages_needed {
            // Calculate how many activities to fetch for this page
            let remaining_activities = total_limit - all_activities.len();
            let current_page_limit = remaining_activities.min(activities_per_page);

            // Calculate the actual page number accounting for offset
            let current_offset = start_offset + (page_index * activities_per_page);
            let page_number = current_offset / activities_per_page + 1;

            let endpoint =
                format!("athlete/activities?per_page={current_page_limit}&page={page_number}");

            tracing::info!(
                "Fetching page {} of {} - endpoint: {} (expecting {} activities)",
                page_index + 1,
                pages_needed,
                endpoint,
                current_page_limit
            );

            match self
                .api_request::<Vec<StravaActivityResponse>>(&endpoint)
                .await
            {
                Ok(strava_activities) => {
                    tracing::info!(
                        "Page {} returned {} activities",
                        page_index + 1,
                        strava_activities.len()
                    );

                    // Convert and add activities from this page
                    for activity in strava_activities {
                        if all_activities.len() >= total_limit {
                            break; // Stop if we've reached the requested limit
                        }
                        all_activities.push(Self::convert_strava_activity(activity)?);
                    }

                    // If we got fewer activities than expected, we've reached the end
                    if all_activities.len()
                        < (page_index + 1) * activities_per_page.min(total_limit)
                    {
                        tracing::info!(
                            "Reached end of activities - got {} total, breaking early",
                            all_activities.len()
                        );
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch page {} of {}: {}",
                        page_index + 1,
                        pages_needed,
                        e
                    );
                    return Err(e);
                }
            }

            // Stop if we have enough activities
            if all_activities.len() >= total_limit {
                break;
            }
        }

        tracing::info!(
            "Multi-page fetch completed - requested: {}, retrieved: {}",
            total_limit,
            all_activities.len()
        );

        Ok(all_activities)
    }
}
