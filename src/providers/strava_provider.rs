// ABOUTME: Clean Strava API provider implementation using unified provider architecture
// ABOUTME: Handles OAuth2 authentication and data fetching with proper error handling

use super::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use crate::constants::oauth_providers;
use crate::models::{Activity, Athlete, PersonalRecord, SportType, Stats};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

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
            default_scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        };

        Self {
            config,
            credentials: None,
            client: Client::new(),
        }
    }

    /// Create provider with custom configuration
    #[must_use]
    pub fn with_config(config: ProviderConfig) -> Self {
        Self {
            config,
            credentials: None,
            client: Client::new(),
        }
    }

    /// Make authenticated API request
    async fn api_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let credentials = self
            .credentials
            .as_ref()
            .context("No credentials available for Strava API request")?;

        let access_token = credentials
            .access_token
            .as_ref()
            .context("No access token available")?;

        let url = format!(
            "{}/{}",
            self.config.api_base_url,
            endpoint.trim_start_matches('/')
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .context("Failed to send request to Strava API")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Strava API request failed with status {}: {}",
                status,
                text
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse Strava API response")
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
            expires_at: Some(Utc.timestamp_opt(token_response.expires_at, 0).unwrap()),
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
        let limit = limit.unwrap_or(30).min(200); // Strava max is 200
        let page = offset.unwrap_or(0) / limit + 1;

        let endpoint = format!("athlete/activities?per_page={limit}&page={page}");
        let strava_activities: Vec<StravaActivityResponse> = self.api_request(&endpoint).await?;

        let mut activities = Vec::new();

        for activity in strava_activities {
            let start_date = DateTime::parse_from_rfc3339(&activity.start_date)
                .context("Failed to parse activity start date")?
                .with_timezone(&Utc);

            activities.push(Activity {
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
            });
        }

        Ok(activities)
    }

    async fn get_activity(&self, id: &str) -> Result<Activity> {
        let endpoint = format!("activities/{id}");
        let strava_activity: StravaActivityResponse = self.api_request(&endpoint).await?;

        let start_date = DateTime::parse_from_rfc3339(&strava_activity.start_date)
            .context("Failed to parse activity start date")?
            .with_timezone(&Utc);

        Ok(Activity {
            id: strava_activity.id.to_string(),
            name: strava_activity.name,
            sport_type: Self::parse_sport_type(&strava_activity.activity_type),
            start_date,
            distance_meters: strava_activity.distance.map(f64::from),
            duration_seconds: u64::from(strava_activity.elapsed_time.unwrap_or(0)),
            elevation_gain: strava_activity.total_elevation_gain.map(f64::from),
            average_speed: strava_activity.average_speed.map(f64::from),
            max_speed: strava_activity.max_speed.map(f64::from),
            average_heart_rate: strava_activity.average_heartrate.map(|hr| {
                // Safe: heart rate values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    hr as u32
                }
            }),
            max_heart_rate: strava_activity.max_heartrate.map(|hr| {
                // Safe: heart rate values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    hr as u32
                }
            }),
            average_cadence: strava_activity.average_cadence.map(|c| {
                // Safe: cadence values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    c as u32
                }
            }),
            average_power: strava_activity.average_watts.map(|p| {
                // Safe: power values are always positive and within u32 range
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    p as u32
                }
            }),
            max_power: strava_activity.max_watts.map(|p| {
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
            suffer_score: strava_activity.suffer_score.map(|s| {
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
        // Strava doesn't have a direct PR endpoint, would need to analyze activities
        // For now return empty vec - this could be computed from activities
        Ok(vec![])
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(credentials) = &self.credentials {
            if let Some(access_token) = &credentials.access_token {
                info!("Revoking Strava access token");

                if let Some(revoke_url) = &self.config.revoke_url {
                    let params = [("access_token", access_token.as_str())];

                    let response = self.client.post(revoke_url).form(&params).send().await;

                    match response {
                        Ok(_) => info!("Successfully revoked Strava token"),
                        Err(e) => warn!("Failed to revoke Strava token: {}", e),
                    }
                }
            }
        }

        self.credentials = None;
        Ok(())
    }
}
