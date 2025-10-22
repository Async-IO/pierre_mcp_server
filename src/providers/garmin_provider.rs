// ABOUTME: Garmin Connect API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 PKCE authentication and fitness data fetching with proper error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use super::utils::{self, RetryConfig};
use crate::constants::{api_provider_limits, oauth_providers};
use crate::errors::AppError;
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use crate::pagination::{CursorPage, PaginationParams};
use crate::utils::http_client::shared_client;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tracing::info;

/// Garmin API response for athlete data
#[derive(Debug, Deserialize)]
struct GarminAthleteResponse {
    user_id: String,
    display_name: Option<String>,
    full_name: Option<String>,
    profile_image_url: Option<String>,
}

/// Garmin API response for activity data
#[derive(Debug, Deserialize)]
struct GarminActivityResponse {
    activity_id: u64,
    activity_name: String,
    activity_type: String,
    start_time_gmt: String,
    distance: Option<f64>,
    duration: Option<f64>,
    elevation_gain: Option<f64>,
    average_speed: Option<f64>,
    max_speed: Option<f64>,
    average_hr: Option<f32>,
    max_hr: Option<f32>,
    average_running_cadence: Option<f32>,
    average_power: Option<f32>,
    max_power: Option<f32>,
    calories: Option<f64>,
}

/// Garmin API response for summary stats
#[derive(Debug, Deserialize)]
struct GarminStatsResponse {
    #[serde(rename = "totalActivities")]
    activities_count: Option<u64>,
    #[serde(rename = "totalDistance")]
    distance: Option<f64>,
    #[serde(rename = "totalDuration")]
    duration: Option<f64>,
    #[serde(rename = "totalElevationGain")]
    elevation_gain: Option<f64>,
}

/// Garmin Connect provider implementation
pub struct GarminProvider {
    config: ProviderConfig,
    credentials: tokio::sync::RwLock<Option<OAuth2Credentials>>,
    client: Client,
}

impl GarminProvider {
    /// Create a new Garmin provider with default configuration
    #[must_use]
    pub fn new() -> Self {
        let config = ProviderConfig {
            name: oauth_providers::GARMIN.to_string(),
            auth_url: crate::constants::env_config::garmin_auth_url(),
            token_url: crate::constants::env_config::garmin_token_url(),
            api_base_url: crate::constants::env_config::garmin_api_base(),
            revoke_url: Some(crate::constants::env_config::garmin_revoke_url()),
            default_scopes: crate::constants::oauth::GARMIN_DEFAULT_SCOPES
                .split(',')
                .map(str::to_string)
                .collect(),
        };

        Self {
            config,
            credentials: tokio::sync::RwLock::new(None),
            // Clone Arc<Client> from shared singleton - cheap reference counting operation
            client: shared_client().clone(),
        }
    }

    /// Create provider with custom configuration
    #[must_use]
    pub fn with_config(config: ProviderConfig) -> Self {
        Self {
            config,
            credentials: tokio::sync::RwLock::new(None),
            // Clone Arc<Client> from shared singleton - cheap reference counting operation
            client: shared_client().clone(),
        }
    }

    /// Make authenticated API request with rate limit handling
    /// Uses shared retry logic with exponential backoff for 429 errors
    async fn api_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Clone access token to avoid holding lock across await
        let access_token = {
            let guard = self.credentials.read().await;
            let credentials = guard
                .as_ref()
                .context("No credentials available for Garmin API request")?;

            let token = credentials
                .access_token
                .clone() // Safe: String ownership needed for async request
                .context("No access token available")?;
            drop(guard); // Release lock immediately after cloning
            token
        };

        let url = format!(
            "{}/{}",
            self.config.api_base_url,
            endpoint.trim_start_matches('/')
        );

        let retry_config = RetryConfig {
            max_retries: 3,
            initial_backoff_ms: 1000,
            retryable_status_codes: vec![StatusCode::TOO_MANY_REQUESTS],
            estimated_block_duration_secs:
                api_provider_limits::garmin::ESTIMATED_RATE_LIMIT_BLOCK_DURATION_SECS,
        };

