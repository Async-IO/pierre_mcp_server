// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use async_trait::async_trait;
use anyhow::Result;
use crate::models::{Activity, Athlete, Stats, PersonalRecord};

pub mod strava;
pub mod fitbit;


#[async_trait]
pub trait FitnessProvider: Send + Sync {
    async fn authenticate(&mut self, auth_data: AuthData) -> Result<()>;
    
    async fn get_athlete(&self) -> Result<Athlete>;
    
    async fn get_activities(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Activity>>;
    
    #[allow(dead_code)]
    async fn get_activity(&self, id: &str) -> Result<Activity>;
    
    async fn get_stats(&self) -> Result<Stats>;
    
    #[allow(dead_code)]
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;
    
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    ApiKey(String),
}

pub fn create_provider(provider_type: &str) -> Result<Box<dyn FitnessProvider>> {
    match provider_type.to_lowercase().as_str() {
        "strava" => Ok(Box::new(strava::StravaProvider::new())),
        "fitbit" => Ok(Box::new(fitbit::FitbitProvider::new())),
        _ => Err(anyhow::anyhow!("Unknown provider: {}. Currently supported: strava, fitbit", provider_type)),
    }
}