// ABOUTME: Tenant-aware Strava provider implementation with isolated OAuth credentials
// ABOUTME: Provides Strava API integration respecting tenant boundaries and rate limits
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - HTTP client Arc sharing across async operations (shared_client().clone())
// - String ownership for name formatting (firstname.clone(), lastname.clone())

use super::tenant_provider::TenantFitnessProvider;
use crate::constants::api_provider_limits;
use crate::errors::{AppError, AppResult};
use crate::models::{Activity, Athlete, PersonalRecord, Stats};
use crate::tenant::{TenantContext, TenantOAuthClient, TenantOAuthCredentials};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use crate::utils::http_client::shared_client;

/// Strava athlete response from API
#[derive(Debug, Deserialize)]
struct StravaAthlete {
    id: u64,
    username: Option<String>,
    firstname: Option<String>,
    lastname: Option<String>,
    profile_medium: Option<String>,
}

/// Strava activity response from API
#[derive(Debug, Deserialize)]
struct StravaActivity {
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

/// Tenant-aware Strava provider
pub struct TenantStravaProvider {
    oauth_client: Arc<TenantOAuthClient>,
    client: Client,
    credentials: Option<TenantOAuthCredentials>,
    access_token: Option<String>,
}

impl TenantStravaProvider {
    /// Create new tenant-aware Strava provider
    #[must_use]
    pub fn new(oauth_client: Arc<TenantOAuthClient>) -> Self {
        Self {
            oauth_client,
            client: shared_client().clone(),
            credentials: None,
            access_token: None,
        }
    }

