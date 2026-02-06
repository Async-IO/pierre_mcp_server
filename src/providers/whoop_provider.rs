// ABOUTME: WHOOP API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 authentication and data fetching for sleep, recovery, workouts
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing across async operations (shared_client().clone())
// - String ownership for API responses and error handling
//
// Clippy allowances for this module:
// - cast_possible_truncation: WHOOP API returns f64 for scores that are always within f32 range
// - cast_sign_loss: Heart rate and score values from WHOOP are always positive
// - cast_precision_loss: Score precision loss is acceptable (0-100 range)
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
// WHOOP API Response Structures
// ============================================================================

/// WHOOP pagination wrapper for API responses
#[derive(Debug, Deserialize)]
struct WhoopPaginatedResponse<T> {
    /// Array of records
    records: Vec<T>,
    /// Token for fetching next page (None if no more pages)
    next_token: Option<String>,
}

/// WHOOP user profile response
#[derive(Debug, Deserialize)]
struct WhoopUserProfile {
    /// User ID (integer in WHOOP)
    user_id: i64,
    /// User's email address
    email: Option<String>,
    /// User's first name
    first_name: Option<String>,
    /// User's last name
    last_name: Option<String>,
}

/// WHOOP body measurement response
#[derive(Debug, Deserialize)]
struct WhoopBodyMeasurement {
    /// Weight in kilograms
    weight_kilogram: Option<f64>,
}

/// WHOOP workout/activity response
#[derive(Debug, Deserialize)]
struct WhoopWorkout {
    /// Unique workout ID (UUID string in v2)
    id: String,
    /// Start time of workout (ISO 8601)
    start: String,
    /// End time of workout (ISO 8601)
    end: String,
    /// Sport ID (WHOOP internal sport classification)
    sport_id: i32,
    /// Workout score details
    score: Option<WhoopWorkoutScore>,
}

/// WHOOP workout score details
#[derive(Debug, Deserialize)]
struct WhoopWorkoutScore {
    /// Strain score (0-21 scale)
    strain: Option<f64>,
    /// Average heart rate during workout
    average_heart_rate: Option<i32>,
    /// Maximum heart rate during workout
    max_heart_rate: Option<i32>,
    /// Kilojoules burned
    kilojoule: Option<f64>,
    /// Distance in meters (for applicable activities)
    distance_meter: Option<f64>,
    /// Altitude gain in meters
    altitude_gain_meter: Option<f64>,
}

/// WHOOP sleep activity response
#[derive(Debug, Deserialize)]
struct WhoopSleep {
    /// Unique sleep ID (UUID string in v2)
    id: String,
    /// Start time of sleep (ISO 8601)
    start: String,
    /// End time of sleep (ISO 8601)
    end: String,
    /// Sleep score details
    score: Option<WhoopSleepScore>,
}

/// WHOOP sleep score details
#[derive(Debug, Deserialize)]
struct WhoopSleepScore {
    /// Stage summary breakdown
    stage_summary: Option<WhoopStageSummary>,
    /// Respiratory rate during sleep
    respiratory_rate: Option<f64>,
    /// Sleep performance percentage (0-100)
    sleep_performance_percentage: Option<f64>,
    /// Sleep efficiency percentage (0-100)
    sleep_efficiency_percentage: Option<f64>,
}

/// WHOOP sleep stage summary
#[derive(Debug, Deserialize)]
struct WhoopStageSummary {
    /// Total time in bed in milliseconds
    total_in_bed_time_milli: Option<i64>,
    /// Total awake time in milliseconds
    total_awake_time_milli: Option<i64>,
    /// Total light sleep time in milliseconds
    total_light_sleep_time_milli: Option<i64>,
    /// Total slow wave (deep) sleep time in milliseconds
    total_slow_wave_sleep_time_milli: Option<i64>,
    /// Total REM sleep time in milliseconds
    total_rem_sleep_time_milli: Option<i64>,
    /// Number of disturbances
    disturbance_count: Option<i32>,
}

/// WHOOP cycle (daily physiological cycle) response
#[derive(Debug, Deserialize)]
struct WhoopCycle {
    /// Start time of cycle (ISO 8601)
    start: String,
    /// Cycle score details (strain and recovery)
    score: Option<WhoopCycleScore>,
}

