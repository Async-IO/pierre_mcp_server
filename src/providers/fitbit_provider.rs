// ABOUTME: Clean Fitbit API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 authentication with PKCE and data fetching with proper error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing across async operations (shared_client().clone())
// - String ownership for API responses and error handling

use super::circuit_breaker::CircuitBreaker;
use super::core::{
    ActivityQueryParams, FitnessProvider, OAuth2Credentials, ProviderConfig, ProviderFactory,
};
use super::errors::ProviderError;
use crate::constants::oauth_providers;
use crate::errors::{AppError, AppResult};
use crate::models::{
    Activity, ActivityBuilder, Athlete, HealthMetrics, HeartRateZone, PersonalRecord,
    RecoveryMetrics, SleepSession, SleepStage, SleepStageType, SportType, Stats,
};
use crate::pagination::{CursorPage, PaginationParams};
use crate::utils::http_client::shared_client;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::from_str;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// Fitbit API base URL
const FITBIT_API_BASE: &str = "https://api.fitbit.com/1";

/// Fitbit API error response format
#[derive(Debug, Deserialize)]
struct FitbitErrorResponse {
    errors: Option<Vec<FitbitError>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitError {
    error_type: Option<String>,
    message: Option<String>,
}

/// Fitbit user profile API response wrapper
#[derive(Debug, Deserialize)]
struct FitbitUserResponse {
    user: FitbitUserProfile,
}

/// Fitbit user profile data
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitUserProfile {
    encoded_id: String,
    display_name: String,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    avatar: Option<String>,
}

/// Fitbit activities list API response
#[derive(Debug, Deserialize)]
struct FitbitActivitiesResponse {
    activities: Vec<FitbitActivity>,
}

/// Fitbit activity data from API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitActivity {
    #[serde(rename = "logId")]
    log_id: u64,
    activity_name: String,
    activity_type_id: u32,
    start_time: String,
    original_start_time: Option<String>,
    duration: u64,         // milliseconds
    distance: Option<f64>, // km
    steps: Option<u32>,
    calories: Option<u32>,
    elevation_gain: Option<f64>, // meters
    average_heart_rate: Option<u32>,
    heart_rate_zones: Option<Vec<FitbitHeartRateZone>>,
}

/// Fitbit heart rate zone data
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitHeartRateZone {
    name: String,
    min: u32,
    max: u32,
    minutes: u32,
}

/// Fitbit lifetime stats API response
#[derive(Debug, Deserialize)]
struct FitbitLifetimeStatsResponse {
    lifetime: FitbitLifetime,
}

#[derive(Debug, Deserialize)]
struct FitbitLifetime {
    total: FitbitLifetimeTotal,
}

#[derive(Debug, Deserialize)]
struct FitbitLifetimeTotal {
    distance: f64, // km
    floors: f64,
}

/// Fitbit sleep log API response
#[derive(Debug, Deserialize)]
struct FitbitSleepResponse {
    sleep: Vec<FitbitSleepLog>,
}

/// Fitbit sleep log entry
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitSleepLog {
    log_id: u64,
    start_time: String,
    end_time: String,
    time_in_bed: u32,
    minutes_asleep: u32,
    efficiency: u32,
    levels: Option<FitbitSleepLevels>,
}

/// Fitbit sleep levels data (stages)
#[derive(Debug, Deserialize)]
struct FitbitSleepLevels {
    summary: Option<FitbitSleepSummary>,
}

/// Fitbit sleep stage summary
#[derive(Debug, Deserialize)]
struct FitbitSleepSummary {
    deep: Option<FitbitStageSummary>,
    light: Option<FitbitStageSummary>,
    rem: Option<FitbitStageSummary>,
    wake: Option<FitbitStageSummary>,
}

/// Fitbit individual stage summary
#[derive(Debug, Deserialize)]
struct FitbitStageSummary {
    minutes: u32,
}

/// Fitbit body weight log API response
#[derive(Debug, Deserialize)]
struct FitbitWeightResponse {
    weight: Vec<FitbitWeightLog>,
}

