// ABOUTME: Fitness data provider integrations for external platforms and devices
// ABOUTME: Unifies access to Strava, Fitbit, and other fitness platforms with consistent APIs
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::models::{Activity, Athlete, PersonalRecord, Stats};
use anyhow::Result;
use async_trait::async_trait;

pub mod fitbit;
pub mod strava;

#[async_trait]
pub trait FitnessProvider: Send + Sync {
    async fn authenticate(&mut self, auth_data: AuthData) -> Result<()>;

    async fn get_athlete(&self) -> Result<Athlete>;

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>>;

    async fn get_activity(&self, id: &str) -> Result<Activity>;

    async fn get_stats(&self) -> Result<Stats>;

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;

    fn provider_name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub enum AuthData {
    OAuth2 {
        client_id: String,
        client_secret: String,
        access_token: Option<String>,
        refresh_token: Option<String>,
    },
    ApiKey(String),
}

/// Create a fitness provider instance based on the provider type
///
/// # Errors
///
/// Returns an error if the provider type is not supported
pub fn create_provider(provider_type: &str) -> Result<Box<dyn FitnessProvider>> {
    match provider_type.to_lowercase().as_str() {
        "strava" => Ok(Box::new(strava::StravaProvider::new())),
        "fitbit" => Ok(Box::new(fitbit::FitbitProvider::new())),
        _ => Err(anyhow::anyhow!(
            "Unknown provider: {provider_type}. Currently supported: strava, fitbit"
        )),
    }
}
