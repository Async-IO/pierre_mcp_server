// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use async_trait::async_trait;
use anyhow::{Result, Context};
use reqwest::Client;
use serde::Deserialize;
use chrono::{DateTime, Utc};
use crate::models::{Activity, Athlete, Stats, PersonalRecord, SportType};
use crate::config::FitnessConfig;
use crate::oauth2_client::PkceParams;
use super::{FitnessProvider, AuthData};
use tracing::info;

const STRAVA_API_BASE: &str = "https://www.strava.com/api/v3";
const STRAVA_AUTH_URL: &str = "https://www.strava.com/oauth/authorize";

pub struct StravaProvider {
    client: Client,
    access_token: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    refresh_token: Option<String>,
}

impl StravaProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            access_token: None,
            client_id: None,
            client_secret: None,
            refresh_token: None,
        }
    }
    
    #[allow(dead_code)]
    pub fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String> {
        let client_id = self.client_id.as_ref()
            .context("Client ID not configured")?;
        
        let mut url = url::Url::parse(STRAVA_AUTH_URL)?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "read,activity:read_all")
            .append_pair("state", state);

        Ok(url.to_string())
    }

    /// Get authorization URL with PKCE support for enhanced security
    #[allow(dead_code)]
    pub fn get_auth_url_with_pkce(&self, redirect_uri: &str, state: &str, pkce: &PkceParams) -> Result<String> {
        let client_id = self.client_id.as_ref()
            .context("Client ID not configured")?;
        
        let mut url = url::Url::parse(STRAVA_AUTH_URL)?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "read,activity:read_all")
            .append_pair("state", state)
            .append_pair("code_challenge", &pkce.code_challenge)
            .append_pair("code_challenge_method", &pkce.code_challenge_method);
        
        Ok(url.to_string())
    }
    
    #[allow(dead_code)]
    pub async fn exchange_code(&mut self, code: &str) -> Result<(String, String)> {
        let client_id = self.client_id.as_ref()
            .context("Client ID not set")?;
        let client_secret = self.client_secret.as_ref()
            .context("Client secret not set")?;
        
        let (token, athlete) = crate::oauth2_client::strava::exchange_strava_code(
            &self.client,
            client_id,
            client_secret,
            code
        ).await?;
        
        self.access_token = Some(token.access_token.clone());
        self.refresh_token = token.refresh_token.clone();
        
        if let Some(athlete) = athlete {
            info!("Authenticated as Strava athlete: {} ({})", 
                athlete.id, 
                athlete.username.as_deref().unwrap_or("unknown"));
        }
        
        // Return tokens for storage
        Ok((token.access_token, token.refresh_token.unwrap_or_default()))
    }

    /// Exchange authorization code with PKCE support for enhanced security
    #[allow(dead_code)]
    pub async fn exchange_code_with_pkce(&mut self, code: &str, pkce: &PkceParams) -> Result<(String, String)> {
        let client_id = self.client_id.as_ref()
            .context("Client ID not set")?;
        let client_secret = self.client_secret.as_ref()
            .context("Client secret not set")?;
        
        let (token, athlete) = crate::oauth2_client::strava::exchange_strava_code_with_pkce(
            &self.client,
            client_id,
            client_secret,
            code,
            pkce
        ).await?;
        
        self.access_token = Some(token.access_token.clone());
        self.refresh_token = token.refresh_token.clone();
        
        if let Some(athlete) = athlete {
            info!("Authenticated as Strava athlete with PKCE: {} ({})", 
                athlete.id, 
                athlete.username.as_deref().unwrap_or("unknown"));
        }
        
        // Return tokens for storage
        Ok((token.access_token, token.refresh_token.unwrap_or_default()))
    }
    
    #[allow(dead_code)]
    pub async fn refresh_access_token(&mut self) -> Result<(String, String)> {
        let refresh_token = self.refresh_token.as_ref()
            .context("No refresh token available")?;
        
        let client_id = self.client_id.as_ref()
            .context("Client ID not set")?;
        let client_secret = self.client_secret.as_ref()
            .context("Client secret not set")?;
        
        let new_token = crate::oauth2_client::strava::refresh_strava_token(
            &self.client,
            client_id,
            client_secret,
            refresh_token
        ).await?;
        
        self.access_token = Some(new_token.access_token.clone());
        self.refresh_token = new_token.refresh_token.clone();
        
        info!("Token refreshed successfully");
        
        // Return tokens for storage
        Ok((new_token.access_token, new_token.refresh_token.unwrap_or_default()))
    }
}