        utils::api_request_with_retry(&self.client, &url, &access_token, "Garmin", &retry_config)
            .await
    }

    /// Convert Garmin activity type to our `SportType` enum
    /// Based on Garmin Connect activity types from `activity_types.properties`
    /// Source: <https://connect.garmin.com/modern/main/js/properties/activity_types/activity_types.properties>
    fn parse_sport_type(garmin_type: &str) -> SportType {
        match garmin_type.to_lowercase().as_str() {
            // Running variants
            "running" | "run" | "track" | "track_running" => SportType::Run,
            "trail_running" | "trail_run" => SportType::TrailRunning,
            "treadmill" | "treadmill_running" => SportType::VirtualRun,

            // Cycling variants
            "cycling" | "bike" | "biking" | "road_cycling" | "road" | "cyclocross" | "cx" => {
                SportType::Ride
            }
            "mountain_biking" | "mountain_bike_ride" | "mountain" => SportType::MountainBike,
            "indoor_cycling" | "spin" | "virtual_ride" => SportType::VirtualRide,
            "gravel_cycling" | "gravel_ride" => SportType::GravelRide,
            "ebike" | "e_bike_ride" | "e_bike" => SportType::EbikeRide,

            // Swimming variants
            "swimming"
            | "swim"
            | "open_water_swimming"
            | "open_water"
            | "pool_swimming"
            | "lap_swimming" => SportType::Swim,

            // Walking and hiking
            "walking" | "walk" | "casual_walking" => SportType::Walk,
            "hiking" | "hike" => SportType::Hike,

            // Winter sports
            "resort_skiing" | "alpine_skiing" | "downhill_skiing" => SportType::AlpineSkiing,
            "backcountry_skiing_snowboarding" | "backcountry_skiing" => {
                SportType::BackcountrySkiing
            }
            "cross_country_skiing" | "xc_skiing" | "nordic_skiing" => SportType::CrossCountrySkiing,
            "snowboarding" | "snowboard" => SportType::Snowboarding,
            "snowshoeing" | "snowshoe" => SportType::Snowshoe,
            "ice_skating" | "skating" => SportType::IceSkating,

            // Water sports
            "kayaking" | "kayak" => SportType::Kayaking,
            "canoeing" | "canoe" => SportType::Canoeing,
            "rowing" | "row" | "indoor_rowing" => SportType::Rowing,
            "stand_up_paddleboarding" | "sup" | "paddleboarding" => SportType::Paddleboarding,
            "surfing" | "surf" => SportType::Surfing,
            "kitesurfing" | "kiteboarding" => SportType::Kitesurfing,

            // Strength and fitness
            "strength_training" | "weight_training" | "weights" => SportType::StrengthTraining,
            "crossfit" | "cross_fit" => SportType::Crossfit,
            "pilates" => SportType::Pilates,
            "yoga" => SportType::Yoga,

            // Climbing and adventure
            "rock_climbing" | "climbing" | "bouldering" => SportType::RockClimbing,

            // Team sports
            "soccer" | "football" => SportType::Soccer,
            "basketball" => SportType::Basketball,
            "tennis" => SportType::Tennis,
            "golf" => SportType::Golf,

            // Alternative transport
            "skateboarding" | "skateboard" => SportType::Skateboarding,
            "inline_skating" | "roller_skating" => SportType::InlineSkating,

            // Generic/other - combined cardio, fitness equipment, and generic workout types
            "cardio" | "cardio_training" | "elliptical" | "fitness_equipment"
            | "stair_climbing" | "stepper" | "other" | "generic" | "workout" | "training" => {
                SportType::Workout
            }

            // Unmapped types fall through to Other variant
            _ => SportType::Other(garmin_type.to_string()),
        }
    }

    /// Convert Garmin activity response to internal Activity model
    fn convert_garmin_activity(activity: GarminActivityResponse) -> Result<Activity> {
        let start_date = DateTime::parse_from_rfc3339(&activity.start_time_gmt)
            .context("Failed to parse activity start date")?
            .with_timezone(&Utc);

        Ok(Activity {
            id: activity.activity_id.to_string(),
            name: activity.activity_name,
            sport_type: Self::parse_sport_type(&activity.activity_type),
            start_date,
            distance_meters: activity.distance,
            duration_seconds: activity.duration.map_or(0, utils::conversions::f64_to_u64),
            elevation_gain: activity.elevation_gain,
            average_speed: activity.average_speed,
            max_speed: activity.max_speed,
            average_heart_rate: activity.average_hr.map(utils::conversions::f32_to_u32),
            max_heart_rate: activity.max_hr.map(utils::conversions::f32_to_u32),
            average_cadence: activity
                .average_running_cadence
                .map(utils::conversions::f32_to_u32),
            average_power: activity.average_power.map(utils::conversions::f32_to_u32),
            max_power: activity.max_power.map(utils::conversions::f32_to_u32),
            calories: activity.calories.map(utils::conversions::f64_to_u32),
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
            suffer_score: None,
            time_series_data: None,
            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: oauth_providers::GARMIN.to_string(),
        })
    }
}

