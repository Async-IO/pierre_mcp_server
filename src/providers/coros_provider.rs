// ABOUTME: COROS API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 authentication and data fetching for workouts, sleep, and daily summaries
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// NOTE: COROS API documentation is private. Apply for access at:
// https://support.coros.com/hc/en-us/articles/17085887816340-Submitting-an-API-Application
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing across async operations (shared_client().clone())
// - String ownership for API responses and error handling
//
// Clippy allowances for this module:
// - cast_possible_truncation: COROS API returns f64 for metrics that are always within f32 range
// - cast_sign_loss: Heart rate and distance values from COROS are always positive
// - cast_precision_loss: Metric precision loss is acceptable for display purposes
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use super::circuit_breaker::CircuitBreaker;
use super::core::{ActivityQueryParams, FitnessProvider, OAuth2Credentials, ProviderConfig};
use super::errors::ProviderError;
use crate::constants::oauth_providers;
use crate::errors::{AppError, AppResult};
use crate::models::{
    Activity, ActivityBuilder, Athlete, HealthMetrics, PersonalRecord, RecoveryMetrics,
    SleepSession, SleepStage, SleepStageType, SportType, Stats,
};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use crate::utils::http_client::shared_client;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::fmt::Write;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

// ============================================================================
// COROS API Response Structures
// ============================================================================
// NOTE: These structures are based on expected COROS API format.
// Update once official API documentation is received.

/// COROS pagination wrapper for API responses
#[derive(Debug, Deserialize)]
struct CorosPaginatedResponse<T> {
    /// Array of data records
    data: Vec<T>,
    /// Pagination info
    #[serde(default)]
    pagination: Option<CorosPagination>,
}

/// COROS pagination metadata
#[derive(Debug, Deserialize)]
struct CorosPagination {
    /// Token for fetching next page (None if no more pages)
    next_token: Option<String>,
}

/// COROS user profile response
#[derive(Debug, Deserialize)]
struct CorosUserProfile {
    /// User ID
    user_id: String,
    /// User's email address
    email: Option<String>,
    /// User's display name or nickname
    nickname: Option<String>,
    /// User's first name
    first_name: Option<String>,
    /// User's last name
    last_name: Option<String>,
    /// Profile picture URL
    avatar_url: Option<String>,
}

/// COROS workout/activity response
#[derive(Debug, Deserialize)]
struct CorosWorkout {
    /// Unique workout ID
    id: String,
    /// Workout name/title
    name: Option<String>,
    /// Start time of workout (ISO 8601 or Unix timestamp)
    start_time: String,
    /// End time of workout (ISO 8601 or Unix timestamp)
    end_time: Option<String>,
    /// Duration in seconds
    duration: Option<u64>,
    /// Sport/activity type ID
    sport_type: i32,
    /// Distance in meters
    distance: Option<f64>,
    /// Elevation gain in meters
    elevation_gain: Option<f64>,
    /// Calories burned
    calories: Option<u32>,
    /// Average heart rate
    avg_heart_rate: Option<u32>,
    /// Maximum heart rate
    max_heart_rate: Option<u32>,
    /// Average speed in m/s
    avg_speed: Option<f64>,
    /// Average cadence (steps/min for running, rpm for cycling)
    avg_cadence: Option<u32>,
    /// Average power in watts (for cycling/running with power meter)
    avg_power: Option<u32>,
    /// Training load/stress score
    training_load: Option<f32>,
}

/// COROS sleep session response
#[derive(Debug, Deserialize)]
struct CorosSleep {
    /// Unique sleep session ID
    id: String,
    /// Sleep start time (ISO 8601)
    start_time: String,
    /// Sleep end time (ISO 8601)
    end_time: String,
    /// Total sleep duration in minutes
    total_sleep_minutes: Option<u32>,
    /// Time awake during sleep in minutes
    awake_minutes: Option<u32>,
    /// Light sleep duration in minutes
    light_sleep_minutes: Option<u32>,
    /// Deep sleep duration in minutes
    deep_sleep_minutes: Option<u32>,
    /// REM sleep duration in minutes
    rem_sleep_minutes: Option<u32>,
    /// Sleep score (0-100)
    sleep_score: Option<u32>,
    /// Sleep efficiency percentage
    efficiency: Option<f32>,
    /// Respiratory rate during sleep
    respiratory_rate: Option<f32>,
}