/// Fitbit weight log entry
#[derive(Debug, Deserialize)]
struct FitbitWeightLog {
    date: String,
    weight: f64,      // kg
    fat: Option<f64>, // percentage
}

/// Fitbit heart rate variability (HRV) API response
#[derive(Debug, Deserialize)]
struct FitbitHrvResponse {
    hrv: Vec<FitbitHrvData>,
}

/// Fitbit HRV data entry
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitHrvData {
    date_time: String,
    value: FitbitHrvValue,
}

/// Fitbit HRV value
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitHrvValue {
    daily_rmssd: Option<f64>,
}

/// Fitbit resting heart rate API response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct FitbitRestingHrResponse {
    activities_heart: Option<Vec<FitbitRestingHrData>>,
}

/// Fitbit resting heart rate data
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitRestingHrData {
    date_time: String,
    value: FitbitRestingHrValue,
}

/// Fitbit resting heart rate value
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitRestingHrValue {
    resting_heart_rate: Option<u32>,
}

/// Clean Fitbit provider implementation
pub struct FitbitProvider {
    config: ProviderConfig,
    credentials: RwLock<Option<OAuth2Credentials>>,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl FitbitProvider {
    /// Create a new Fitbit provider with default configuration
    #[must_use]
    pub fn new() -> Self {
        let config = ProviderConfig {
            name: oauth_providers::FITBIT.to_owned(),
            auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
            token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
            api_base_url: FITBIT_API_BASE.to_owned(),
            revoke_url: Some("https://api.fitbit.com/oauth2/revoke".to_owned()),
            default_scopes: oauth_providers::FITBIT_DEFAULT_SCOPES
                .split(' ')
                .map(str::to_owned)
                .collect(),
        };

        Self {
            circuit_breaker: CircuitBreaker::new(oauth_providers::FITBIT),
            config,
            credentials: RwLock::new(None),
            client: shared_client().clone(),
        }
    }

    /// Create provider with custom configuration
    #[must_use]
    pub fn with_config(config: ProviderConfig) -> Self {
        let provider_name = config.name.clone();
        Self {
            circuit_breaker: CircuitBreaker::new(&provider_name),
            config,
            credentials: RwLock::new(None),
            client: shared_client().clone(),
        }
    }

    /// Retrieve the current access token from credentials
    async fn get_access_token(&self) -> AppResult<String> {
        let token = self
            .credentials
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::internal("No credentials available for Fitbit API request"))?
            .access_token
            .clone();

        token.ok_or_else(|| AppError::internal("No access token available"))
    }

    /// Handle non-success API responses
    fn handle_api_error(status: reqwest::StatusCode, text: &str, url: &str) -> AppError {
        error!(
            "Fitbit API request failed - status: {status}, url: {url}, body_length: {} bytes",
            text.len()
        );

        // Try to parse Fitbit error response
        if let Ok(error_response) = from_str::<FitbitErrorResponse>(text) {
            if let Some(errors) = error_response.errors {
                if let Some(first_error) = errors.into_iter().next() {
                    let error_type = first_error.error_type.unwrap_or_default();
                    let message = first_error.message.unwrap_or_default();

                    // Handle specific error types
                    if error_type == "expired_token" {
                        return AppError::external_service(
                            "Fitbit",
                            "Access token expired. Please refresh token.".to_owned(),
                        );
                    }

                    if error_type == "insufficient_scope" {
                        return AppError::external_service(
                            "Fitbit",
                            format!("Insufficient permissions: {message}"),
                        );
                    }

                    return AppError::external_service("Fitbit", message);
                }
            }
        }

        let err = ProviderError::ApiError {
            provider: oauth_providers::FITBIT.to_owned(),
            status_code: status.as_u16(),
            message: format!("Fitbit API request failed with status {status}: {text}"),
            retryable: status.as_u16() >= 500,
        };
        AppError::external_service("Fitbit", err.to_string())
    }

