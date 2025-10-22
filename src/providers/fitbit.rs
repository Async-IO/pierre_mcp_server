// ABOUTME: Fitbit API integration and health data fetching
// ABOUTME: Handles Fitbit authentication, activity retrieval, and health metrics
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Fitbit provider implementation for fitness data retrieval.
//!
//! This module provides integration with the Fitbit Web API, supporting:
//! - `OAuth2` authentication with PKCE for enhanced security
//! - Activity data retrieval with comprehensive metrics
//! - User profile information
//! - Aggregated fitness statistics
//!
//! # API Documentation
//! - [Fitbit Web API](https://dev.fitbit.com/build/reference/web-api/)
//! - [OAuth2 Authorization](https://dev.fitbit.com/build/reference/web-api/developer-guide/authorization/)

use super::{AuthData, FitnessProvider};
use crate::errors::AppError;
use crate::models::{Activity, Athlete, HeartRateZone, PersonalRecord, SportType, Stats};
use crate::oauth2_client::PkceParams;
use crate::pagination::{CursorPage, PaginationParams};
use crate::utils::http_client::api_client;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

const FITBIT_API_BASE: &str = "https://api.fitbit.com/1";
const FITBIT_AUTH_URL: &str = "https://www.fitbit.com/oauth2/authorize";

/// Fitbit provider implementation supporting `OAuth2` with `PKCE`
pub struct FitbitProvider {
    client: Client,
    access_token: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    refresh_token: Option<String>,
}

