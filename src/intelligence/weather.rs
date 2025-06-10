// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Weather service integration for contextual activity analysis

use chrono::{DateTime, Utc, Timelike, Datelike};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use super::WeatherConditions;
use crate::config::fitness_config::WeatherApiConfig;

/// Weather service for fetching historical weather data
pub struct WeatherService {
    client: Client,
    config: WeatherApiConfig,
    cache: HashMap<String, CachedWeatherData>,
}

/// Cached weather data with timestamp
#[derive(Debug, Clone)]
struct CachedWeatherData {
    weather: WeatherConditions,
    cached_at: SystemTime,
}

/// OpenWeatherMap historical API response structure
#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    data: Vec<OpenWeatherHourlyData>,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherHourlyData {
    dt: i64, // Unix timestamp
    temp: f64,
    humidity: Option<f64>,
    wind_speed: Option<f64>,
    weather: Vec<OpenWeatherCondition>,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherCondition {
    main: String,
    #[allow(dead_code)]
    description: String,
}

impl WeatherService {
    /// Create a new weather service with configuration
    pub fn new(config: WeatherApiConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(config.request_timeout_seconds))
                .build()
                .unwrap_or_else(|_| Client::new()),
            config,
            cache: HashMap::new(),
        }
    }
    
    /// Create weather service with default configuration
    pub fn with_default_config() -> Self {
        Self::new(WeatherApiConfig::default())
    }
    
    /// Get the current weather service configuration
    #[allow(dead_code)]
    pub fn get_config(&self) -> &WeatherApiConfig {
        &self.config
    }

    /// Get weather conditions for a specific time and location
    pub async fn get_weather_at_time(
        &mut self,
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<WeatherConditions, WeatherError> {
        // Check if weather API is enabled
        if !self.config.enabled {
            return Ok(self.generate_mock_weather());
        }
        
        // Create cache key
        let cache_key = format!("{}_{}_{}_{}", 
            latitude, longitude, 
            timestamp.timestamp() / 3600, // Hour-based caching
            self.config.provider
        );
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            if cached.cached_at.elapsed().unwrap_or(Duration::MAX) 
                < Duration::from_secs(self.config.cache_duration_hours * 3600) {
                return Ok(cached.weather.clone());
            }
        }
        
        // Try to fetch from API
        match self.fetch_weather_from_api(latitude, longitude, timestamp).await {
            Ok(weather) => {
                // Cache the result
                self.cache.insert(cache_key, CachedWeatherData {
                    weather: weather.clone(),
                    cached_at: SystemTime::now(),
                });
                Ok(weather)
            }
            Err(e) => {
                // Fall back to mock data if configured
                if self.config.fallback_to_mock {
                    tracing::warn!("Weather API failed, falling back to mock data: {}", e);
                    Ok(self.generate_mock_weather())
                } else {
                    Err(e)
                }
            }
        }
    }
    
    /// Fetch weather data from the configured API
    async fn fetch_weather_from_api(
        &self,
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<WeatherConditions, WeatherError> {
        match self.config.provider.as_str() {
            "openweathermap" => self.fetch_from_openweather(latitude, longitude, timestamp).await,
            _ => Err(WeatherError::ApiError(format!("Unsupported weather provider: {}", self.config.provider))),
        }
    }
    
    /// Fetch weather from OpenWeatherMap Historical API
    async fn fetch_from_openweather(
        &self,
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<WeatherConditions, WeatherError> {
        let api_key = std::env::var("OPENWEATHER_API_KEY")
            .map_err(|_| WeatherError::ApiError("OPENWEATHER_API_KEY environment variable not set".to_string()))?;
        
        let url = format!(
            "https://api.openweathermap.org/data/3.0/onecall/timemachine?lat={}&lon={}&dt={}&appid={}&units=metric",
            latitude, longitude, timestamp.timestamp(), api_key
        );
        
        tracing::debug!("Fetching weather from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(WeatherError::ApiError(format!(
                "OpenWeather API returned status {}: {}", 
                status,
                error_text
            )));
        }
        
        let weather_response: OpenWeatherResponse = response.json().await?;
        
        // Find the closest data point to our timestamp
        let target_timestamp = timestamp.timestamp();
        let closest_data = weather_response.data
            .into_iter()
            .min_by_key(|data| (data.dt - target_timestamp).abs())
            .ok_or_else(|| WeatherError::DataUnavailable)?;
        
        // Convert to our format
        let conditions = if let Some(weather) = closest_data.weather.first() {
            weather.main.clone()
        } else {
            "clear".to_string()
        };
        
        Ok(WeatherConditions {
            temperature_celsius: closest_data.temp as f32,
            humidity_percentage: closest_data.humidity.map(|h| h as f32),
            wind_speed_kmh: closest_data.wind_speed.map(|ws| (ws * 3.6) as f32), // Convert m/s to km/h
            conditions,
        })
    }

    /// Get weather conditions for an activity's start location and time
    pub async fn get_weather_for_activity(
        &mut self,
        start_latitude: Option<f64>,
        start_longitude: Option<f64>,
        start_time: DateTime<Utc>,
    ) -> Result<Option<WeatherConditions>, WeatherError> {
        if let (Some(lat), Some(lon)) = (start_latitude, start_longitude) {
            let weather = self.get_weather_at_time(lat, lon, start_time).await?;
            Ok(Some(weather))
        } else {
            Ok(None)
        }
    }

    /// Generate mock weather data for testing
    pub fn generate_mock_weather(&self) -> WeatherConditions {
        // Generate realistic weather based on current time
        let now = Utc::now();
        let hour = now.hour();
        
        // Simple seasonal/time-based mock data with varied conditions
        let (base_temp, conditions) = match now.month() {
            12 | 1 | 2 => (2.0, if now.day() % 4 == 0 { "snow" } else { "cloudy" }),      // Winter
            3 | 4 | 5 => (12.0, if now.day() % 3 == 0 { "rainy" } else { "partly cloudy" }),      // Spring
            6 | 7 | 8 => (22.0, if now.day() % 5 == 0 { "thunderstorms" } else { "sunny" }),      // Summer
            _ => (8.0, if now.day() % 3 == 0 { "rainy" } else { "overcast" }),               // Fall
        };

        // Add some variation based on time of day
        let temp_adjustment = match hour {
            6..=11 => -2.0,   // Cooler morning
            12..=17 => 3.0,   // Warmer afternoon
            18..=21 => 0.0,   // Moderate evening
            _ => -5.0,        // Cooler night
        };

        let temperature = base_temp + temp_adjustment;
        
        // Use the conditions from seasonal data
        let conditions_str = conditions;

        WeatherConditions {
            temperature_celsius: temperature,
            humidity_percentage: Some(60.0 + (now.timestamp() % 30) as f32),
            wind_speed_kmh: Some(5.0 + (now.timestamp() % 15) as f32),
            conditions: conditions_str.to_string(),
        }
    }

    /// Analyze weather impact on performance
    #[allow(dead_code)]
    pub fn analyze_weather_impact(&self, weather: &WeatherConditions) -> WeatherImpact {
        let mut impact_factors = Vec::new();
        let mut overall_difficulty = 0.0;

        // Temperature impact
        match weather.temperature_celsius {
            t if t < -5.0 => {
                impact_factors.push("Extremely cold conditions increase energy expenditure".to_string());
                overall_difficulty += 3.0;
            }
            t if t < 0.0 => {
                impact_factors.push("Cold conditions may affect performance".to_string());
                overall_difficulty += 2.0;
            }
            t if t > 30.0 => {
                impact_factors.push("Hot conditions increase heat stress".to_string());
                overall_difficulty += 2.5;
            }
            t if t > 25.0 => {
                impact_factors.push("Warm conditions may increase perceived effort".to_string());
                overall_difficulty += 1.0;
            }
            _ => {
                impact_factors.push("Ideal temperature conditions".to_string());
            }
        }

        // Wind impact
        if let Some(wind_speed) = weather.wind_speed_kmh {
            match wind_speed {
                w if w > 30.0 => {
                    impact_factors.push("Strong winds significantly impact performance".to_string());
                    overall_difficulty += 2.0;
                }
                w if w > 15.0 => {
                    impact_factors.push("Moderate winds may affect pace".to_string());
                    overall_difficulty += 1.0;
                }
                _ => {}
            }
        }

        // Precipitation impact
        if weather.conditions.contains("rain") {
            impact_factors.push("Wet conditions require extra caution and mental focus".to_string());
            overall_difficulty += 1.5;
        } else if weather.conditions.contains("snow") {
            impact_factors.push("Snow conditions significantly increase difficulty".to_string());
            overall_difficulty += 2.5;
        }

        // Humidity impact
        if let Some(humidity) = weather.humidity_percentage {
            if humidity > 80.0 && weather.temperature_celsius > 20.0 {
                impact_factors.push("High humidity makes cooling less efficient".to_string());
                overall_difficulty += 1.0;
            }
        }

        let difficulty_level = match overall_difficulty {
            d if d < 1.0 => WeatherDifficulty::Ideal,
            d if d < 2.5 => WeatherDifficulty::Challenging,
            d if d < 5.0 => WeatherDifficulty::Difficult,
            _ => WeatherDifficulty::Extreme,
        };

        WeatherImpact {
            difficulty_level,
            impact_factors,
            performance_adjustment: -overall_difficulty * 2.0, // Negative adjustment for difficulty
        }
    }
}