/// COROS daily summary response
#[derive(Debug, Deserialize)]
struct CorosDailySummary {
    /// Date of summary (YYYY-MM-DD format)
    date: String,
    /// Resting heart rate
    resting_heart_rate: Option<u32>,
    /// HRV (Heart Rate Variability) RMSSD in ms
    hrv_rmssd: Option<f64>,
    /// Body battery / recovery score
    recovery_score: Option<u32>,
}

// ============================================================================
// COROS Provider Implementation
// ============================================================================

/// COROS fitness provider for GPS sports watch data
///
/// Supports:
/// - Workout/activity data from COROS watches (APEX, PACE, VERTIX series)
/// - Sleep tracking data
/// - Daily health summaries
///
/// Note: COROS OAuth endpoints are placeholders until API documentation is received.
pub struct CorosProvider {
    config: ProviderConfig,
    credentials: RwLock<Option<OAuth2Credentials>>,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl CorosProvider {
    /// Create a new COROS provider with default configuration
    ///
    /// Note: OAuth endpoints are placeholders. Update when API docs are received.
    #[must_use]
    pub fn new() -> Self {
        // Placeholder OAuth endpoints - update when COROS provides API documentation
        let config = ProviderConfig {
            name: oauth_providers::COROS.to_owned(),
            // Placeholder URLs - update with actual COROS OAuth endpoints
            auth_url: "https://open.coros.com/oauth2/authorize".to_owned(),
            token_url: "https://open.coros.com/oauth2/token".to_owned(),
            api_base_url: "https://open.coros.com/api/v1".to_owned(),
            revoke_url: Some("https://open.coros.com/oauth2/revoke".to_owned()),
            default_scopes: oauth_providers::COROS_DEFAULT_SCOPES
                .split(' ')
                .map(str::to_owned)
                .collect(),
        };

        Self {
            circuit_breaker: CircuitBreaker::new(oauth_providers::COROS),
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
            .ok_or_else(|| AppError::internal("No credentials available for COROS API request"))?
            .access_token
            .clone();

        token.ok_or_else(|| AppError::internal("No access token available"))
    }

    /// Make authenticated API request to COROS with circuit breaker protection
    async fn api_request<T>(&self, endpoint: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        debug!("Starting COROS API request to endpoint: {endpoint}");

        // Check circuit breaker before making request
        if !self.circuit_breaker.is_allowed() {
            let err = ProviderError::CircuitBreakerOpen {
                provider: oauth_providers::COROS.to_owned(),
                retry_after_secs: 30,
            };
            return Err(AppError::external_service("COROS", err.to_string()));
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
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| {
                AppError::external_service("COROS", format!("Failed to send request: {e}"))
            })?;

        let status = response.status();
        debug!("COROS API response status: {status}");

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Self::handle_api_error(status, &text));
        }

        response.json().await.map_err(|e| {
            AppError::external_service("COROS", format!("Failed to parse API response: {e}"))
        })
    }

    /// Handle non-success API responses
    fn handle_api_error(status: reqwest::StatusCode, text: &str) -> AppError {
        error!(
            "COROS API request failed - status: {status}, body_length: {} bytes",
            text.len()
        );

        let status_code = status.as_u16();

        // Check for rate limiting
        if status_code == 429 {
            let err = ProviderError::RateLimitExceeded {
                provider: oauth_providers::COROS.to_owned(),
                retry_after_secs: 60,
                limit_type: "API rate limit".to_owned(),
            };
            return AppError::external_service("COROS", err.to_string());
        }

        // Check for auth errors
        if status_code == 401 {
            let err = ProviderError::AuthenticationFailed {
                provider: oauth_providers::COROS.to_owned(),
                reason: "Access token expired or invalid".to_owned(),
            };
            return AppError::external_service("COROS", err.to_string());
        }

        let err = ProviderError::ApiError {
            provider: oauth_providers::COROS.to_owned(),
            status_code,
            message: format!("COROS API request failed with status {status}: {text}"),
            retryable: status_code >= 500,
        };
        AppError::external_service("COROS", err.to_string())
    }