impl Default for FitbitProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FitbitProvider {
    /// Create a new Fitbit provider instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: api_client(),
            access_token: None,
            client_id: None,
            client_secret: None,
            refresh_token: None,
        }
    }

    /// Get `OAuth2` authorization URL for Fitbit
    ///
    /// # Arguments
    /// * `redirect_uri` - The redirect URI registered with your Fitbit app
    /// * `state` - A unique state parameter for CSRF protection
    ///
    /// # Scopes
    /// Requests the following Fitbit scopes:
    /// - `activity` - Access to activities and exercise logs
    /// - `profile` - Access to profile information
    /// - `sleep` - Access to sleep data (for future enhancement)
    ///
    /// # Errors
    /// Returns an error if `client_id` is not configured
    pub fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String> {
        let client_id = self
            .client_id
            .as_ref()
            .context("Client ID not configured")?;

        let mut url = url::Url::parse(FITBIT_AUTH_URL)?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "activity profile sleep")
            .append_pair("state", state);

        Ok(url.to_string())
    }

    /// Get `OAuth2` authorization URL with PKCE support for enhanced security
    ///
    /// # Arguments
    /// * `redirect_uri` - The redirect URI registered with your Fitbit app
    /// * `state` - A unique state parameter for CSRF protection
    /// * `pkce` - PKCE parameters for enhanced security
    ///
    /// # Errors
    /// Returns an error if `client_id` is not configured or URL parsing fails
    pub fn get_auth_url_with_pkce(
        &self,
        redirect_uri: &str,
        state: &str,
        pkce: &PkceParams,
    ) -> Result<String> {
        let client_id = self
            .client_id
            .as_ref()
            .context("Client ID not configured")?;

        let mut url = url::Url::parse(FITBIT_AUTH_URL)?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "activity profile sleep")
            .append_pair("state", state)
            .append_pair("code_challenge", &pkce.code_challenge)
            .append_pair("code_challenge_method", &pkce.code_challenge_method);

        Ok(url.to_string())
    }

    /// Exchange authorization code for access and refresh tokens
    ///
    /// # Arguments
    /// * `code` - Authorization code received from Fitbit
    /// * `redirect_uri` - The same redirect URI used in authorization
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client ID or secret is not configured
    /// - HTTP request to token endpoint fails
    /// - Fitbit API returns error response
    /// - Response cannot be parsed as JSON
    /// - Token exchange fails
    pub async fn exchange_code(
        &mut self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<(String, String)> {
        let client_id = self.client_id.as_ref().context("Client ID not set")?;
        let client_secret = self
            .client_secret
            .as_ref()
            .context("Client secret not set")?;

        let (token, _) = crate::oauth2_client::fitbit::exchange_fitbit_code(
            &self.client,
            client_id,
            client_secret,
            code,
            redirect_uri,
        )
        .await?;

        self.access_token = Some(token.access_token.clone());
        self.refresh_token.clone_from(&token.refresh_token);

        info!("Fitbit authentication successful");

        // Return tokens for storage
        Ok((token.access_token, token.refresh_token.unwrap_or_default()))
    }

    /// Exchange authorization code with PKCE support for enhanced security
    ///
    /// # Arguments
    /// * `code` - Authorization code received from Fitbit
    /// * `redirect_uri` - The same redirect URI used in authorization
    /// * `pkce` - PKCE parameters used in authorization
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client ID or secret is not configured
    /// - HTTP request to token endpoint fails
    /// - PKCE verification fails
    /// - Fitbit API returns error response
    /// - Response cannot be parsed as JSON
    /// - Token exchange fails
    pub async fn exchange_code_with_pkce(
        &mut self,
        code: &str,
        redirect_uri: &str,
        pkce: &PkceParams,
    ) -> Result<(String, String)> {
        let client_id = self.client_id.as_ref().context("Client ID not set")?;
        let client_secret = self
            .client_secret
            .as_ref()
            .context("Client secret not set")?;

        let (token, _) = crate::oauth2_client::fitbit::exchange_fitbit_code_with_pkce(
            &self.client,
            client_id,
            client_secret,
            code,
            redirect_uri,
            pkce,
        )
        .await?;

        self.access_token = Some(token.access_token.clone());
        self.refresh_token.clone_from(&token.refresh_token);

        info!("Fitbit authentication with PKCE successful");

        // Return tokens for storage
        Ok((token.access_token, token.refresh_token.unwrap_or_default()))
    }

    /// Refresh access token using refresh token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No refresh token is available
    /// - Client ID or secret is not configured
    /// - HTTP request to token endpoint fails
    /// - Fitbit API returns error response
    /// - Response cannot be parsed as JSON
    /// - Token refresh fails
    pub async fn refresh_access_token(&mut self) -> Result<(String, String)> {
        let refresh_token = self
            .refresh_token
            .as_ref()
            .context("No refresh token available")?;

        let client_id = self.client_id.as_ref().context("Client ID not set")?;
        let client_secret = self
            .client_secret
            .as_ref()
            .context("Client secret not set")?;

        let new_token = crate::oauth2_client::fitbit::refresh_fitbit_token(
            &self.client,
            client_id,
            client_secret,
            refresh_token,
        )
        .await?;

        self.access_token = Some(new_token.access_token.clone());
        self.refresh_token.clone_from(&new_token.refresh_token);

        info!("Fitbit token refreshed successfully");

        // Return tokens for storage
        Ok((
            new_token.access_token,
            new_token.refresh_token.unwrap_or_default(),
        ))
    }

    /// Get activities for a specific date range
    /// Fitbit API requires date-based queries rather than pagination
    async fn get_activities_for_period(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<FitbitActivity>> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        let response: FitbitActivitiesResponse = self
            .client
            .get(format!("{FITBIT_API_BASE}/user/-/activities/list.json"))
            .bearer_auth(token)
            .query(&[
                ("beforeDate", end_date),
                ("afterDate", start_date),
                ("sort", "desc"),
                ("limit", "100"),
                ("offset", "0"),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(response.activities)
    }
}

#[async_trait]
impl FitnessProvider for FitbitProvider {
    async fn authenticate(&mut self, auth_data: AuthData) -> Result<()> {
        match auth_data {
            AuthData::OAuth2 {
                client_id,
                client_secret,
                access_token,
                refresh_token,
            } => {
                self.client_id = Some(client_id);
                self.client_secret = Some(client_secret);
                self.access_token = access_token;
                self.refresh_token = refresh_token;
                Ok(())
            }
            AuthData::ApiKey(_) => Err(AppError::invalid_input("Fitbit requires OAuth2 authentication").into()),
        }
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        let response: FitbitUser = self
            .client
            .get(format!("{FITBIT_API_BASE}/user/-/profile.json"))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        Ok(Athlete {
            id: response.user.encoded_id,
            username: response.user.display_name,
            firstname: response.user.first_name,
            lastname: response.user.last_name,
            profile_picture: response.user.avatar,
            provider: "fitbit".into(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
        // Fitbit API works with date ranges rather than offset pagination
        // Get activities from the last 30 days by default
        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(30);

        let activities = self
            .get_activities_for_period(
                &start_date.format("%Y-%m-%d").to_string(),
                &end_date.format("%Y-%m-%d").to_string(),
            )
            .await?;

        let mut result: Vec<Activity> = activities
            .into_iter()
            .map(std::convert::Into::into)
            .collect();

        // Apply limit if specified
        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
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
        let token = self.access_token.as_ref().context("Not authenticated")?;

        let response: FitbitActivityDetail = self
            .client
            .get(format!("{FITBIT_API_BASE}/user/-/activities/{id}.json"))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        Ok(response.activity.into())
    }

    async fn get_stats(&self) -> Result<Stats> {
        let token = self.access_token.as_ref().context("Not authenticated")?;

        // Get lifetime stats from Fitbit
        let response: FitbitLifetimeStats = self
            .client
            .get(format!("{FITBIT_API_BASE}/user/-/activities.json"))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;

        // Fitbit provides lifetime totals
        Ok(Stats {
            total_activities: 0, // Fitbit doesn't provide activity count in lifetime stats
            total_distance: response.lifetime.total.distance * 1000.0, // Convert km to meters
            total_duration: 0,   // Not available in lifetime stats
            total_elevation_gain: response.lifetime.total.floors * 3.0, // Estimate: 1 floor ≈ 3m
        })
    }

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        // Fitbit doesn't have a direct personal records API
        // This would need to be calculated from activity history
        Ok(vec![])
    }

    fn provider_name(&self) -> &'static str {
        "Fitbit"
    }
}

// Fitbit API response structures

#[derive(Debug, Deserialize)]
struct FitbitUser {
    user: FitbitUserProfile,
}

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

#[derive(Debug, Deserialize)]
struct FitbitActivitiesResponse {
    activities: Vec<FitbitActivity>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitActivity {
    activity_id: u64,
    activity_name: String,
    activity_type_id: u32,
    start_time: String,
    duration: u64,         // milliseconds
    distance: Option<f64>, // km
    steps: Option<u32>,
    calories: Option<u32>,
    elevation_gain: Option<f64>, // meters
    average_heart_rate: Option<u32>,
    heart_rate_zones: Option<Vec<FitbitHeartRateZone>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FitbitHeartRateZone {
    name: String,
    min: u32,
    max: u32,
    minutes: u32,
}

#[derive(Debug, Deserialize)]
struct FitbitActivityDetail {
    activity: FitbitActivity,
}

#[derive(Debug, Deserialize)]
struct FitbitLifetimeStats {
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

impl From<FitbitActivity> for Activity {
    fn from(fitbit: FitbitActivity) -> Self {
        // Parse start time
        let start_date = DateTime::parse_from_rfc3339(&fitbit.start_time)
            .or_else(|_| DateTime::parse_from_str(&fitbit.start_time, "%Y-%m-%dT%H:%M:%S%.3f"))
            .unwrap_or_else(|_| {
                tracing::warn!(
                    "Failed to parse start_time '{}', using current time",
                    fitbit.start_time
                );
                Utc::now().fixed_offset()
            })
            .with_timezone(&Utc);

        // Map Fitbit activity types to our sport types
        let sport_type = match fitbit.activity_type_id {
            90009 => SportType::Run,  // Running
            90001 => SportType::Walk, // Walking
            1071 => SportType::Ride,  // Biking
            90024 => SportType::Swim, // Swimming
            90013 => SportType::Hike, // Hiking
            17190 => SportType::Yoga, // Yoga
            _ => SportType::Other(fitbit.activity_name.clone()),
        };

        Self {
            id: fitbit.activity_id.to_string(),
            name: fitbit.activity_name,
            sport_type,
            start_date,
            duration_seconds: fitbit.duration / 1000, // Convert ms to seconds
            distance_meters: fitbit.distance.map(|d| d * 1000.0), // Convert km to meters
            elevation_gain: fitbit.elevation_gain,
            average_heart_rate: fitbit.average_heart_rate,
            max_heart_rate: None, // Not directly available in Fitbit API
            average_speed: fitbit.distance.and_then(|d| {
                if fitbit.duration > 0 {
                    let duration_seconds =
                        f64::from(u32::try_from(fitbit.duration / 1000).unwrap_or_else(|_| {
                            tracing::warn!(
                                "Duration too large for conversion: {}",
                                fitbit.duration
                            );
                            u32::MAX
                        }));
                    Some((d * 1000.0) / duration_seconds) // m/s
                } else {
                    None
                }
            }),
            max_speed: None, // Not available in Fitbit API
            calories: fitbit.calories,
            steps: fitbit.steps,
            heart_rate_zones: fitbit.heart_rate_zones.map(|zones| {
                zones
                    .into_iter()
                    .map(|zone| HeartRateZone {
                        name: zone.name,
                        min_hr: zone.min,
                        max_hr: zone.max,
                        minutes: zone.minutes,
                    })
                    .collect()
            }),

            // Advanced metrics - all None for basic Fitbit data
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

            start_latitude: None, // Fitbit API doesn't provide GPS coordinates
            start_longitude: None,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: "fitbit".into(),
        }
    }
}