impl Default for WeatherService {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Weather impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherImpact {
    pub difficulty_level: WeatherDifficulty,
    pub impact_factors: Vec<String>,
    pub performance_adjustment: f32, // Percentage adjustment to expected performance
}

/// Weather difficulty classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherDifficulty {
    Ideal,
    Challenging,
    Difficult,
    Extreme,
}

/// Weather service errors
#[derive(Debug, thiserror::Error)]
pub enum WeatherError {
    #[error("Weather API request failed: {0}")]
    #[allow(dead_code)]
    ApiError(String),
    
    #[error("Invalid coordinates: lat={lat}, lon={lon}")]
    #[allow(dead_code)]
    InvalidCoordinates { lat: f64, lon: f64 },
    
    #[error("Weather data unavailable for requested time")]
    #[allow(dead_code)]
    DataUnavailable,
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_weather_service_creation() {
        let config = crate::config::fitness_config::WeatherApiConfig::default();
        let _service = WeatherService::new(config);
        assert!(true); // Just test creation
    }

    #[test]
    fn test_generate_mock_weather() {
        let config = crate::config::fitness_config::WeatherApiConfig::default();
        let service = WeatherService::new(config);
        let weather = service.generate_mock_weather();
        
        assert!(weather.temperature_celsius > -20.0 && weather.temperature_celsius < 40.0);
        assert!(weather.humidity_percentage.is_some());
        assert!(weather.wind_speed_kmh.is_some());
        assert!(!weather.conditions.is_empty());
    }