    /// Make authenticated API request with circuit breaker protection
    async fn api_request<T>(&self, endpoint: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        debug!("Starting Fitbit API request to endpoint: {endpoint}");

        // Check circuit breaker before making request
        if !self.circuit_breaker.is_allowed() {
            let err = ProviderError::CircuitBreakerOpen {
                provider: oauth_providers::FITBIT.to_owned(),
                retry_after_secs: 30,
            };
            return Err(AppError::external_service("Fitbit", err.to_string()));
        }

        self.refresh_token_if_needed().await?;

        let access_token = self.get_access_token().await?;

        let url = format!(
            "{}/{}",
            self.config.api_base_url,
            endpoint.trim_start_matches('/')
        );

        let result = self.execute_api_request(&url, &access_token).await;

        // Record success/failure for circuit breaker
        match &result {
            Ok(_) => self.circuit_breaker.record_success(),
            Err(_) => self.circuit_breaker.record_failure(),
        }

        result
    }

    /// Execute the actual API request (separated for circuit breaker wrapping)
    async fn execute_api_request<T>(&self, url: &str, access_token: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self.send_authenticated_request(url, access_token).await?;
        self.parse_response(response, url).await
    }

    /// Send authenticated HTTP request to Fitbit API
    async fn send_authenticated_request(
        &self,
        url: &str,
        access_token: &str,
    ) -> AppResult<reqwest::Response> {
        debug!("Making HTTP GET request to: {url}");

        self.client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| {
                AppError::external_service("Fitbit", format!("Failed to send request: {e}"))
            })
    }

    /// Parse Fitbit API response or handle errors
    async fn parse_response<T>(&self, response: reqwest::Response, url: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        debug!("Received HTTP response with status: {status}");

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Self::handle_api_error(status, &text, url));
        }

        debug!("Parsing JSON response from Fitbit API");
        response.json().await.map_err(|e| {
            error!("Failed to parse JSON response: {e}");
            AppError::external_service("Fitbit", format!("Failed to parse API response: {e}"))
        })
    }

    /// Convert Fitbit activity type ID to our `SportType` enum
    fn parse_sport_type(activity_type_id: u32, activity_name: &str) -> SportType {
        // Fitbit activity type IDs based on their API documentation
        // See: https://dev.fitbit.com/build/reference/web-api/activity/
        match activity_type_id {
            90009 | 90019 | 3001 => SportType::Run, // Run, Running, Treadmill
            90001 => SportType::Walk,               // Walk
            1 | 1071 => SportType::Ride,            // Bike, Cycling
            90024 | 18120 => SportType::Swim,       // Swimming, Walking(water)
            90013 | 17180 => SportType::Hike,       // Hiking
            52001 | 17190 => SportType::Yoga,       // Yoga
            15680 => SportType::StrengthTraining,   // Weight Training
            15000 | 15010 | 15020 => SportType::Workout, // Workout types
            _ => SportType::Other(activity_name.to_owned()),
        }
    }

    /// Convert Fitbit activity to internal Activity model
    fn convert_fitbit_activity(activity: FitbitActivity) -> AppResult<Activity> {
        // Parse start time - Fitbit uses ISO 8601 format
        let start_time_str = activity
            .original_start_time
            .as_ref()
            .unwrap_or(&activity.start_time);

        // Parse start time - try RFC3339 first, then fall back to naive datetime (assume UTC)
        let start_date = DateTime::parse_from_rfc3339(start_time_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                // Try alternative Fitbit format: "2024-01-15T10:30:00.000" (assume UTC)
                chrono::NaiveDateTime::parse_from_str(start_time_str, "%Y-%m-%dT%H:%M:%S%.f")
                    .map(|naive| naive.and_utc())
                    .or_else(|_| {
                        // Try without milliseconds: "2024-01-15T10:30:00"
                        chrono::NaiveDateTime::parse_from_str(start_time_str, "%Y-%m-%dT%H:%M:%S")
                            .map(|naive| naive.and_utc())
                    })
            })
            .map_err(|e| {
                AppError::internal(format!(
                    "Failed to parse activity start time '{start_time_str}': {e}"
                ))
            })?;

        let duration_seconds = activity.duration / 1000; // Convert ms to seconds

        Ok(ActivityBuilder::new(
            activity.log_id.to_string(),
            activity.activity_name.clone(),
            Self::parse_sport_type(activity.activity_type_id, &activity.activity_name),
            start_date,
            duration_seconds,
            oauth_providers::FITBIT,
        )
        .distance_meters_opt(activity.distance.map(|d| d * 1000.0)) // Convert km to meters
        .elevation_gain_opt(activity.elevation_gain)
        .average_speed_opt(activity.distance.and_then(|d| {
            if duration_seconds > 0 {
                #[allow(clippy::cast_precision_loss)]
                Some((d * 1000.0) / (duration_seconds as f64)) // m/s
            } else {
                None
            }
        }))
        .average_heart_rate_opt(activity.average_heart_rate)
        .calories_opt(activity.calories)
        .steps_opt(activity.steps)
        .heart_rate_zones_opt(activity.heart_rate_zones.map(|zones| {
            zones
                .into_iter()
                .map(|zone| HeartRateZone {
                    name: zone.name,
                    min_hr: zone.min,
                    max_hr: zone.max,
                    minutes: zone.minutes,
                })
                .collect()
        }))
        .sport_type_detail(activity.activity_name.clone())
        .build())
    }

    /// Convert Fitbit sleep log to internal `SleepSession` model
    fn convert_fitbit_sleep(sleep: &FitbitSleepLog) -> AppResult<SleepSession> {
        // Parse sleep start time - try RFC3339 first, then fall back to naive datetime (assume UTC)
        let start_time = DateTime::parse_from_rfc3339(&sleep.start_time)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(&sleep.start_time, "%Y-%m-%dT%H:%M:%S%.f")
                    .map(|naive| naive.and_utc())
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(
                            &sleep.start_time,
                            "%Y-%m-%dT%H:%M:%S",
                        )
                        .map(|naive| naive.and_utc())
                    })
            })
            .map_err(|e| AppError::internal(format!("Failed to parse sleep start time: {e}")))?;

        // Parse sleep end time - try RFC3339 first, then fall back to naive datetime (assume UTC)
        let end_time = DateTime::parse_from_rfc3339(&sleep.end_time)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(&sleep.end_time, "%Y-%m-%dT%H:%M:%S%.f")
                    .map(|naive| naive.and_utc())
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(&sleep.end_time, "%Y-%m-%dT%H:%M:%S")
                            .map(|naive| naive.and_utc())
                    })
            })
            .map_err(|e| AppError::internal(format!("Failed to parse sleep end time: {e}")))?;

        // Build sleep stages from Fitbit levels data
        // Fitbit provides aggregate summaries per stage, not individual intervals
        // We use session start_time as the stage start_time since these are summary records
        let mut stages: Vec<SleepStage> = Vec::new();

        if let Some(levels) = &sleep.levels {
            if let Some(summary) = &levels.summary {
                // Add stage summaries - using session start_time for all stages
                // since Fitbit summary API returns aggregated durations not individual intervals
                if let Some(deep) = &summary.deep {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Deep,
                        duration_minutes: deep.minutes,
                        start_time,
                    });
                }
                if let Some(light) = &summary.light {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Light,
                        duration_minutes: light.minutes,
                        start_time,
                    });
                }
                if let Some(rem) = &summary.rem {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Rem,
                        duration_minutes: rem.minutes,
                        start_time,
                    });
                }
                if let Some(wake) = &summary.wake {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Awake,
                        duration_minutes: wake.minutes,
                        start_time,
                    });
                }
            }
        }

        // Calculate sleep efficiency
        #[allow(clippy::cast_precision_loss)]
        let sleep_efficiency = if sleep.time_in_bed > 0 {
            (sleep.minutes_asleep as f32 / sleep.time_in_bed as f32) * 100.0
        } else {
            sleep.efficiency as f32
        };

        Ok(SleepSession {
            id: sleep.log_id.to_string(),
            start_time,
            end_time,
            time_in_bed: sleep.time_in_bed,
            total_sleep_time: sleep.minutes_asleep,
            sleep_efficiency,
            sleep_score: None, // Fitbit sleep score requires separate API call
            stages,
            hrv_during_sleep: None, // Would need separate HRV API call
            respiratory_rate: None,
            temperature_variation: None,
            wake_count: None,
            sleep_onset_latency: None,
            provider: oauth_providers::FITBIT.to_owned(),
        })
    }
}