    /// Convert COROS sport type ID to our `SportType` enum
    ///
    /// Note: Sport type mappings are estimated based on common COROS activities.
    /// Update when official API documentation is received.
    fn parse_sport_type(sport_type: i32) -> SportType {
        // Estimated COROS sport type IDs based on common activities
        // Update when official documentation is received
        match sport_type {
            0 => SportType::Workout,                       // Generic/Other
            1 => SportType::Run,                           // Outdoor Run
            2 => SportType::VirtualRun,                    // Indoor Run/Treadmill
            3 => SportType::TrailRunning,                  // Trail Run
            4 => SportType::Ride,                          // Outdoor Cycling
            5 => SportType::VirtualRide,                   // Indoor Cycling
            6 => SportType::MountainBike,                  // Mountain Biking
            7 | 8 => SportType::Swim,                      // Pool Swim / Open Water Swim
            9 => SportType::Other("triathlon".to_owned()), // Triathlon
            10 => SportType::Hike,                         // Hiking
            11 => SportType::Walk,                         // Walking
            12 => SportType::AlpineSkiing,                 // Skiing
            13 => SportType::CrossCountrySkiing,           // Cross-Country Skiing
            14 => SportType::Snowboarding,                 // Snowboarding
            15 => SportType::Rowing,                       // Rowing
            16 => SportType::Kayaking,                     // Kayaking
            17 => SportType::StrengthTraining,             // Strength Training
            18 => SportType::Yoga,                         // Yoga
            19 => SportType::RockClimbing,                 // Climbing
            20 => SportType::Golf,                         // Golf
            _ => SportType::Other(format!("coros_sport_{sport_type}")),
        }
    }

    /// Convert COROS workout to our Activity model
    fn convert_workout(workout: &CorosWorkout) -> AppResult<Activity> {
        let start_date = Self::parse_datetime(&workout.start_time)?;

        let duration_seconds = if let Some(duration) = workout.duration {
            duration
        } else if let Some(end_time) = &workout.end_time {
            let end_date = Self::parse_datetime(end_time)?;
            (end_date - start_date).num_seconds().unsigned_abs()
        } else {
            0
        };

        let sport_type = Self::parse_sport_type(workout.sport_type);
        let name = workout
            .name
            .clone()
            .unwrap_or_else(|| format!("COROS {}", sport_type.display_name()));

        Ok(ActivityBuilder::new(
            workout.id.clone(),
            name,
            sport_type,
            start_date,
            duration_seconds,
            oauth_providers::COROS,
        )
        .distance_meters_opt(workout.distance)
        .elevation_gain_opt(workout.elevation_gain)
        .average_heart_rate_opt(workout.avg_heart_rate)
        .max_heart_rate_opt(workout.max_heart_rate)
        .calories_opt(workout.calories)
        .average_speed_opt(workout.avg_speed)
        .average_cadence_opt(workout.avg_cadence)
        .average_power_opt(workout.avg_power)
        .training_stress_score_opt(workout.training_load)
        .sport_type_detail_opt(Some(format!("coros_sport_{}", workout.sport_type)))
        .build())
    }