/// WHOOP cycle score containing strain and recovery data
#[derive(Debug, Deserialize)]
struct WhoopCycleScore {
    /// Strain score for the cycle (0-21)
    strain: Option<f64>,
    /// Recovery score details
    recovery: Option<WhoopRecoveryScore>,
}

/// WHOOP recovery score details
#[derive(Debug, Deserialize)]
struct WhoopRecoveryScore {
    /// Recovery score as percentage (0-100)
    recovery_score: Option<f64>,
    /// Resting heart rate
    resting_heart_rate: Option<f64>,
    /// Heart rate variability (RMSSD)
    hrv_rmssd_milli: Option<f64>,
    /// Skin temperature in Celsius
    skin_temp_celsius: Option<f64>,
}

// ============================================================================
// WHOOP Provider Implementation
// ============================================================================

/// WHOOP fitness provider for sleep, recovery, and workout data
pub struct WhoopProvider {
    config: ProviderConfig,
    credentials: RwLock<Option<OAuth2Credentials>>,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl WhoopProvider {
    /// Create a new WHOOP provider with default configuration
    #[must_use]
    pub fn new() -> Self {
        let config = ProviderConfig {
            name: oauth_providers::WHOOP.to_owned(),
            auth_url: "https://api.prod.whoop.com/oauth/oauth2/auth".to_owned(),
            token_url: "https://api.prod.whoop.com/oauth/oauth2/token".to_owned(),
            api_base_url: "https://api.prod.whoop.com/developer/v1".to_owned(),
            revoke_url: Some("https://api.prod.whoop.com/oauth/oauth2/revoke".to_owned()),
            default_scopes: oauth_providers::WHOOP_DEFAULT_SCOPES
                .split(' ')
                .map(str::to_owned)
                .collect(),
        };

        Self {
            circuit_breaker: CircuitBreaker::new(oauth_providers::WHOOP),
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
            .ok_or_else(|| AppError::internal("No credentials available for WHOOP API request"))?
            .access_token
            .clone();

        token.ok_or_else(|| AppError::internal("No access token available"))
    }

    /// Make authenticated API request to WHOOP with circuit breaker protection
    async fn api_request<T>(&self, endpoint: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        debug!("Starting WHOOP API request to endpoint: {endpoint}");

        // Check circuit breaker before making request
        if !self.circuit_breaker.is_allowed() {
            let err = ProviderError::CircuitBreakerOpen {
                provider: oauth_providers::WHOOP.to_owned(),
                retry_after_secs: 30,
            };
            return Err(AppError::external_service("WHOOP", err.to_string()));
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
                AppError::external_service("WHOOP", format!("Failed to send request: {e}"))
            })?;

        let status = response.status();
        debug!("WHOOP API response status: {status}");

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Self::handle_api_error(status, &text));
        }