    /// Get the access token, returning an error if not authenticated
    fn get_access_token(&self) -> AppResult<&str> {
        self.access_token
            .as_deref()
            .ok_or_else(|| AppError::auth_required())
    }
}

#[async_trait]
impl TenantFitnessProvider for TenantStravaProvider {
    async fn authenticate_tenant(
        &mut self,
        tenant_context: &TenantContext,
        provider: &str,
        database: &dyn DatabaseProvider,
    ) -> AppResult<()> {
        // Get tenant credentials
        let credentials = self
            .oauth_client
            .get_tenant_credentials(tenant_context.tenant_id, provider, database)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "OAuth credentials for tenant {} and provider {}",
                    tenant_context.tenant_id,
                    provider
                ))
            })?;

        // Store credentials for later use
        self.credentials = Some(credentials);

        Ok(())
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        let token = self.get_access_token()?;

        let response: StravaAthlete = self
            .client
            .get(format!("{}/athlete", crate::constants::api::strava_api_base()))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::external_service("strava", format!("Failed to fetch athlete: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::internal(format!("JSON parsing failed: {e}")))?;

        // Clone name fields before using them in the closure to avoid borrow checker issues
        let firstname_clone = response.firstname.clone();
        let lastname_clone = response.lastname.clone();

        Ok(Athlete {
            id: response.id.to_string(),
            username: response.username.unwrap_or_else(|| {
                format!(
                    "{} {}",
                    firstname_clone.unwrap_or_default(),
                    lastname_clone.unwrap_or_default()
                )
                .trim()
                .to_string()
            }),
            firstname: response.firstname,
            lastname: response.lastname,
            profile_picture: response.profile_medium,
            provider: "strava".to_owned(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        let token = self.get_access_token()?;

        let mut url = url::Url::parse(&format!("{}/athlete/activities", crate::constants::api::strava_api_base()))
            .map_err(|e| AppError::internal(format!("URL parsing failed: {e}")))?;

        if let Some(limit) = limit {
            url.query_pairs_mut()
                .append_pair("per_page", &limit.to_string());
        }
        if let Some(offset) = offset {
            url.query_pairs_mut()
                .append_pair("page", &((offset / limit.unwrap_or(api_provider_limits::strava::DEFAULT_ACTIVITIES_PER_PAGE)) + 1).to_string());
        }

        let response: Vec<StravaActivity> = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::external_service("strava", format!("Failed to fetch activities: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::internal(format!("JSON parsing failed: {e}")))?;

        // Use default fitness config for sport type mapping
        let fitness_config = crate::config::FitnessConfig::default();

        let activities = response
            .into_iter()
            .map(|activity| Activity {
                id: activity.id.to_string(),
                name: activity.name,
                sport_type: crate::models::SportType::from_provider_string(
                    &activity.activity_type,
                    &fitness_config,
                ),
                start_date: chrono::DateTime::parse_from_rfc3339(&activity.start_date)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc),
                duration_seconds: u64::from(activity.elapsed_time.unwrap_or(0)),
                distance_meters: activity.distance.map(f64::from),
                elevation_gain: activity.total_elevation_gain.map(f64::from),
                average_speed: activity.average_speed.map(f64::from),
                max_speed: activity.max_speed.map(f64::from),
                // Safe: heart rates are always positive integers in normal ranges (0-250 bpm)
                average_heart_rate: activity.average_heartrate.map(|hr| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        hr as u32
                    }
                }),
                // Safe: heart rates are always positive integers in normal ranges (0-250 bpm)
                max_heart_rate: activity.max_heartrate.map(|hr| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        hr as u32
                    }
                }),
                // Safe: cadence values are always positive integers in normal ranges (0-MAX_NORMAL_CADENCE rpm)
                average_cadence: activity.average_cadence.map(|c| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        c as u32
                    }
                }),
                // Safe: power values are always positive integers in normal ranges (0-2000 watts)
                average_power: activity.average_watts.map(|w| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        w as u32
                    }
                }),
                // Safe: power values are always positive integers in normal ranges (0-2000 watts)
                max_power: activity.max_watts.map(|w| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        w as u32
                    }
                }),
                // Safe: suffer score is always positive integer from 0-100 range
                suffer_score: activity.suffer_score.map(|s| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        s as u32
                    }
                }),
                provider: "strava".to_owned(),
                ..Default::default()
            })
            .collect();

        Ok(activities)
    }

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let token = self.get_access_token()?;

        let response: StravaActivity = self
            .client
            .get(format!("{}/activities/{id}", crate::constants::api::strava_api_base()))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::external_service("strava", format!("Failed to fetch activity: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::internal(format!("JSON parsing failed: {e}")))?;

        // Use default fitness config for sport type mapping
        let fitness_config = crate::config::FitnessConfig::default();

        Ok(Activity {
            id: response.id.to_string(),
            name: response.name,
            sport_type: crate::models::SportType::from_provider_string(
                &response.activity_type,
                &fitness_config,
            ),
            start_date: chrono::DateTime::parse_from_rfc3339(&response.start_date)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .with_timezone(&chrono::Utc),
            duration_seconds: u64::from(response.elapsed_time.unwrap_or(0)),
            distance_meters: response.distance.map(f64::from),
            elevation_gain: response.total_elevation_gain.map(f64::from),
            average_speed: response.average_speed.map(f64::from),
            max_speed: response.max_speed.map(f64::from),
            // Safe: heart rates are always positive integers in normal ranges (0-250 bpm)
            average_heart_rate: response.average_heartrate.map(|hr| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    hr as u32
                }
            }),
            // Safe: heart rates are always positive integers in normal ranges (0-250 bpm)
            max_heart_rate: response.max_heartrate.map(|hr| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    hr as u32
                }
            }),
            // Safe: cadence values are always positive integers in normal ranges (0-MAX_NORMAL_CADENCE rpm)
            average_cadence: response.average_cadence.map(|c| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    c as u32
                }
            }),
            // Safe: power values are always positive integers in normal ranges (0-2000 watts)
            average_power: response.average_watts.map(|w| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    w as u32
                }
            }),
            // Safe: power values are always positive integers in normal ranges (0-2000 watts)
            max_power: response.max_watts.map(|w| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    w as u32
                }
            }),
            // Safe: suffer score is always positive integer from 0-100 range
            suffer_score: response.suffer_score.map(|s| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    s as u32
                }
            }),
            provider: "strava".to_owned(),
            ..Default::default()
        })
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        // Validate token access before proceeding
        self.get_access_token()?;

        // Strava doesn't have a single stats endpoint, so we'll return empty stats
        // In a real implementation, you'd aggregate data from multiple endpoints
        Ok(Stats {
            total_activities: 0,
            total_distance: 0.0,
            total_duration: 0,
            total_elevation_gain: 0.0,
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Strava doesn't provide a direct personal records endpoint
        // In a real implementation, you'd analyze activities to find PRs
        Ok(vec![])
    }

    fn provider_name(&self) -> &'static str {
        "strava"
    }
}
