// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Fitness-specific configuration for sport types and intelligence parameters

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

/// Main fitness configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessConfig {
    pub sport_types: HashMap<String, String>,
    pub intelligence: IntelligenceConfig,
    pub weather_api: Option<WeatherApiConfig>,
}

/// Intelligence analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig {
    pub effort_thresholds: EffortThresholds,
    pub zone_thresholds: ZoneThresholds,
    pub weather_mapping: WeatherMapping,
    pub personal_records: PersonalRecordConfig,
}

/// Effort level thresholds for categorizing workout intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortThresholds {
    pub light_max: f32,
    pub moderate_max: f32,
    pub hard_max: f32,
    // > hard_max = very_high
}

/// Heart rate zone thresholds (as percentage of max HR)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneThresholds {
    pub recovery_max: f32,
    pub endurance_max: f32,
    pub tempo_max: f32,
    pub threshold_max: f32,
    // > threshold_max = vo2max
}

/// Weather detection and mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherMapping {
    pub rain_keywords: Vec<String>,
    pub snow_keywords: Vec<String>,
    pub wind_threshold: f32,
}

/// Personal record detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecordConfig {
    pub pace_improvement_threshold: f32,
    pub distance_pr_types: Vec<String>,
    pub time_pr_types: Vec<String>,
}

/// Weather API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherApiConfig {
    pub provider: String,
    pub enabled: bool,
    pub cache_duration_hours: u64,
    pub request_timeout_seconds: u64,
    pub fallback_to_mock: bool,
    pub rate_limit_requests_per_minute: u64,
}

impl FitnessConfig {
    /// Load fitness configuration from file or use defaults
    #[allow(dead_code)]
    pub fn load(path: Option<String>) -> Result<Self> {
        // Try explicit path first
        if let Some(config_path) = path {
            return Self::load_from_file(&config_path);
        }
        
        // Try default fitness config file
        if Path::new("fitness_config.toml").exists() {
            return Self::load_from_file("fitness_config.toml");
        }
        
        // Fall back to embedded defaults
        Ok(Self::default())
    }
    
    /// Load configuration from a specific file
    #[allow(dead_code)]
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read fitness config file: {}", path))?;
        
        let config: FitnessConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse fitness config file: {}", path))?;
        
        Ok(config)
    }
    
    /// Get the internal sport type name for a provider sport type
    pub fn map_sport_type(&self, provider_sport: &str) -> Option<&str> {
        self.sport_types.get(provider_sport).map(|s| s.as_str())
    }
    
    /// Get all configured sport type mappings
    #[allow(dead_code)]
    pub fn get_sport_mappings(&self) -> &HashMap<String, String> {
        &self.sport_types
    }
}

impl Default for FitnessConfig {
    fn default() -> Self {
        let mut sport_types = HashMap::new();
        
        // Standard activities
        sport_types.insert("Run".to_string(), "run".to_string());
        sport_types.insert("Ride".to_string(), "bike_ride".to_string());
        sport_types.insert("Swim".to_string(), "swim".to_string());
        sport_types.insert("Walk".to_string(), "walk".to_string());
        sport_types.insert("Hike".to_string(), "hike".to_string());
        
        // Virtual/Indoor activities
        sport_types.insert("VirtualRide".to_string(), "virtual_ride".to_string());
        sport_types.insert("VirtualRun".to_string(), "virtual_run".to_string());
        sport_types.insert("Workout".to_string(), "workout".to_string());
        sport_types.insert("Yoga".to_string(), "yoga".to_string());
        
        // E-bike and specialty cycling
        sport_types.insert("EBikeRide".to_string(), "ebike_ride".to_string());
        sport_types.insert("MountainBikeRide".to_string(), "mountain_bike".to_string());
        sport_types.insert("GravelRide".to_string(), "gravel_ride".to_string());
        
        // Winter sports
        sport_types.insert("CrossCountrySkiing".to_string(), "cross_country_skiing".to_string());
        sport_types.insert("AlpineSkiing".to_string(), "alpine_skiing".to_string());
        sport_types.insert("Snowboarding".to_string(), "snowboarding".to_string());
        sport_types.insert("Snowshoe".to_string(), "snowshoe".to_string());
        sport_types.insert("IceSkate".to_string(), "ice_skating".to_string());
        sport_types.insert("BackcountrySki".to_string(), "backcountry_skiing".to_string());
        
        // Water sports
        sport_types.insert("Kayaking".to_string(), "kayaking".to_string());
        sport_types.insert("Canoeing".to_string(), "canoeing".to_string());
        sport_types.insert("Rowing".to_string(), "rowing".to_string());
        sport_types.insert("StandUpPaddling".to_string(), "paddleboarding".to_string());
        sport_types.insert("Surfing".to_string(), "surfing".to_string());
        sport_types.insert("Kitesurf".to_string(), "kitesurfing".to_string());
        
        // Strength and fitness
        sport_types.insert("WeightTraining".to_string(), "strength_training".to_string());
        sport_types.insert("Crossfit".to_string(), "crossfit".to_string());
        sport_types.insert("Pilates".to_string(), "pilates".to_string());
        
        // Climbing and adventure
        sport_types.insert("RockClimbing".to_string(), "rock_climbing".to_string());
        sport_types.insert("TrailRunning".to_string(), "trail_running".to_string());
        
        // Team and racquet sports
        sport_types.insert("Soccer".to_string(), "soccer".to_string());
        sport_types.insert("Basketball".to_string(), "basketball".to_string());
        sport_types.insert("Tennis".to_string(), "tennis".to_string());
        sport_types.insert("Golf".to_string(), "golf".to_string());
        
        // Alternative transport
        sport_types.insert("Skateboard".to_string(), "skateboarding".to_string());
        sport_types.insert("InlineSkate".to_string(), "inline_skating".to_string());
        
        Self {
            sport_types,
            intelligence: IntelligenceConfig::default(),
            weather_api: Some(WeatherApiConfig::default()),
        }
    }
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            effort_thresholds: EffortThresholds::default(),
            zone_thresholds: ZoneThresholds::default(),
            weather_mapping: WeatherMapping::default(),
            personal_records: PersonalRecordConfig::default(),
        }
    }
}