    /// Parse datetime from various formats (ISO 8601 or Unix timestamp)
    fn parse_datetime(datetime_str: &str) -> AppResult<DateTime<Utc>> {
        // Try ISO 8601 first
        if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try Unix timestamp (seconds)
        if let Ok(ts) = datetime_str.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                return Ok(dt);
            }
        }

        // Try Unix timestamp (milliseconds)
        if let Ok(ts_ms) = datetime_str.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp(ts_ms / 1000, 0) {
                return Ok(dt);
            }
        }

        Err(AppError::internal(format!(
            "Failed to parse COROS datetime: {datetime_str}"
        )))
    }

    /// Convert COROS sleep data to our `SleepSession` model
    fn convert_sleep(sleep: CorosSleep) -> AppResult<SleepSession> {
        let start_time = Self::parse_datetime(&sleep.start_time)?;
        let end_time = Self::parse_datetime(&sleep.end_time)?;

        let total_sleep_time = sleep.total_sleep_minutes.unwrap_or(0);
        let time_in_bed = total_sleep_time + sleep.awake_minutes.unwrap_or(0);

        let sleep_efficiency = sleep.efficiency.unwrap_or_else(|| {
            if time_in_bed > 0 {
                (f64::from(total_sleep_time) / f64::from(time_in_bed) * 100.0) as f32
            } else {
                0.0
            }
        });

        // Build sleep stages with approximate start times
        // COROS provides durations but not exact stage timestamps, so we use sequential times
        let mut stages = Vec::new();
        let mut current_time = start_time;

        if let Some(awake_mins) = sleep.awake_minutes {
            if awake_mins > 0 {
                stages.push(SleepStage {
                    stage_type: SleepStageType::Awake,
                    start_time: current_time,
                    duration_minutes: awake_mins,
                });
                current_time += chrono::Duration::minutes(i64::from(awake_mins));
            }
        }

        if let Some(light_mins) = sleep.light_sleep_minutes {
            if light_mins > 0 {
                stages.push(SleepStage {
                    stage_type: SleepStageType::Light,
                    start_time: current_time,
                    duration_minutes: light_mins,
                });
                current_time += chrono::Duration::minutes(i64::from(light_mins));
            }
        }

        if let Some(deep_mins) = sleep.deep_sleep_minutes {
            if deep_mins > 0 {
                stages.push(SleepStage {
                    stage_type: SleepStageType::Deep,
                    start_time: current_time,
                    duration_minutes: deep_mins,
                });
                current_time += chrono::Duration::minutes(i64::from(deep_mins));
            }
        }

        if let Some(rem_mins) = sleep.rem_sleep_minutes {
            if rem_mins > 0 {
                stages.push(SleepStage {
                    stage_type: SleepStageType::Rem,
                    start_time: current_time,
                    duration_minutes: rem_mins,
                });
            }
        }

        Ok(SleepSession {
            id: sleep.id,
            start_time,
            end_time,
            time_in_bed,
            total_sleep_time,
            sleep_efficiency,
            sleep_score: sleep.sleep_score.map(|s| s as f32),
            stages,
            hrv_during_sleep: None,
            respiratory_rate: sleep.respiratory_rate,
            temperature_variation: None,
            wake_count: None,
            sleep_onset_latency: None,
            provider: oauth_providers::COROS.to_owned(),
        })
    }

    /// Convert COROS daily summary to recovery metrics
    fn convert_daily_to_recovery(daily: &CorosDailySummary) -> Option<RecoveryMetrics> {
        // Only create recovery metrics if we have meaningful data
        if daily.recovery_score.is_none() && daily.hrv_rmssd.is_none() {
            return None;
        }

        let date = chrono::NaiveDate::parse_from_str(&daily.date, "%Y-%m-%d").ok()?;
        let datetime = date.and_hms_opt(0, 0, 0)?.and_utc();

        Some(RecoveryMetrics {
            date: datetime,
            recovery_score: daily.recovery_score.map(|s| s as f32),
            readiness_score: None,
            hrv_status: daily.hrv_rmssd.map(|h| format!("{h:.1} ms")),
            sleep_score: None,
            stress_level: None,
            training_load: None,
            resting_heart_rate: daily.resting_heart_rate,
            body_temperature: None,
            resting_respiratory_rate: None,
            provider: oauth_providers::COROS.to_owned(),
        })
    }

    /// Convert COROS daily summary to health metrics
    fn convert_daily_to_health(daily: &CorosDailySummary) -> Option<HealthMetrics> {
        let date = chrono::NaiveDate::parse_from_str(&daily.date, "%Y-%m-%d").ok()?;
        let datetime = date.and_hms_opt(0, 0, 0)?.and_utc();

        Some(HealthMetrics {
            date: datetime,
            weight: None,
            body_fat_percentage: None,
            muscle_mass: None,
            bone_mass: None,
            body_water_percentage: None,
            bmr: None,
            blood_pressure: None,
            blood_glucose: None,
            vo2_max: None,
            provider: oauth_providers::COROS.to_owned(),
        })
    }
}

