// ABOUTME: Garmin Connect API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 PKCE authentication and fitness data fetching with proper error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use super::utils::{self, RetryConfig};
use crate::constants::{api_provider_limits, oauth_providers};
use crate::errors::{AppError, AppResult};
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use crate::pagination::{CursorPage, PaginationParams};
use crate::utils::http_client::shared_client;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tracing::{debug, info, warn};

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
        let config = crate::constants::get_server_config().map_or_else(
            || ProviderConfig {
                name: oauth_providers::GARMIN.to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connectapi.garmin.com/oauth-service/oauth/access_token"
                    .to_owned(),
                api_base_url: "https://connectapi.garmin.com".to_owned(),
                revoke_url: Some(
                    "https://connectapi.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ),
                default_scopes: vec!["activity".to_owned()],
            },
            |server_config| ProviderConfig {
                name: oauth_providers::GARMIN.to_owned(),
                auth_url: server_config.external_services.garmin_api.auth_url.clone(),
                token_url: server_config.external_services.garmin_api.token_url.clone(),
                api_base_url: server_config.external_services.garmin_api.base_url.clone(),
                revoke_url: Some(
                    server_config
                        .external_services
                        .garmin_api
                        .revoke_url
                        .clone(),
                ),
                default_scopes: crate::constants::oauth::GARMIN_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_owned)
                    .collect(),
            },
        );

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
    async fn api_request<T>(&self, endpoint: &str) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Clone access token to avoid holding lock across await
        let access_token = {
            let guard = self.credentials.read().await;
            let credentials = guard.as_ref().ok_or_else(|| {
                AppError::internal("No credentials available for Garmin API request")
            })?;

            let token = credentials
                .access_token
                .clone() // Safe: String ownership needed for async request
                .ok_or_else(|| AppError::internal("No access token available"))?;
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
            _ => SportType::Other(garmin_type.to_owned()),
        }
    }

    /// Convert Garmin activity response to internal Activity model
    fn convert_garmin_activity(activity: GarminActivityResponse) -> AppResult<Activity> {
        let start_date = DateTime::parse_from_rfc3339(&activity.start_time_gmt)
            .map_err(|e| AppError::internal(format!("Failed to parse activity start date: {e}")))?
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

            // Optional fields pending Garmin API documentation
            // These will be populated from GarminActivityResponse once API schema is available
            workout_type: None,
            sport_type_detail: None,
            segment_efforts: None,

            provider: oauth_providers::GARMIN.to_owned(),
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

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
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

    async fn refresh_token_if_needed(&self) -> AppResult<()> {
        // Check if refresh is needed and extract credentials
        let (needs_refresh, credentials) = {
            let guard = self.credentials.read().await;
            let needs_refresh = if let Some(creds) = guard.as_ref() {
                creds.expires_at.is_some_and(|expires_at| {
                    Utc::now() + chrono::Duration::minutes(5) > expires_at
                })
            } else {
                return Err(AppError::internal("No credentials available"));
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

    async fn get_athlete(&self) -> AppResult<Athlete> {
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
                .to_owned(),
            firstname: garmin_athlete.full_name,
            lastname: None,
            profile_picture: garmin_athlete.profile_image_url,
            provider: oauth_providers::GARMIN.to_owned(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
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
    ) -> AppResult<CursorPage<Activity>> {
        // Garmin API uses numeric pagination - delegate to offset-based approach
        let activities = self.get_activities(Some(params.limit), None).await?;
        Ok(CursorPage::new(activities, None, None, false))
    }

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        // Source: https://github.com/cyberjunky/python-garminconnect
        // Endpoint: /activity-service/activity/{activity_id}
        let endpoint = format!("activity-service/activity/{id}");
        let garmin_activity: GarminActivityResponse = self.api_request(&endpoint).await?;
        Self::convert_garmin_activity(garmin_activity)
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        // Source: https://github.com/cyberjunky/python-garminconnect
        // Using aggregate stats endpoint which provides all-time totals
        // Alternative endpoint /usersummary-service/usersummary/daily/{display_name}?calendarDate={date}
        // provides daily summaries but requires display_name and specific date parameters
        let stats: GarminStatsResponse = self
            .api_request("usersummary-service/stats/aggregate")
            .await?;

        let total_activities = stats.activities_count.unwrap_or_else(|| {
            debug!("Garmin API returned None for activities_count - defaulting to 0");
            0
        });

        let total_distance = stats.distance.unwrap_or_else(|| {
            debug!("Garmin API returned None for distance - defaulting to 0.0");
            0.0
        });

        let total_duration = stats.duration.map_or_else(
            || {
                debug!("Garmin API returned None for duration - defaulting to 0");
                0
            },
            utils::conversions::f64_to_u64,
        );

        let total_elevation_gain = stats.elevation_gain.unwrap_or_else(|| {
            debug!("Garmin API returned None for elevation_gain - defaulting to 0.0");
            0.0
        });

        Ok(Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Garmin Connect does not expose a dedicated personal records endpoint
        // Personal records would need to be computed from activity history analysis
        // or extracted from the athlete profile if available in future API updates
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
                        "Failed to revoke Garmin access token - continuing with credential cleanup"
                    );
                })
                .ok();
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
    ) -> AppResult<Vec<Activity>> {
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
    /// Build the endpoint URL for a page of activities
    fn build_activities_endpoint(offset: usize, limit: usize) -> String {
        format!("activitylist-service/activities/search/activities?start={offset}&limit={limit}")
    }

    /// Convert and add Garmin activities to the collection
    fn add_converted_activities(
        activities: &mut Vec<Activity>,
        garmin_activities: Vec<GarminActivityResponse>,
        limit: usize,
    ) -> AppResult<()> {
        for activity in garmin_activities {
            if activities.len() >= limit {
                break;
            }
            activities.push(Self::convert_garmin_activity(activity)?);
        }
        Ok(())
    }

    /// Check if pagination should stop based on current state
    fn should_stop_pagination(
        activities_count: usize,
        page_index: usize,
        activities_per_page: usize,
        total_limit: usize,
    ) -> bool {
        if activities_count >= total_limit {
            return true;
        }

        let expected_count = (page_index + 1) * activities_per_page.min(total_limit);
        if activities_count < expected_count {
            tracing::info!(
                "Reached end of activities - got {} total, breaking early",
                activities_count
            );
            return true;
        }

        false
    }

    /// Fetch a single page of activities and add to collection
    async fn fetch_activities_page(
        &self,
        all_activities: &mut Vec<Activity>,
        page_index: usize,
        pages_needed: usize,
        start_offset: usize,
        activities_per_page: usize,
        total_limit: usize,
    ) -> AppResult<bool> {
        let remaining = total_limit - all_activities.len();
        let current_page_limit = remaining.min(activities_per_page);
        let current_offset = start_offset + (page_index * activities_per_page);

        let endpoint = Self::build_activities_endpoint(current_offset, current_page_limit);
        tracing::info!(
            "Fetching page {} of {} - endpoint: {}",
            page_index + 1,
            pages_needed,
            endpoint
        );

        let garmin_activities = self
            .api_request::<Vec<GarminActivityResponse>>(&endpoint)
            .await?;

        tracing::info!(
            "Page {} returned {} activities",
            page_index + 1,
            garmin_activities.len()
        );

        Self::add_converted_activities(all_activities, garmin_activities, total_limit)?;

        Ok(Self::should_stop_pagination(
            all_activities.len(),
            page_index,
            activities_per_page,
            total_limit,
        ))
    }

    async fn get_activities_multi_page(
        &self,
        total_limit: usize,
        start_offset: usize,
    ) -> AppResult<Vec<Activity>> {
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
            let should_stop = self
                .fetch_activities_page(
                    &mut all_activities,
                    page_index,
                    pages_needed,
                    start_offset,
                    activities_per_page,
                    total_limit,
                )
                .await?;

            if should_stop {
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

// ============================================================================
// Provider Factory
// ============================================================================

use super::core::ProviderFactory;

/// Factory for creating Garmin provider instances
pub struct GarminProviderFactory;

impl ProviderFactory for GarminProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(GarminProvider::with_config(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &[oauth_providers::GARMIN]
    }
}