    #[test]
    fn test_analyze_weather_impact_cold() {
        let config = crate::config::fitness_config::WeatherApiConfig::default();
        let service = WeatherService::new(config);
        let cold_weather = WeatherConditions {
            temperature_celsius: -10.0,
            humidity_percentage: Some(50.0),
            wind_speed_kmh: Some(10.0),
            conditions: "snow".to_string(),
        };
        
        let impact = service.analyze_weather_impact(&cold_weather);
        assert!(matches!(impact.difficulty_level, WeatherDifficulty::Difficult | WeatherDifficulty::Extreme));
        assert!(!impact.impact_factors.is_empty());
        assert!(impact.performance_adjustment < 0.0);
    }

    #[test]
    fn test_analyze_weather_impact_ideal() {
        let config = crate::config::fitness_config::WeatherApiConfig::default();
        let service = WeatherService::new(config);
        let ideal_weather = WeatherConditions {
            temperature_celsius: 15.0,
            humidity_percentage: Some(50.0),
            wind_speed_kmh: Some(5.0),
            conditions: "sunny".to_string(),
        };
        
        let impact = service.analyze_weather_impact(&ideal_weather);
        assert!(matches!(impact.difficulty_level, WeatherDifficulty::Ideal));
    }

    #[test]
    fn test_analyze_weather_impact_hot_humid() {
        let config = crate::config::fitness_config::WeatherApiConfig::default();
        let service = WeatherService::new(config);
        let hot_humid_weather = WeatherConditions {
            temperature_celsius: 32.0,
            humidity_percentage: Some(85.0),
            wind_speed_kmh: Some(2.0),
            conditions: "sunny".to_string(),
        };
        
        let impact = service.analyze_weather_impact(&hot_humid_weather);
        assert!(matches!(impact.difficulty_level, WeatherDifficulty::Challenging | WeatherDifficulty::Difficult));
        assert!(impact.performance_adjustment < 0.0);
    }

    #[tokio::test]
    async fn test_get_weather_at_time() {
        let mut service = WeatherService::with_default_config();
        let result = service.get_weather_at_time(45.5017, -73.5673, Utc::now()).await; // Montreal coords
        
        assert!(result.is_ok());
        let weather = result.unwrap();
        assert!(weather.temperature_celsius > -50.0 && weather.temperature_celsius < 50.0);
    }

    #[tokio::test]
    async fn test_get_weather_for_activity_with_coords() {
        let mut service = WeatherService::with_default_config();
        let result = service.get_weather_for_activity(
            Some(45.5017), 
            Some(-73.5673), 
            Utc::now()
        ).await;
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_weather_for_activity_without_coords() {
        let mut service = WeatherService::with_default_config();
        let result = service.get_weather_for_activity(None, None, Utc::now()).await;
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}