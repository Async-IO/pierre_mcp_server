// ABOUTME: Clean Strava API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 authentication and data fetching with proper error handling

use super::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use crate::constants::{api_provider_limits, oauth_providers};
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use crate::utils::http_client::shared_client;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

/// Strava API response for athlete data
#[derive(Debug, Deserialize)]
struct StravaAthleteResponse {
    id: u64,
    username: Option<String>,
    firstname: Option<String>,
    lastname: Option<String>,
    profile_medium: Option<String>,
}

/// Strava API response for activity data
#[derive(Debug, Deserialize)]
struct StravaActivityResponse {
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
    credentials: Option<OAuth2Credentials>,
    client: Client,
}

impl StravaProvider {
    /// Create a new Strava provider with default configuration
    #[must_use]
    pub fn new() -> Self {
        let config = ProviderConfig {
            name: oauth_providers::STRAVA.to_string(),
            auth_url: "https://www.strava.com/oauth/authorize".to_string(),
            token_url: "https://www.strava.com/oauth/token".to_string(),
            api_base_url: "https://www.strava.com/api/v3".to_string(),
            revoke_url: Some("https://www.strava.com/oauth/deauthorize".to_string()),
            default_scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                .split(',')
                .map(str::to_string)
                .collect(),
        };

        Self {
            config,
            credentials: None,
            client: shared_client().clone(),
        }
    }

    /// Create provider with custom configuration
    #[must_use]
    pub fn with_config(config: ProviderConfig) -> Self {
        Self {
            config,
            credentials: None,
            client: shared_client().clone(),
        }
    }

    /// Make authenticated API request
    async fn api_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        tracing::info!("Starting API request to endpoint: {}", endpoint);

        let credentials = self
            .credentials
            .as_ref()
            .context("No credentials available for Strava API request")?;

        let access_token = credentials
            .access_token
            .as_ref()
            .context("No access token available")?;

        // Reject test/invalid tokens with proper error message
        if access_token.starts_with("at_") || access_token.len() < 40 {
            anyhow::bail!(
                "Invalid Strava access token. Please authenticate with Strava first to access real data."
            );
        }

        tracing::info!("Using access token: {}...", &access_token[..10]);

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
            .context("Failed to send request to Strava API")?;

        tracing::info!("Received HTTP response with status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            tracing::error!(
                "Strava API request failed - status: {}, body: {}",
                status,
                text
            );
            return Err(anyhow::anyhow!(
                "Strava API request failed with status {}: {}",
                status,
                text
            ));
        }

        tracing::info!("Parsing JSON response from Strava API");
        let result = response
            .json()
            .await
            .context("Failed to parse Strava API response");

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
            _ => SportType::Other(strava_type.to_string()),
        }
    }

    /// Convert Strava activity response to internal Activity model
    fn convert_strava_activity(activity: StravaActivityResponse) -> Result<Activity> {
        let start_date = DateTime::parse_from_rfc3339(&activity.start_date)
            .context("Failed to parse activity start date")?
            .with_timezone(&Utc);

        Ok(Activity {
            id: activity.id.to_string(),
            name: activity.name,
            sport_type: Self::parse_sport_type(&activity.activity_type),
            start_date,
            distance_meters: activity.distance.map(f64::from),
            duration_seconds: u64::from(activity.elapsed_time.unwrap_or(0)),
            elevation_gain: activity.total_elevation_gain.map(f64::from),
            average_speed: activity.average_speed.map(f64::from),
            max_speed: activity.max_speed.map(f64::from),
            average_heart_rate: activity.average_heartrate.map(|hr| {
                // Safe: heart rate values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    hr as u32
                }
            }),
            max_heart_rate: activity.max_heartrate.map(|hr| {
                // Safe: heart rate values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    hr as u32
                }
            }),
            average_cadence: activity.average_cadence.map(|c| {
                // Safe: cadence values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    c as u32
                }
            }),
            average_power: activity.average_watts.map(|p| {
                // Safe: power values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    p as u32
                }
            }),
            max_power: activity.max_watts.map(|p| {
                // Safe: power values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    p as u32
                }
            }),
            calories: None, // Strava doesn't provide calories in basic activity data
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
            suffer_score: activity.suffer_score.map(|s| {
                // Safe: suffer score values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    s as u32
                }
            }),
            time_series_data: None,
            start_latitude: None,
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: oauth_providers::STRAVA.to_string(),
        })
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

    async fn set_credentials(&mut self, credentials: OAuth2Credentials) -> Result<()> {
        info!("Setting Strava credentials");
        self.credentials = Some(credentials);
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        if let Some(creds) = &self.credentials {
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

    async fn refresh_token_if_needed(&mut self) -> Result<()> {
        let needs_refresh = if let Some(creds) = &self.credentials {
            creds
                .expires_at
                .is_some_and(|expires_at| Utc::now() + chrono::Duration::minutes(5) > expires_at)
        } else {
            return Err(anyhow::anyhow!("No credentials available"));
        };

        if !needs_refresh {
            return Ok(());
        }

        let credentials = self
            .credentials
            .take()
            .context("No credentials available for refresh")?;

        let refresh_token = credentials
            .refresh_token
            .context("No refresh token available")?;

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
            .context("Failed to send token refresh request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Token refresh failed with status: {}",
                response.status()
            ));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: String,
            expires_at: i64,
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token refresh response")?;

        let new_credentials = OAuth2Credentials {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            access_token: Some(token_response.access_token),
            refresh_token: Some(token_response.refresh_token),
            expires_at: Utc.timestamp_opt(token_response.expires_at, 0).single(),
            scopes: credentials.scopes,
        };

        self.credentials = Some(new_credentials);
        Ok(())
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        let strava_athlete: StravaAthleteResponse = self.api_request("athlete").await?;

        Ok(Athlete {
            id: strava_athlete.id.to_string(),
            username: strava_athlete.username.unwrap_or_default(),
            firstname: strava_athlete.firstname,
            lastname: strava_athlete.lastname,
            profile_picture: strava_athlete.profile_medium,
            provider: oauth_providers::STRAVA.to_string(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
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

    async fn get_activity(&self, id: &str) -> Result<Activity> {
        let endpoint = format!("activities/{id}");
        let strava_activity: StravaActivityResponse = self.api_request(&endpoint).await?;
        Self::convert_strava_activity(strava_activity)
    }

    async fn get_stats(&self) -> Result<Stats> {
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

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        // Strava doesn't provide personal records via API in the same format
        // This would require analyzing activities to determine PRs
        Ok(vec![])
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(credentials) = &self.credentials {
            if let Some(access_token) = &credentials.access_token {
                // Attempt to revoke the token if revoke URL is available
                if let Some(revoke_url) = &self.config.revoke_url {
                    let _result = self
                        .client
                        .post(revoke_url)
                        .form(&[("token", access_token)])
                        .send()
                        .await;
                    // Don't fail if revoke fails, just log it
                    info!("Attempted to revoke Strava access token");
                }
            }
        }

        // Clear credentials regardless of revoke success
        self.credentials = None;
        Ok(())
    }
}

impl StravaProvider {
    /// Fetch activities using single API call (for requests <= `BULK_ACTIVITY_FETCH_THRESHOLD`)
    async fn get_activities_single_page(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Activity>> {
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
    ) -> Result<Vec<Activity>> {
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