        response.json().await.map_err(|e| {
            AppError::external_service("WHOOP", format!("Failed to parse API response: {e}"))
        })
    }

    /// Handle non-success API responses
    fn handle_api_error(status: reqwest::StatusCode, text: &str) -> AppError {
        error!(
            "WHOOP API request failed - status: {status}, body_length: {} bytes",
            text.len()
        );

        let status_code = status.as_u16();

        // Check for rate limiting
        if status_code == 429 {
            let err = ProviderError::RateLimitExceeded {
                provider: oauth_providers::WHOOP.to_owned(),
                retry_after_secs: 60, // Default to 60 seconds
                limit_type: "API rate limit".to_owned(),
            };
            return AppError::external_service("WHOOP", err.to_string());
        }

        // Check for auth errors
        if status_code == 401 {
            let err = ProviderError::AuthenticationFailed {
                provider: oauth_providers::WHOOP.to_owned(),
                reason: "Access token expired or invalid".to_owned(),
            };
            return AppError::external_service("WHOOP", err.to_string());
        }

        let err = ProviderError::ApiError {
            provider: oauth_providers::WHOOP.to_owned(),
            status_code,
            message: format!("WHOOP API request failed with status {status}: {text}"),
            retryable: status_code >= 500,
        };
        AppError::external_service("WHOOP", err.to_string())
    }

    /// Convert WHOOP sport ID to our `SportType` enum
    fn parse_sport_type(sport_id: i32) -> SportType {
        // WHOOP sport IDs (from their API documentation)
        match sport_id {
            0 => SportType::Workout,             // Generic activity
            1 | 33 => SportType::Run,            // Running / Outdoor run
            34 => SportType::VirtualRun,         // Indoor run/Treadmill
            16 => SportType::Ride,               // Cycling
            17 => SportType::VirtualRide,        // Indoor cycling/Spin
            18 => SportType::MountainBike,       // Mountain biking
            43 | 44 => SportType::Swim,          // Swimming / Open water swim
            48 => SportType::Rowing,             // Rowing
            63 => SportType::Yoga,               // Yoga
            64 => SportType::Pilates,            // Pilates
            71 => SportType::StrengthTraining,   // Weightlifting
            47 => SportType::CrossCountrySkiing, // Cross-country skiing
            46 => SportType::AlpineSkiing,       // Alpine skiing
            45 => SportType::Snowboarding,       // Snowboarding
            52 => SportType::Hike,               // Hiking
            50 => SportType::Walk,               // Walking
            82 => SportType::Golf,               // Golf
            83 => SportType::Tennis,             // Tennis
            84 => SportType::Basketball,         // Basketball
            85 => SportType::Soccer,             // Soccer
            54 => SportType::RockClimbing,       // Climbing
            _ => SportType::Other(format!("whoop_sport_{sport_id}")),
        }
    }

    /// Convert WHOOP workout to our Activity model
    fn convert_workout(workout: &WhoopWorkout) -> AppResult<Activity> {
        let start_date = DateTime::parse_from_rfc3339(&workout.start)
            .map_err(|e| AppError::internal(format!("Failed to parse workout start date: {e}")))?
            .with_timezone(&Utc);

        let end_date = DateTime::parse_from_rfc3339(&workout.end)
            .map_err(|e| AppError::internal(format!("Failed to parse workout end date: {e}")))?
            .with_timezone(&Utc);

        let duration_seconds = (end_date - start_date).num_seconds().unsigned_abs();

        let score = workout.score.as_ref();

        Ok(ActivityBuilder::new(
            workout.id.clone(),
            format!(
                "WHOOP {}",
                Self::parse_sport_type(workout.sport_id).display_name()
            ),
            Self::parse_sport_type(workout.sport_id),
            start_date,
            duration_seconds,
            oauth_providers::WHOOP,
        )
        .distance_meters_opt(score.and_then(|s| s.distance_meter))
        .elevation_gain_opt(score.and_then(|s| s.altitude_gain_meter))
        .average_heart_rate_opt(score.and_then(|s| s.average_heart_rate).map(|hr| hr as u32))
        .max_heart_rate_opt(score.and_then(|s| s.max_heart_rate).map(|hr| hr as u32))
        .calories_opt(
            score
                .and_then(|s| s.kilojoule)
                .map(|kj| (kj * 0.239) as u32),
        )
        .training_stress_score_opt(score.and_then(|s| s.strain).map(|s| s as f32))
        .sport_type_detail_opt(Some(format!("whoop_sport_{}", workout.sport_id)))
        .build())
    }

    /// Convert WHOOP sleep to our `SleepSession` model
    fn convert_sleep(sleep: WhoopSleep) -> AppResult<SleepSession> {
        let start_time = DateTime::parse_from_rfc3339(&sleep.start)
            .map_err(|e| AppError::internal(format!("Failed to parse sleep start: {e}")))?
            .with_timezone(&Utc);

        let end_time = DateTime::parse_from_rfc3339(&sleep.end)
            .map_err(|e| AppError::internal(format!("Failed to parse sleep end: {e}")))?
            .with_timezone(&Utc);

        let score = sleep.score.as_ref();
        let stage_summary = score.and_then(|s| s.stage_summary.as_ref());

        // Calculate times from stage summary
        let time_in_bed = stage_summary
            .and_then(|s| s.total_in_bed_time_milli)
            .map_or(0, |ms| (ms / 60_000) as u32);

        let awake_time = stage_summary
            .and_then(|s| s.total_awake_time_milli)
            .map_or(0, |ms| (ms / 60_000) as u32);

        let total_sleep_time = time_in_bed.saturating_sub(awake_time);

        let sleep_efficiency = score
            .and_then(|s| s.sleep_efficiency_percentage)
            .map_or_else(
                || {
                    if time_in_bed > 0 {
                        (f64::from(total_sleep_time) / f64::from(time_in_bed) * 100.0) as f32
                    } else {
                        0.0
                    }
                },
                |p| p as f32,
            );

        // Build sleep stages
        let mut stages = Vec::new();

        if let Some(summary) = stage_summary {
            if let Some(awake_ms) = summary.total_awake_time_milli {
                if awake_ms > 0 {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Awake,
                        start_time, // Simplified: stages don't have exact times from WHOOP summary
                        duration_minutes: (awake_ms / 60_000) as u32,
                    });
                }
            }
            if let Some(light_ms) = summary.total_light_sleep_time_milli {
                if light_ms > 0 {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Light,
                        start_time,
                        duration_minutes: (light_ms / 60_000) as u32,
                    });
                }
            }
            if let Some(deep_ms) = summary.total_slow_wave_sleep_time_milli {
                if deep_ms > 0 {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Deep,
                        start_time,
                        duration_minutes: (deep_ms / 60_000) as u32,
                    });
                }
            }
            if let Some(rem_ms) = summary.total_rem_sleep_time_milli {
                if rem_ms > 0 {
                    stages.push(SleepStage {
                        stage_type: SleepStageType::Rem,
                        start_time,
                        duration_minutes: (rem_ms / 60_000) as u32,
                    });
                }
            }
        }

        Ok(SleepSession {
            id: sleep.id,
            start_time,
            end_time,
            time_in_bed,
            total_sleep_time,
            sleep_efficiency,
            sleep_score: score
                .and_then(|s| s.sleep_performance_percentage)
                .map(|p| p as f32),
            stages,
            hrv_during_sleep: None, // HRV is in recovery, not sleep
            respiratory_rate: score.and_then(|s| s.respiratory_rate).map(|r| r as f32),
            temperature_variation: None,
            wake_count: stage_summary
                .and_then(|s| s.disturbance_count)
                .map(|c| c as u32),
            sleep_onset_latency: None,
            provider: oauth_providers::WHOOP.to_owned(),
        })
    }

    /// Convert WHOOP cycle to recovery metrics
    fn convert_cycle_to_recovery(cycle: &WhoopCycle) -> AppResult<RecoveryMetrics> {
        let date = DateTime::parse_from_rfc3339(&cycle.start)
            .map_err(|e| AppError::internal(format!("Failed to parse cycle start: {e}")))?
            .with_timezone(&Utc);

        let score = cycle.score.as_ref();
        let recovery = score.and_then(|s| s.recovery.as_ref());

        Ok(RecoveryMetrics {
            date,
            recovery_score: recovery.and_then(|r| r.recovery_score).map(|s| s as f32),
            readiness_score: recovery.and_then(|r| r.recovery_score).map(|s| s as f32), // WHOOP recovery = readiness
            hrv_status: recovery.and_then(|r| r.hrv_rmssd_milli).map(|hrv| {
                // Classify HRV into status categories
                if hrv > 100.0 {
                    "high".to_owned()
                } else if hrv > 50.0 {
                    "normal".to_owned()
                } else {
                    "low".to_owned()
                }
            }),
            sleep_score: None,  // Sleep score comes from separate sleep endpoint
            stress_level: None, // WHOOP doesn't have explicit stress metric
            training_load: score.and_then(|s| s.strain).map(|s| s as f32),
            resting_heart_rate: recovery
                .and_then(|r| r.resting_heart_rate)
                .map(|hr| hr as u32),
            body_temperature: recovery.and_then(|r| r.skin_temp_celsius).map(|t| t as f32),
            resting_respiratory_rate: None,
            provider: oauth_providers::WHOOP.to_owned(),
        })
    }

    /// Convert WHOOP body measurement to health metrics
    fn convert_body_measurement_to_health(
        measurement: &WhoopBodyMeasurement,
        date: DateTime<Utc>,
    ) -> HealthMetrics {
        HealthMetrics {
            date,
            weight: measurement.weight_kilogram,
            body_fat_percentage: None,
            muscle_mass: None,
            bone_mass: None,
            body_water_percentage: None,
            bmr: None,
            blood_pressure: None,
            blood_glucose: None,
            vo2_max: None,
            provider: oauth_providers::WHOOP.to_owned(),
        }
    }
}