impl Default for CorosProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FitnessProvider for CorosProvider {
    fn name(&self) -> &'static str {
        oauth_providers::COROS
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        info!("Setting COROS credentials");
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
                    provider: oauth_providers::COROS.to_owned(),
                    details: "No credentials available".to_owned(),
                };
                return Err(AppError::external_service("COROS", err.to_string()));
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

        info!("Refreshing COROS access token");

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
                    "COROS",
                    format!("Failed to send token refresh request: {e}"),
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err = ProviderError::AuthenticationFailed {
                provider: oauth_providers::COROS.to_owned(),
                reason: format!("token refresh failed with status: {status}"),
            };
            return Err(AppError::external_service("COROS", err.to_string()));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AppError::external_service(
                "COROS",
                format!("Failed to parse token refresh response: {e}"),
            )
        })?;

        let expires_at = Utc::now() + chrono::Duration::seconds(token_response.expires_in);

        let new_credentials = OAuth2Credentials {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            access_token: Some(token_response.access_token),
            refresh_token: token_response.refresh_token.or(Some(refresh_token)),
            expires_at: Some(expires_at),
            scopes: credentials.scopes,
        };

        *self.credentials.write().await = Some(new_credentials);
        Ok(())
    }

    #[instrument(skip(self), fields(provider = "coros", api_call = "get_athlete"))]
    async fn get_athlete(&self) -> AppResult<Athlete> {
        let profile: CorosUserProfile = self.api_request("user/profile").await?;

        Ok(Athlete {
            id: profile.user_id,
            username: profile
                .nickname
                .or_else(|| profile.email.clone())
                .unwrap_or_default(),
            firstname: profile.first_name,
            lastname: profile.last_name,
            profile_picture: profile.avatar_url,
            provider: oauth_providers::COROS.to_owned(),
        })
    }

    #[instrument(
        skip(self, params),
        fields(
            provider = "coros",
            api_call = "get_activities",
            limit = ?params.limit,
            offset = ?params.offset,
        )
    )]
    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>> {
        let page_limit = params.limit.unwrap_or(25).min(50);

        // Build endpoint with pagination and time filters
        let mut endpoint = format!("workouts?limit={page_limit}");

        if let Some(offset) = params.offset {
            let _ = write!(endpoint, "&offset={offset}");
        }

        if let Some(after) = params.after {
            if let Some(dt) = chrono::DateTime::from_timestamp(after, 0) {
                let _ = write!(endpoint, "&start_date={}", dt.format("%Y-%m-%d"));
            }
        }

        if let Some(before) = params.before {
            if let Some(dt) = chrono::DateTime::from_timestamp(before, 0) {
                let _ = write!(endpoint, "&end_date={}", dt.format("%Y-%m-%d"));
            }
        }

        let response: CorosPaginatedResponse<CorosWorkout> = self.api_request(&endpoint).await?;

        let mut activities = Vec::with_capacity(response.data.len());
        for workout in &response.data {
            match Self::convert_workout(workout) {
                Ok(activity) => activities.push(activity),
                Err(e) => {
                    warn!("Failed to convert COROS workout: {e}");
                }
            }
        }

        Ok(activities)
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        let limit = params.limit.min(50);

        // Build endpoint with cursor-based parameters
        let mut endpoint = format!("workouts?limit={limit}");

        // If cursor provided, use it as the pagination token
        if let Some(cursor) = &params.cursor {
            if let Some((_, token)) = cursor.decode() {
                let _ = write!(endpoint, "&page_token={token}");
            }
        }

        let response: CorosPaginatedResponse<CorosWorkout> = self.api_request(&endpoint).await?;

        let mut activities = Vec::with_capacity(response.data.len());
        for workout in &response.data {
            match Self::convert_workout(workout) {
                Ok(activity) => activities.push(activity),
                Err(e) => {
                    warn!("Failed to convert COROS workout: {e}");
                }
            }
        }

        // Determine if there are more results
        let has_more = response
            .pagination
            .as_ref()
            .and_then(|p| p.next_token.as_ref())
            .is_some();

        // Create next cursor from pagination token
        let next_cursor = response
            .pagination
            .as_ref()
            .and_then(|p| p.next_token.as_ref())
            .map(|token| Cursor::new(Utc::now(), token));

        // Create previous cursor from first activity
        let prev_cursor = if params.cursor.is_some() {
            activities
                .first()
                .map(|first| Cursor::new(first.start_date(), first.id()))
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

    #[instrument(
        skip(self),
        fields(provider = "coros", api_call = "get_activity", activity_id = %id)
    )]
    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let endpoint = format!("workouts/{id}");
        let workout: CorosWorkout = self.api_request(&endpoint).await?;
        Self::convert_workout(&workout)
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        // COROS may not have a direct stats endpoint
        // Return empty stats - can be aggregated from activities if needed
        Ok(Stats {
            total_activities: 0,
            total_distance: 0.0,
            total_duration: 0,
            total_elevation_gain: 0.0,
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // COROS may not expose personal records via API
        Ok(vec![])
    }

    #[instrument(
        skip(self),
        fields(provider = "coros", api_call = "get_sleep_sessions")
    )]
    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        let start_str = start_date.format("%Y-%m-%d");
        let end_str = end_date.format("%Y-%m-%d");

        let endpoint = format!("sleep?start_date={start_str}&end_date={end_str}");

        let response: CorosPaginatedResponse<CorosSleep> = self
            .api_request(&endpoint)
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::COROS.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        let mut sessions = Vec::with_capacity(response.data.len());
        for sleep in response.data {
            match Self::convert_sleep(sleep) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    warn!("Failed to convert COROS sleep session: {e}");
                }
            }
        }

        Ok(sessions)
    }

    #[instrument(
        skip(self),
        fields(provider = "coros", api_call = "get_recovery_metrics")
    )]
    async fn get_recovery_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<RecoveryMetrics>, ProviderError> {
        let start_str = start_date.format("%Y-%m-%d");
        let end_str = end_date.format("%Y-%m-%d");

        let endpoint = format!("daily?start_date={start_str}&end_date={end_str}");

        let response: CorosPaginatedResponse<CorosDailySummary> = self
            .api_request(&endpoint)
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::COROS.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        let metrics: Vec<RecoveryMetrics> = response
            .data
            .iter()
            .filter_map(Self::convert_daily_to_recovery)
            .collect();

        Ok(metrics)
    }

    #[instrument(
        skip(self),
        fields(provider = "coros", api_call = "get_health_metrics")
    )]
    async fn get_health_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<HealthMetrics>, ProviderError> {
        let start_str = start_date.format("%Y-%m-%d");
        let end_str = end_date.format("%Y-%m-%d");

        let endpoint = format!("daily?start_date={start_str}&end_date={end_str}");

        let response: CorosPaginatedResponse<CorosDailySummary> = self
            .api_request(&endpoint)
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::COROS.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        let metrics: Vec<HealthMetrics> = response
            .data
            .iter()
            .filter_map(Self::convert_daily_to_health)
            .collect();

        Ok(metrics)
    }

    async fn disconnect(&self) -> AppResult<()> {
        // Clear stored credentials
        *self.credentials.write().await = None;

        // If revoke URL is available, attempt to revoke the token
        if let Some(revoke_url) = &self.config.revoke_url {
            if let Some(creds) = self.credentials.read().await.as_ref() {
                if let Some(access_token) = &creds.access_token {
                    let params = [("token", access_token.as_str())];

                    let response = self.client.post(revoke_url).form(&params).send().await;

                    if let Err(e) = response {
                        warn!("Failed to revoke COROS token: {e}");
                    }
                }
            }
        }

        info!("COROS provider disconnected");
        Ok(())
    }
}

// ============================================================================
// Provider Factory for Registry Registration
// ============================================================================

use super::core::ProviderFactory;

/// Factory for creating COROS provider instances
pub struct CorosProviderFactory;

impl ProviderFactory for CorosProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(CorosProvider::with_config(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::COROS]
    }
}
