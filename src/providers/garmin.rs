use async_trait::async_trait;
use anyhow::Result;
use crate::models::{Activity, Athlete, Stats, PersonalRecord};
use super::{FitnessProvider, AuthData};

pub struct GarminProvider {
}

impl GarminProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl FitnessProvider for GarminProvider {
    async fn authenticate(&mut self, _auth_data: AuthData) -> Result<()> {
        Err(anyhow::anyhow!("Garmin provider not yet implemented"))
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        Err(anyhow::anyhow!("Garmin provider not yet implemented"))
    }

    async fn get_activities(&self, _limit: Option<usize>, _offset: Option<usize>) -> Result<Vec<Activity>> {
        Err(anyhow::anyhow!("Garmin provider not yet implemented"))
    }

    async fn get_activity(&self, _id: &str) -> Result<Activity> {
        Err(anyhow::anyhow!("Garmin provider not yet implemented"))
    }

    async fn get_stats(&self) -> Result<Stats> {
        Err(anyhow::anyhow!("Garmin provider not yet implemented"))
    }

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        Err(anyhow::anyhow!("Garmin provider not yet implemented"))
    }

    fn provider_name(&self) -> &'static str {
        "Garmin"
    }
}