impl Default for EffortThresholds {
    fn default() -> Self {
        Self {
            light_max: 3.0,
            moderate_max: 5.0,
            hard_max: 7.0,
        }
    }
}

impl Default for ZoneThresholds {
    fn default() -> Self {
        Self {
            recovery_max: 60.0,
            endurance_max: 70.0,
            tempo_max: 80.0,
            threshold_max: 90.0,
        }
    }
}

impl Default for WeatherMapping {
    fn default() -> Self {
        Self {
            rain_keywords: vec![
                "rain".to_string(),
                "shower".to_string(),
                "storm".to_string(),
                "thunderstorm".to_string(),
                "drizzle".to_string(),
            ],
            snow_keywords: vec![
                "snow".to_string(),
                "blizzard".to_string(),
                "sleet".to_string(),
                "flurry".to_string(),
            ],
            wind_threshold: 15.0,
        }
    }
}

impl Default for PersonalRecordConfig {
    fn default() -> Self {
        Self {
            pace_improvement_threshold: 5.0,
            distance_pr_types: vec![
                "longest_run".to_string(),
                "longest_ride".to_string(),
                "longest_ski".to_string(),
            ],
            time_pr_types: vec![
                "fastest_5k".to_string(),
                "fastest_10k".to_string(),
                "fastest_marathon".to_string(),
            ],
        }
    }
}

impl Default for WeatherApiConfig {
    fn default() -> Self {
        Self {
            provider: "openweathermap".to_string(),
            enabled: true,
            cache_duration_hours: 24,
            request_timeout_seconds: 10,
            fallback_to_mock: true,
            rate_limit_requests_per_minute: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_default_fitness_config() {
        let config = FitnessConfig::default();
        
        // Test standard sport mappings
        assert_eq!(config.map_sport_type("Run"), Some("run"));
        assert_eq!(config.map_sport_type("Ride"), Some("bike_ride"));
        assert_eq!(config.map_sport_type("CrossCountrySkiing"), Some("cross_country_skiing"));
        assert_eq!(config.map_sport_type("NonExistent"), None);
    }
    
    #[test]
    fn test_effort_thresholds() {
        let config = FitnessConfig::default();
        let thresholds = &config.intelligence.effort_thresholds;
        
        assert_eq!(thresholds.light_max, 3.0);
        assert_eq!(thresholds.moderate_max, 5.0);
        assert_eq!(thresholds.hard_max, 7.0);
    }
    
    #[test]
    fn test_zone_thresholds() {
        let config = FitnessConfig::default();
        let zones = &config.intelligence.zone_thresholds;
        
        assert_eq!(zones.recovery_max, 60.0);
        assert_eq!(zones.endurance_max, 70.0);
        assert_eq!(zones.tempo_max, 80.0);
        assert_eq!(zones.threshold_max, 90.0);
    }
    
    #[test]
    fn test_weather_mapping() {
        let config = FitnessConfig::default();
        let weather = &config.intelligence.weather_mapping;
        
        assert!(weather.rain_keywords.contains(&"rain".to_string()));
        assert!(weather.snow_keywords.contains(&"snow".to_string()));
        assert_eq!(weather.wind_threshold, 15.0);
    }
    
    #[test]
    fn test_config_file_loading() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, r#"
[sport_types]
"TestRun" = "test_run"
"TestRide" = "test_ride"

[intelligence.effort_thresholds]
light_max = 2.5
moderate_max = 4.5
hard_max = 6.5

[intelligence.zone_thresholds]
recovery_max = 55.0
endurance_max = 65.0
tempo_max = 75.0
threshold_max = 85.0

[intelligence.weather_mapping]
rain_keywords = ["rain", "storm"]
snow_keywords = ["snow"]
wind_threshold = 20.0

[intelligence.personal_records]
pace_improvement_threshold = 3.0
distance_pr_types = ["longest_test"]
time_pr_types = ["fastest_test"]
        "#)?;
        
        let config = FitnessConfig::load_from_file(temp_file.path().to_str().unwrap())?;
        
        // Test custom sport mappings
        assert_eq!(config.map_sport_type("TestRun"), Some("test_run"));
        assert_eq!(config.map_sport_type("TestRide"), Some("test_ride"));
        
        // Test custom thresholds
        assert_eq!(config.intelligence.effort_thresholds.light_max, 2.5);
        assert_eq!(config.intelligence.zone_thresholds.recovery_max, 55.0);
        assert_eq!(config.intelligence.weather_mapping.wind_threshold, 20.0);
        assert_eq!(config.intelligence.personal_records.pace_improvement_threshold, 3.0);
        
        Ok(())
    }
}