#[async_trait]
impl FitnessProvider for StravaProvider {
    async fn authenticate(&mut self, auth_data: AuthData) -> Result<()> {
        match auth_data {
            AuthData::OAuth2 { client_id, client_secret, access_token, refresh_token } => {
                self.client_id = Some(client_id);
                self.client_secret = Some(client_secret);
                self.access_token = access_token;
                self.refresh_token = refresh_token;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Strava requires OAuth2 authentication")),
        }
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        let token = self.access_token.as_ref()
            .context("Not authenticated")?;
        
        let response: StravaAthlete = self.client
            .get(format!("{}/athlete", STRAVA_API_BASE))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(Athlete {
            id: response.id.to_string(),
            username: response.username.unwrap_or_default(),
            firstname: response.firstname,
            lastname: response.lastname,
            profile_picture: response.profile,
            provider: "strava".to_string(),
        })
    }

    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Activity>> {
        let token = self.access_token.as_ref()
            .context("Not authenticated")?;
        
        let mut query = vec![];
        if let Some(limit) = limit {
            query.push(("per_page", limit.to_string()));
        }
        if let Some(offset) = offset {
            query.push(("page", (offset / limit.unwrap_or(30) + 1).to_string()));
        }
        
        let response: Vec<StravaActivity> = self.client
            .get(format!("{}/athlete/activities", STRAVA_API_BASE))
            .bearer_auth(token)
            .query(&query)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(response.into_iter().map(|a| a.into()).collect())
    }

    #[allow(dead_code)]
    async fn get_activity(&self, id: &str) -> Result<Activity> {
        let token = self.access_token.as_ref()
            .context("Not authenticated")?;
        
        let response: StravaActivity = self.client
            .get(format!("{}/activities/{}", STRAVA_API_BASE, id))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(response.into())
    }

    async fn get_stats(&self) -> Result<Stats> {
        // Try Strava's athlete stats endpoint first
        if let Ok(strava_stats) = self.get_strava_athlete_stats().await {
            return Ok(strava_stats);
        }
        
        // Fallback: Calculate from recent activities (limited to avoid rate limits)
        let activities = self.get_activities(Some(100), None).await?;
        
        let total_activities = activities.len() as u64;
        let total_distance = activities.iter()
            .filter_map(|a| a.distance_meters)
            .sum();
        let total_duration = activities.iter()
            .map(|a| a.duration_seconds)
            .sum();
        let total_elevation_gain = activities.iter()
            .filter_map(|a| a.elevation_gain)
            .sum();
        
        Ok(Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        })
    }

    #[allow(dead_code)]
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        Ok(vec![])
    }

    #[allow(dead_code)]
    fn provider_name(&self) -> &'static str {
        "Strava"
    }
}

impl StravaProvider {
    // Try to get stats from Strava's athlete stats endpoint
    async fn get_strava_athlete_stats(&self) -> Result<Stats> {
        let token = self.access_token.as_ref()
            .context("Not authenticated")?;
        
        // Get athlete ID first
        let athlete: StravaAthlete = self.client
            .get(format!("{}/athlete", STRAVA_API_BASE))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;
        
        // Get athlete stats
        let response: StravaAthleteStats = self.client
            .get(format!("{}/athletes/{}/stats", STRAVA_API_BASE, athlete.id))
            .bearer_auth(token)
            .send()
            .await?
            .json()
            .await?;
        
        // Convert Strava stats to our format
        Ok(Stats {
            total_activities: response.all_ride_totals.count + response.all_run_totals.count,
            total_distance: response.all_ride_totals.distance + response.all_run_totals.distance,
            total_duration: response.all_ride_totals.moving_time + response.all_run_totals.moving_time,
            total_elevation_gain: response.all_ride_totals.elevation_gain + response.all_run_totals.elevation_gain,
        })
    }
}

#[derive(Debug, Deserialize)]
struct StravaAthlete {
    id: u64,
    username: Option<String>,
    firstname: Option<String>,
    lastname: Option<String>,
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StravaActivity {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    activity_type: String,
    start_date: DateTime<Utc>,
    elapsed_time: u64,
    distance: Option<f64>,
    total_elevation_gain: Option<f64>,
    average_heartrate: Option<f32>,
    max_heartrate: Option<f32>,
    average_speed: Option<f64>,
    max_speed: Option<f64>,
    start_latlng: Option<Vec<f64>>, // [latitude, longitude]
}

impl From<StravaActivity> for Activity {
    fn from(strava: StravaActivity) -> Self {
        // Use default fitness config for sport type mapping
        let fitness_config = FitnessConfig::default();
        
        // Extract GPS coordinates from start_latlng array
        let (start_latitude, start_longitude) = if let Some(coords) = strava.start_latlng {
            if coords.len() >= 2 {
                (Some(coords[0]), Some(coords[1]))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        
        Activity {
            id: strava.id.to_string(),
            name: strava.name,
            sport_type: SportType::from_provider_string(&strava.activity_type, &fitness_config),
            start_date: strava.start_date,
            duration_seconds: strava.elapsed_time,
            distance_meters: strava.distance,
            elevation_gain: strava.total_elevation_gain,
            average_heart_rate: strava.average_heartrate.map(|hr| hr as u32),
            max_heart_rate: strava.max_heartrate.map(|hr| hr as u32),
            average_speed: strava.average_speed,
            max_speed: strava.max_speed,
            calories: None,
            start_latitude,
            start_longitude,
            city: None,
            region: None,
            country: None,
            trail_name: None,
            provider: "strava".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct StravaAthleteStats {
    all_ride_totals: StravaTotals,
    all_run_totals: StravaTotals,
}

#[derive(Debug, Deserialize)]
struct StravaTotals {
    count: u64,
    distance: f64,
    moving_time: u64,
    elevation_gain: f64,
}