impl Default for FitbitProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FitnessProvider for FitbitProvider {
    fn name(&self) -> &'static str {
        oauth_providers::FITBIT
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        info!("Setting Fitbit credentials");
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
            refresh_token: Option<String>,
            expires_in: i64,
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
                    provider: oauth_providers::FITBIT.to_owned(),
                    details: "No credentials available".to_owned(),
                };
                return Err(AppError::external_service("Fitbit", err.to_string()));
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

        info!("Refreshing Fitbit access token");

        // Fitbit requires Basic auth for token refresh
        let auth_value = Engine::encode(
            &BASE64_STANDARD,
            format!("{}:{}", credentials.client_id, credentials.client_secret),
        );

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        let response = self
            .client
            .post(&self.config.token_url)
            .header("Authorization", format!("Basic {auth_value}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                AppError::external_service(
                    "Fitbit",
                    format!("Failed to send token refresh request: {e}"),
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err = ProviderError::AuthenticationFailed {
                provider: oauth_providers::FITBIT.to_owned(),
                reason: format!("token refresh failed with status: {status}"),
            };
            return Err(AppError::external_service("Fitbit", err.to_string()));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AppError::external_service(
                "Fitbit",
                format!("Failed to parse token refresh response: {e}"),
            )
        })?;

        let new_credentials = OAuth2Credentials {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            access_token: Some(token_response.access_token),
            refresh_token: token_response.refresh_token.or(Some(refresh_token)),
            expires_at: Some(Utc::now() + chrono::Duration::seconds(token_response.expires_in)),
            scopes: credentials.scopes,
        };

        *self.credentials.write().await = Some(new_credentials);
        Ok(())
    }

    #[instrument(skip(self), fields(provider = "fitbit", api_call = "get_athlete"))]
    async fn get_athlete(&self) -> AppResult<Athlete> {
        let response: FitbitUserResponse = self.api_request("user/-/profile.json").await?;

        Ok(Athlete {
            id: response.user.encoded_id,
            username: response.user.display_name.clone(),
            firstname: response.user.first_name,
            lastname: response.user.last_name,
            profile_picture: response.user.avatar,
            provider: oauth_providers::FITBIT.to_owned(),
        })
    }

    #[instrument(
        skip(self, params),
        fields(
            provider = "fitbit",
            api_call = "get_activities",
            limit = ?params.limit,
            offset = ?params.offset,
        )
    )]
    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>> {
        // Fitbit API uses date-based pagination with beforeDate/afterDate
        let fitbit_offset = params.offset.unwrap_or(0);

        // Use before/after timestamps if provided, otherwise default to last 30 days
        let (start_date, end_date) = if params.before.is_some() || params.after.is_some() {
            let end = params.before.map_or_else(
                || chrono::Utc::now().date_naive(),
                |ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map_or_else(|| chrono::Utc::now().date_naive(), |dt| dt.date_naive())
                },
            );
            let start = params.after.map_or_else(
                || end - chrono::Duration::days(365),
                |ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map_or_else(|| end - chrono::Duration::days(365), |dt| dt.date_naive())
                },
            );
            (start, end)
        } else {
            let end = chrono::Utc::now().date_naive();
            (end - chrono::Duration::days(30), end)
        };

        let endpoint = format!(
            "user/-/activities/list.json?beforeDate={}&afterDate={}&sort=desc&limit={}&offset={}",
            end_date.format("%Y-%m-%d"),
            start_date.format("%Y-%m-%d"),
            params.limit.unwrap_or(100),
            fitbit_offset
        );

        let response: FitbitActivitiesResponse = self.api_request(&endpoint).await?;

        let mut activities = Vec::with_capacity(response.activities.len());
        for fitbit_activity in response.activities {
            match Self::convert_fitbit_activity(fitbit_activity) {
                Ok(activity) => activities.push(activity),
                Err(e) => {
                    warn!("Failed to convert Fitbit activity: {e}");
                }
            }
        }

        // Apply limit if specified
        if let Some(limit) = params.limit {
            activities.truncate(limit);
        }

        Ok(activities)
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        // Fitbit API uses date-based pagination - delegate to offset-based approach
        let activities = self.get_activities(Some(params.limit), None).await?;
        let has_more = activities.len() == params.limit;
        Ok(CursorPage::new(activities, None, None, has_more))
    }

    #[instrument(
        skip(self),
        fields(provider = "fitbit", api_call = "get_activity", activity_id = %id)
    )]
    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        // Fitbit doesn't have a direct single activity endpoint
        // We need to use the activity log endpoint
        let endpoint = format!("user/-/activities/{id}.json");
        let response: FitbitActivitiesResponse = self.api_request(&endpoint).await?;

        response
            .activities
            .into_iter()
            .next()
            .ok_or_else(|| AppError::not_found(format!("Activity {id} not found")))
            .and_then(Self::convert_fitbit_activity)
    }

    #[instrument(skip(self), fields(provider = "fitbit", api_call = "get_stats"))]
    async fn get_stats(&self) -> AppResult<Stats> {
        let response: FitbitLifetimeStatsResponse =
            self.api_request("user/-/activities.json").await?;

        // Fitbit provides lifetime totals
        Ok(Stats {
            total_activities: 0, // Fitbit doesn't provide activity count in lifetime stats
            total_distance: response.lifetime.total.distance * 1000.0, // Convert km to meters
            total_duration: 0,   // Not available in lifetime stats
            total_elevation_gain: response.lifetime.total.floors * 3.0, // Estimate: 1 floor ≈ 3m
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Fitbit doesn't have a direct personal records API
        // This would need to be calculated from activity history
        Ok(vec![])
    }

    #[instrument(
        skip(self),
        fields(provider = "fitbit", api_call = "get_sleep_sessions")
    )]
    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        let endpoint = format!(
            "user/-/sleep/list.json?beforeDate={}&afterDate={}&sort=desc&limit=100&offset=0",
            end_date.format("%Y-%m-%d"),
            start_date.format("%Y-%m-%d")
        );

        let response: FitbitSleepResponse =
            self.api_request(&endpoint)
                .await
                .map_err(|e| ProviderError::ApiError {
                    provider: oauth_providers::FITBIT.to_owned(),
                    status_code: 0,
                    message: e.to_string(),
                    retryable: false,
                })?;

        let mut sessions = Vec::with_capacity(response.sleep.len());
        for sleep in &response.sleep {
            match Self::convert_fitbit_sleep(sleep) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    warn!("Failed to convert Fitbit sleep session: {e}");
                }
            }
        }

        Ok(sessions)
    }

    #[instrument(
        skip(self),
        fields(provider = "fitbit", api_call = "get_latest_sleep_session")
    )]
    async fn get_latest_sleep_session(&self) -> Result<SleepSession, ProviderError> {
        let today = Utc::now();
        let week_ago = today - chrono::Duration::days(7);

        let sessions = self.get_sleep_sessions(week_ago, today).await?;

        sessions
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::NotFound {
                provider: oauth_providers::FITBIT.to_owned(),
                resource_type: "SleepSession".to_owned(),
                resource_id: "latest".to_owned(),
            })
    }

    #[instrument(
        skip(self),
        fields(provider = "fitbit", api_call = "get_recovery_metrics")
    )]
    async fn get_recovery_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<RecoveryMetrics>, ProviderError> {
        // Fitbit recovery metrics require multiple API calls to aggregate
        // We combine HRV, resting heart rate, and sleep data

        let mut metrics: Vec<RecoveryMetrics> = Vec::new();

        // Get HRV data (requires Fitbit Premium in some cases)
        let hrv_endpoint = format!(
            "user/-/hrv/date/{}/{}.json",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );

        let hrv_response: Result<FitbitHrvResponse, _> = self.api_request(&hrv_endpoint).await;

        // Get resting heart rate data
        let rhr_endpoint = format!(
            "user/-/activities/heart/date/{}/{}.json",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );

        let rhr_response: Result<FitbitRestingHrResponse, _> =
            self.api_request(&rhr_endpoint).await;

        // Build recovery metrics from available data
        // For each day in range, create a RecoveryMetrics entry
        let days = (end_date - start_date).num_days();

        for day_offset in 0..=days {
            let date = start_date + chrono::Duration::days(day_offset);
            let date_str = date.format("%Y-%m-%d").to_string();

            let mut recovery = RecoveryMetrics {
                date,
                recovery_score: None,
                readiness_score: None,
                hrv_status: None,
                sleep_score: None,
                stress_level: None,
                training_load: None,
                resting_heart_rate: None,
                body_temperature: None,
                resting_respiratory_rate: None,
                provider: oauth_providers::FITBIT.to_owned(),
            };

            // Add HRV data if available
            if let Ok(ref hrv) = hrv_response {
                if let Some(hrv_data) = hrv.hrv.iter().find(|h| h.date_time == date_str) {
                    if let Some(rmssd) = hrv_data.value.daily_rmssd {
                        // Convert HRV RMSSD to a status indicator
                        recovery.hrv_status = Some(if rmssd > 50.0 {
                            "High".to_owned()
                        } else if rmssd > 30.0 {
                            "Normal".to_owned()
                        } else {
                            "Low".to_owned()
                        });

                        // Estimate recovery score based on HRV (simplified model)
                        #[allow(clippy::cast_possible_truncation)]
                        let hrv_score = ((rmssd / 100.0) * 100.0).min(100.0) as f32;
                        recovery.recovery_score = Some(hrv_score);
                    }
                }
            }

            // Add resting heart rate if available
            if let Ok(ref rhr) = rhr_response {
                if let Some(activities_heart) = &rhr.activities_heart {
                    if let Some(rhr_data) =
                        activities_heart.iter().find(|h| h.date_time == date_str)
                    {
                        recovery.resting_heart_rate = rhr_data.value.resting_heart_rate;
                    }
                }
            }

            // Only add metrics if we have some data
            if recovery.recovery_score.is_some()
                || recovery.resting_heart_rate.is_some()
                || recovery.hrv_status.is_some()
            {
                metrics.push(recovery);
            }
        }

        Ok(metrics)
    }

    #[instrument(
        skip(self),
        fields(provider = "fitbit", api_call = "get_health_metrics")
    )]
    async fn get_health_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<HealthMetrics>, ProviderError> {
        // Get body weight/composition data
        let weight_endpoint = format!(
            "user/-/body/log/weight/date/{}/{}.json",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );

        let weight_response: Result<FitbitWeightResponse, _> =
            self.api_request(&weight_endpoint).await;

        let mut metrics: Vec<HealthMetrics> = Vec::new();

        if let Ok(response) = weight_response {
            for weight_log in response.weight {
                // Parse date - use midnight UTC for the given date
                let date = chrono::NaiveDate::parse_from_str(&weight_log.date, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map_or_else(Utc::now, |dt| Utc.from_utc_datetime(&dt));

                metrics.push(HealthMetrics {
                    date,
                    weight: Some(weight_log.weight),
                    #[allow(clippy::cast_possible_truncation)]
                    body_fat_percentage: weight_log.fat.map(|f| f as f32),
                    muscle_mass: None, // Not available from Fitbit basic API
                    bone_mass: None,
                    body_water_percentage: None,
                    bmr: None,
                    blood_pressure: None,
                    blood_glucose: None,
                    vo2_max: None,
                    provider: oauth_providers::FITBIT.to_owned(),
                });
            }
        }

        Ok(metrics)
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
            // Fitbit uses POST with token in body
            self.client
                .post(&revoke_url)
                .form(&[("token", access_token.as_str())])
                .send()
                .await
                .inspect_err(|e| {
                    warn!(
                        error = ?e,
                        "Failed to revoke Fitbit access token - continuing with credential cleanup"
                    );
                })
                .ok();
            info!("Attempted to revoke Fitbit access token");
        }

        // Clear credentials regardless of revoke success
        *self.credentials.write().await = None;
        Ok(())
    }
}

// ============================================================================
// Provider Factory
// ============================================================================

/// Factory for creating Fitbit provider instances
pub struct FitbitProviderFactory;

impl ProviderFactory for FitbitProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(FitbitProvider::with_config(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::FITBIT]
    }
}