impl Default for GarminProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FitnessProvider for GarminProvider {
    fn name(&self) -> &'static str {
        oauth_providers::GARMIN
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> Result<()> {
        info!("Setting Garmin credentials");
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

    async fn refresh_token_if_needed(&self) -> Result<()> {
        // Check if refresh is needed and extract credentials
        let (needs_refresh, credentials) = {
            let guard = self.credentials.read().await;
            let needs_refresh = if let Some(creds) = guard.as_ref() {
                creds.expires_at.is_some_and(|expires_at| {
                    Utc::now() + chrono::Duration::minutes(5) > expires_at
                })
            } else {
                return Err(AppError::internal("No credentials available").into());
            };

            let credentials = guard
                .as_ref()
                .context("No credentials available for refresh")?
                .clone(); // Safe: OAuth2Credentials ownership for refresh operation
            drop(guard); // Release lock early to avoid contention

            (needs_refresh, credentials)
        };

        if !needs_refresh {
            return Ok(());
        }

        let refresh_token = credentials
            .refresh_token
            .context("No refresh token available")?;

        info!("Refreshing Garmin access token");

        let mut new_credentials = utils::refresh_oauth_token(
            &self.client,
            &self.config.token_url,
            &credentials.client_id,
            &credentials.client_secret,
            &refresh_token,
            "Garmin",
        )
        .await?;

        // Preserve original scopes
        new_credentials.scopes = credentials.scopes;

        *self.credentials.write().await = Some(new_credentials);
        Ok(())
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        // Source: https://github.com/cyberjunky/python-garminconnect
        // Endpoint: /userprofile-service/userprofile/profile
        let garmin_athlete: GarminAthleteResponse = self
            .api_request("userprofile-service/userprofile/profile")
            .await?;

        Ok(Athlete {
            id: garmin_athlete.user_id,
            // Use as_deref() to borrow rather than clone the String
            username: garmin_athlete
                .display_name
                .as_deref()
                .unwrap_or_default()
                .to_string(),
            firstname: garmin_athlete.full_name,
            lastname: None,
            profile_picture: garmin_athlete.profile_image_url,
            provider: oauth_providers::GARMIN.to_string(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
        let requested_limit =
            limit.unwrap_or(api_provider_limits::garmin::DEFAULT_ACTIVITIES_PER_PAGE);
        let start_offset = offset.unwrap_or(0);

        tracing::info!(
            "Starting get_activities - requested_limit: {}, offset: {}",
            requested_limit,
            start_offset
        );

        if requested_limit <= api_provider_limits::garmin::MAX_ACTIVITIES_PER_REQUEST {
            return self
                .get_activities_single_page(requested_limit, start_offset)
                .await;
        }

        self.get_activities_multi_page(requested_limit, start_offset)
            .await
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> Result<CursorPage<Activity>> {
        // Stub implementation: delegate to offset-based pagination
        let activities = self.get_activities(Some(params.limit), None).await?;
        Ok(CursorPage::new(activities, None, None, false))
    }

    async fn get_activity(&self, id: &str) -> Result<Activity> {
        // Source: https://github.com/cyberjunky/python-garminconnect
        // Endpoint: /activity-service/activity/{activity_id}
        let endpoint = format!("activity-service/activity/{id}");
        let garmin_activity: GarminActivityResponse = self.api_request(&endpoint).await?;
        Self::convert_garmin_activity(garmin_activity)
    }

    async fn get_stats(&self) -> Result<Stats> {
        // Source: https://github.com/cyberjunky/python-garminconnect
        // Using aggregate stats endpoint which provides all-time totals
        // Alternative endpoint /usersummary-service/usersummary/daily/{display_name}?calendarDate={date}
        // provides daily summaries but requires display_name and specific date parameters
        let stats: GarminStatsResponse = self
            .api_request("usersummary-service/stats/aggregate")
            .await?;

        Ok(Stats {
            total_activities: stats.activities_count.unwrap_or(0),
            total_distance: stats.distance.unwrap_or(0.0),
            total_duration: stats.duration.map_or(0, utils::conversions::f64_to_u64),
            total_elevation_gain: stats.elevation_gain.unwrap_or(0.0),
        })
    }

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        // Garmin personal records require activity analysis
        // Implementation TBD based on Garmin API structure
        Ok(vec![])
    }

    async fn disconnect(&self) -> Result<()> {
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
            let _result = self
                .client
                .post(&revoke_url)
                .form(&[("token", access_token.as_str())])
                .send()
                .await;
            // Don't fail if revoke fails, just log it
            info!("Attempted to revoke Garmin access token");
        }

        // Clear credentials regardless of revoke success
        *self.credentials.write().await = None;
        Ok(())
    }
}

impl GarminProvider {
    /// Fetch activities using single API call
    async fn get_activities_single_page(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Activity>> {
        // Source: https://github.com/cyberjunky/python-garminconnect
        // Endpoint: /activitylist-service/activities/search/activities?start={offset}&limit={limit}
        let endpoint = format!(
            "activitylist-service/activities/search/activities?start={offset}&limit={limit}"
        );

        tracing::info!("Single page request - endpoint: {}", endpoint);

        let garmin_activities: Vec<GarminActivityResponse> = self.api_request(&endpoint).await?;
        tracing::info!(
            "Received {} activities from single page",
            garmin_activities.len()
        );

        let mut activities = Vec::new();
        for activity in garmin_activities {
            activities.push(Self::convert_garmin_activity(activity)?);
        }

        Ok(activities)
    }

    /// Fetch activities using multiple API calls
    async fn get_activities_multi_page(
        &self,
        total_limit: usize,
        start_offset: usize,
    ) -> Result<Vec<Activity>> {
        let mut all_activities = Vec::with_capacity(total_limit);

        let activities_per_page = api_provider_limits::garmin::MAX_ACTIVITIES_PER_REQUEST;
        let pages_needed = total_limit.div_ceil(activities_per_page);

        tracing::info!(
            "Multi-page request - total_limit: {}, pages_needed: {}, start_offset: {}",
            total_limit,
            pages_needed,
            start_offset
        );

        for page_index in 0..pages_needed {
            let remaining_activities = total_limit - all_activities.len();
            let current_page_limit = remaining_activities.min(activities_per_page);

            let current_offset = start_offset + (page_index * activities_per_page);
            // Source: https://github.com/cyberjunky/python-garminconnect
            // Endpoint: /activitylist-service/activities/search/activities?start={offset}&limit={limit}
            let endpoint = format!(
                "activitylist-service/activities/search/activities?start={current_offset}&limit={current_page_limit}"
            );

            tracing::info!(
                "Fetching page {} of {} - endpoint: {}",
                page_index + 1,
                pages_needed,
                endpoint
            );

            match self
                .api_request::<Vec<GarminActivityResponse>>(&endpoint)
                .await
            {
                Ok(garmin_activities) => {
                    tracing::info!(
                        "Page {} returned {} activities",
                        page_index + 1,
                        garmin_activities.len()
                    );

                    for activity in garmin_activities {
                        if all_activities.len() >= total_limit {
                            break;
                        }
                        all_activities.push(Self::convert_garmin_activity(activity)?);
                    }

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