impl Default for WhoopProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FitnessProvider for WhoopProvider {
    fn name(&self) -> &'static str {
        oauth_providers::WHOOP
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        info!("Setting WHOOP credentials");
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
                    provider: oauth_providers::WHOOP.to_owned(),
                    details: "No credentials available".to_owned(),
                };
                return Err(AppError::external_service("WHOOP", err.to_string()));
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

        info!("Refreshing WHOOP access token");

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
                    "WHOOP",
                    format!("Failed to send token refresh request: {e}"),
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err = ProviderError::AuthenticationFailed {
                provider: oauth_providers::WHOOP.to_owned(),
                reason: format!("token refresh failed with status: {status}"),
            };
            return Err(AppError::external_service("WHOOP", err.to_string()));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            AppError::external_service(
                "WHOOP",
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

    #[instrument(skip(self), fields(provider = "whoop", api_call = "get_athlete"))]
    async fn get_athlete(&self) -> AppResult<Athlete> {
        let profile: WhoopUserProfile = self.api_request("user/profile/basic").await?;

        Ok(Athlete {
            id: profile.user_id.to_string(),
            username: profile.email.clone().unwrap_or_default(),
            firstname: profile.first_name,
            lastname: profile.last_name,
            profile_picture: None, // WHOOP doesn't provide profile pictures via API
            provider: oauth_providers::WHOOP.to_owned(),
        })
    }

    #[instrument(
        skip(self, params),
        fields(
            provider = "whoop",
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

        // WHOOP uses token-based pagination, offset is not directly supported
        // For offset support, we'd need to paginate through until we reach the offset
        // For simplicity, we'll fetch from the beginning
        if params.offset.is_some_and(|o| o > 0) {
            warn!("WHOOP provider offset pagination is limited - fetching from beginning");
        }

        // Build endpoint with optional time filter (WHOOP supports start/end parameters)
        let mut endpoint = format!("activity/workout?limit={page_limit}");
        if let Some(after) = params.after {
            if let Some(dt) = chrono::DateTime::from_timestamp(after, 0) {
                let _ = write!(endpoint, "&start={}", dt.format("%Y-%m-%dT%H:%M:%S%.3fZ"));
            }
        }
        if let Some(before) = params.before {
            if let Some(dt) = chrono::DateTime::from_timestamp(before, 0) {
                let _ = write!(endpoint, "&end={}", dt.format("%Y-%m-%dT%H:%M:%S%.3fZ"));
            }
        }

        let response: WhoopPaginatedResponse<WhoopWorkout> = self.api_request(&endpoint).await?;

        let mut activities = Vec::with_capacity(response.records.len());
        for workout in &response.records {
            match Self::convert_workout(workout) {
                Ok(activity) => activities.push(activity),
                Err(e) => {
                    warn!("Failed to convert WHOOP workout: {e}");
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
        let mut endpoint = format!("activity/workout?limit={limit}");

        // If cursor provided, use it as the next_token
        if let Some(cursor) = &params.cursor {
            if let Some((_, token)) = cursor.decode() {
                let _ = write!(endpoint, "&nextToken={token}");
            }
        }

        let response: WhoopPaginatedResponse<WhoopWorkout> = self.api_request(&endpoint).await?;

        let mut activities = Vec::with_capacity(response.records.len());
        for workout in &response.records {
            match Self::convert_workout(workout) {
                Ok(activity) => activities.push(activity),
                Err(e) => {
                    warn!("Failed to convert WHOOP workout: {e}");
                }
            }
        }

        // Determine if there are more results
        let has_more = response.next_token.is_some();

        // Create next cursor from next_token
        let next_cursor = response.next_token.as_ref().map(|token| {
            // Use current time and token as cursor
            Cursor::new(Utc::now(), token)
        });

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
        fields(provider = "whoop", api_call = "get_activity", activity_id = %id)
    )]
    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let endpoint = format!("activity/workout/{id}");
        let workout: WhoopWorkout = self.api_request(&endpoint).await?;
        Self::convert_workout(&workout)
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        // WHOOP doesn't have a direct stats endpoint
        // We could aggregate from activities, but that would be expensive
        // Return empty stats for now
        Ok(Stats {
            total_activities: 0,
            total_distance: 0.0,
            total_duration: 0,
            total_elevation_gain: 0.0,
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // WHOOP doesn't track personal records in the same way
        Ok(vec![])
    }

    #[instrument(
        skip(self),
        fields(provider = "whoop", api_call = "get_sleep_sessions")
    )]
    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        let start_str = start_date.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let end_str = end_date.format("%Y-%m-%dT%H:%M:%S%.3fZ");

        let endpoint = format!("activity/sleep?start={start_str}&end={end_str}&limit=50");

        let response: WhoopPaginatedResponse<WhoopSleep> = self
            .api_request(&endpoint)
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::WHOOP.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        let mut sessions = Vec::with_capacity(response.records.len());
        for sleep in response.records {
            match Self::convert_sleep(sleep) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    warn!("Failed to convert WHOOP sleep: {e}");
                }
            }
        }

        Ok(sessions)
    }

    #[instrument(
        skip(self),
        fields(provider = "whoop", api_call = "get_latest_sleep_session")
    )]
    async fn get_latest_sleep_session(&self) -> Result<SleepSession, ProviderError> {
        let endpoint = "activity/sleep?limit=1";

        let response: WhoopPaginatedResponse<WhoopSleep> = self
            .api_request(endpoint)
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::WHOOP.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        let sleep = response
            .records
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::NotFound {
                provider: oauth_providers::WHOOP.to_owned(),
                resource_type: "sleep_session".to_owned(),
                resource_id: "latest".to_owned(),
            })?;

        Self::convert_sleep(sleep).map_err(|e| ProviderError::InvalidData {
            provider: oauth_providers::WHOOP.to_owned(),
            field: "sleep".to_owned(),
            reason: e.to_string(),
        })
    }

    #[instrument(
        skip(self),
        fields(provider = "whoop", api_call = "get_recovery_metrics")
    )]
    async fn get_recovery_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<RecoveryMetrics>, ProviderError> {
        let start_str = start_date.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let end_str = end_date.format("%Y-%m-%dT%H:%M:%S%.3fZ");

        let endpoint = format!("cycle?start={start_str}&end={end_str}&limit=50");

        let response: WhoopPaginatedResponse<WhoopCycle> = self
            .api_request(&endpoint)
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::WHOOP.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        let mut metrics = Vec::with_capacity(response.records.len());
        for cycle in &response.records {
            match Self::convert_cycle_to_recovery(cycle) {
                Ok(recovery) => metrics.push(recovery),
                Err(e) => {
                    warn!("Failed to convert WHOOP cycle to recovery: {e}");
                }
            }
        }

        Ok(metrics)
    }

    #[instrument(
        skip(self),
        fields(provider = "whoop", api_call = "get_health_metrics")
    )]
    async fn get_health_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<HealthMetrics>, ProviderError> {
        // Note: WHOOP body measurement endpoint doesn't support date range filtering.
        // The start_date and end_date parameters are part of the trait interface but
        // WHOOP only returns current measurements. We log the requested range for debugging.
        debug!(
            "WHOOP get_health_metrics requested for {} to {} (date filtering not supported)",
            start_date, end_date
        );

        let measurement: WhoopBodyMeasurement = self
            .api_request("user/measurement/body")
            .await
            .map_err(|e| ProviderError::ApiError {
                provider: oauth_providers::WHOOP.to_owned(),
                status_code: 500,
                message: e.to_string(),
                retryable: true,
            })?;

        // Return single health metric for current date
        let health = Self::convert_body_measurement_to_health(&measurement, Utc::now());

        Ok(vec![health])
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
                        "Failed to revoke WHOOP access token - continuing with credential cleanup"
                    );
                })
                .ok();
            info!("Attempted to revoke WHOOP access token");
        }

        // Clear credentials regardless of revoke success
        *self.credentials.write().await = None;
        Ok(())
    }
}

// ============================================================================
// Provider Factory
// ============================================================================

use super::core::ProviderFactory;

/// Factory for creating WHOOP provider instances
pub struct WhoopProviderFactory;

impl ProviderFactory for WhoopProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(WhoopProvider::with_config(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::WHOOP]
    }
}
