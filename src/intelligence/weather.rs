// ABOUTME: Weather data integration and environmental impact analysis for fitness activities
// ABOUTME: Provides weather context, environmental adjustments, and performance correlations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Weather service integration for contextual activity analysis

use super::WeatherConditions;
use crate::config::fitness_config::WeatherApiConfig;
use crate::config::intelligence_config::{IntelligenceConfig, WeatherAnalysisConfig};
use crate::intelligence::physiological_constants::{
    unit_conversions::MS_TO_KMH_FACTOR,
    weather_impact_factors::{
        COLD_DIFFICULTY, EXTREME_COLD_DIFFICULTY, EXTREME_HOT_DIFFICULTY, HIGH_HUMIDITY_DIFFICULTY,
        MODERATE_WIND_DIFFICULTY, RAIN_DIFFICULTY, SNOW_DIFFICULTY, STRONG_WIND_DIFFICULTY,
        WARM_DIFFICULTY,
    },
    weather_thresholds::{
        COLD_THRESHOLD_CELSIUS, EXTREME_COLD_CELSIUS, EXTREME_HOT_THRESHOLD_CELSIUS,
        HIGH_HUMIDITY_THRESHOLD, HOT_THRESHOLD_CELSIUS, HUMIDITY_IMPACT_TEMP_THRESHOLD,
        MODERATE_WIND_THRESHOLD, STRONG_WIND_THRESHOLD,
    },
};
use crate::utils::http_client::create_client_with_timeout;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Safe casting helper functions to avoid clippy warnings
#[inline]
#[allow(clippy::cast_possible_truncation)] // Safe: clamped to f32 range
fn safe_f64_to_f32(value: f64) -> f32 {
    use std::cmp::Ordering;

    // Handle special cases
    if value.is_nan() {
        return 0.0_f32;
    }

    // Use total_cmp for proper comparison without casting warnings
    if value.total_cmp(&f64::from(f32::MAX)) == Ordering::Greater {
        f32::MAX
    } else if value.total_cmp(&f64::from(f32::MIN)) == Ordering::Less {
        f32::MIN
    } else {
        // Value is within f32 range, use rounding conversion
        let rounded = value.round();
        if rounded > f64::from(f32::MAX) {
            f32::MAX
        } else if rounded < f64::from(f32::MIN) {
            f32::MIN
        } else {
            // Safe conversion using IEEE 754 standard rounding
            rounded as f32
        }
    }
}

/// Weather service for fetching historical weather data
pub struct WeatherService {
    /// HTTP client for weather API requests
    client: Client,
    /// Weather API configuration
    api_config: WeatherApiConfig,
    /// Weather analysis configuration
    weather_config: WeatherAnalysisConfig,
    /// In-memory cache of weather data
    cache: HashMap<String, CachedWeatherData>,
    /// Optional API key for weather service
    api_key: Option<String>,
}

/// Cached weather data with timestamp
#[derive(Debug, Clone)]
struct CachedWeatherData {
    /// Weather conditions data
    weather: WeatherConditions,
    /// When this data was cached
    cached_at: SystemTime,
}

/// `OpenWeatherMap` historical API response structure
#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    /// Array of hourly weather data points
    data: Vec<OpenWeatherHourlyData>,
}

/// Hourly weather data from `OpenWeatherMap` API
#[derive(Debug, Deserialize)]
struct OpenWeatherHourlyData {
    /// Unix timestamp for this data point
    dt: i64,
    /// Temperature in Celsius
    temp: f64,
    /// Humidity percentage (0-100)
    humidity: Option<f64>,
    /// Wind speed in meters per second
    wind_speed: Option<f64>,
    /// Weather condition descriptions
    weather: Vec<OpenWeatherCondition>,
}

/// Weather condition description from `OpenWeatherMap`
#[derive(Debug, Deserialize)]
struct OpenWeatherCondition {
    /// Main weather category (e.g., "Rain", "Clear")
    main: String,
    /// Detailed description (e.g., "light rain")
    description: String,
}

impl WeatherService {
    /// Create a new weather service with configuration and API key
    #[must_use]
    pub fn new(api_config: WeatherApiConfig, api_key: Option<String>) -> Self {
        let intelligence_config = IntelligenceConfig::global();
        Self {
            client: create_client_with_timeout(api_config.request_timeout_seconds, 10),
            api_config,
            weather_config: intelligence_config.weather_analysis.clone(),
            cache: HashMap::new(),
            api_key,
        }
    }

    /// Create weather service with default configuration
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(
            WeatherApiConfig::default(),
            crate::constants::get_server_config()
                .and_then(|c| c.external_services.weather.api_key.clone()),
        )
    }

    /// Create weather service with custom weather analysis configuration
    #[must_use]
    pub fn with_weather_config(
        api_config: WeatherApiConfig,
        weather_config: WeatherAnalysisConfig,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client: create_client_with_timeout(api_config.request_timeout_seconds, 10),
            api_config,
            weather_config,
            cache: HashMap::new(),
            api_key,
        }
    }

    /// Get the current weather service configuration
    #[must_use]
    pub const fn get_config(&self) -> &WeatherApiConfig {
        &self.api_config
    }

    /// Get the weather analysis configuration
    #[must_use]
    pub const fn get_weather_config(&self) -> &WeatherAnalysisConfig {
        &self.weather_config
    }

    /// Get weather conditions for a specific time and location
    ///
    /// # Errors
    ///
    /// Returns an error if the weather API is disabled, the API request fails,
    /// or the response cannot be parsed
    pub async fn get_weather_at_time(
        &mut self,
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<WeatherConditions, WeatherError> {
        // Check if weather API is enabled
        if !self.api_config.enabled {
            return Err(WeatherError::ApiDisabled);
        }

        // Create cache key
        let cache_key = format!(
            "{}_{}_{}_{}",
            latitude,
            longitude,
            timestamp.timestamp() / 3600, // Hour-based caching
            self.api_config.provider
        );

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            if cached.cached_at.elapsed().unwrap_or(Duration::MAX)
                < Duration::from_secs(self.api_config.cache_duration_hours * 3600)
            {
                return Ok(cached.weather.clone());
            }
        }

        // Try to fetch from API
        match self
            .fetch_weather_from_api(latitude, longitude, timestamp)
            .await
        {
            Ok(weather) => {
                // Cache the result
                self.cache.insert(
                    cache_key,
                    CachedWeatherData {
                        weather: weather.clone(),
                        cached_at: SystemTime::now(),
                    },
                );
                Ok(weather)
            }
            Err(e) => {
                tracing::error!("Weather API request failed: {}", e);
                Err(e)
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
        match self.api_config.provider.as_str() {
            "openweathermap" => {
                self.fetch_from_openweather(latitude, longitude, timestamp)
                    .await
            }
            _ => Err(WeatherError::ApiError(format!(
                "Unsupported weather provider: {}",
                self.api_config.provider
            ))),
        }
    }

    /// Fetch weather from `OpenWeatherMap` Historical API
    async fn fetch_from_openweather(
        &self,
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<WeatherConditions, WeatherError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| WeatherError::ApiError("OpenWeather API key not configured".into()))?;

        let url = format!(
            "{}/data/3.0/onecall/timemachine?lat={}&lon={}&dt={}&appid={}&units=metric",
            &self.api_config.base_url,
            latitude,
            longitude,
            timestamp.timestamp(),
            api_key
        );

        tracing::debug!("Fetching weather from: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".into());
            return Err(WeatherError::ApiError(format!(
                "OpenWeather API returned status {status}: {error_text}"
            )));
        }

        let weather_response: OpenWeatherResponse = response.json().await?;

        // Find the closest data point to our timestamp
        let target_timestamp = timestamp.timestamp();
        let closest_data = weather_response
            .data
            .into_iter()
            .min_by_key(|data| (data.dt - target_timestamp).abs())
            .ok_or_else(|| WeatherError::DataUnavailable)?;

        // Convert to our format - use both main and description for detailed conditions
        let conditions = closest_data.weather.first().map_or_else(
            || "clear".into(),
            |weather| {
                // Combine main weather type with detailed description
                if weather.description.to_lowercase() == weather.main.to_lowercase() {
                    weather.main.clone()
                } else {
                    format!("{} - {}", weather.main, weather.description)
                }
            },
        );
        Ok(WeatherConditions {
            temperature_celsius: safe_f64_to_f32(closest_data.temp),
            humidity_percentage: closest_data.humidity.map(safe_f64_to_f32),
            wind_speed_kmh: closest_data
                .wind_speed
                .map(|ws| safe_f64_to_f32(ws * MS_TO_KMH_FACTOR)), // Convert m/s to km/h
            conditions,
        })
    }

    /// Get weather conditions for an activity's start location and time
    ///
    /// # Errors
    ///
    /// Returns an error if weather API calls fail or location data is invalid
    pub async fn get_weather_for_activity(
        &mut self,
        start_latitude: Option<f64>,
        start_longitude: Option<f64>,
        start_time: DateTime<Utc>,
    ) -> Result<Option<WeatherConditions>, WeatherError> {
        if let (Some(lat), Some(lon)) = (start_latitude, start_longitude) {
            match self.get_weather_at_time(lat, lon, start_time).await {
                Ok(weather) => Ok(Some(weather)),
                Err(WeatherError::ApiDisabled) => Ok(None), // Gracefully handle disabled API
                Err(e) => Err(e),
            }
        } else {
            Ok(None)
        }
    }

    /// Analyze weather impact on performance
    #[must_use]
    pub fn analyze_weather_impact(&self, weather: &WeatherConditions) -> WeatherImpact {
        let mut impact_factors = Vec::new();
        let mut overall_difficulty = 0.0;

        // Temperature impact using physiological constants
        match weather.temperature_celsius {
            t if t < EXTREME_COLD_CELSIUS => {
                impact_factors.push("Extremely cold conditions increase energy expenditure".into());
                overall_difficulty += EXTREME_COLD_DIFFICULTY;
            }
            t if t < COLD_THRESHOLD_CELSIUS => {
                impact_factors.push("Cold conditions may affect performance".into());
                overall_difficulty += COLD_DIFFICULTY;
            }
            t if t > EXTREME_HOT_THRESHOLD_CELSIUS => {
                impact_factors.push("Hot conditions increase heat stress".into());
                overall_difficulty += EXTREME_HOT_DIFFICULTY;
            }
            t if t > HOT_THRESHOLD_CELSIUS => {
                impact_factors.push("Warm conditions may increase perceived effort".into());
                overall_difficulty += WARM_DIFFICULTY;
            }
            _ => {
                impact_factors.push("Ideal temperature conditions".into());
            }
        }

        // Wind impact using established thresholds
        if let Some(wind_speed) = weather.wind_speed_kmh {
            match wind_speed {
                w if w > STRONG_WIND_THRESHOLD => {
                    impact_factors.push("Strong winds significantly impact performance".into());
                    overall_difficulty += STRONG_WIND_DIFFICULTY;
                }
                w if w > MODERATE_WIND_THRESHOLD => {
                    impact_factors.push("Moderate winds may affect pace".into());
                    overall_difficulty += MODERATE_WIND_DIFFICULTY;
                }
                _ => {}
            }
        }

        // Precipitation impact
        if weather.conditions.contains("rain") {
            impact_factors.push("Wet conditions require extra caution and mental focus".into());
            overall_difficulty += RAIN_DIFFICULTY;
        } else if weather.conditions.contains("snow") {
            impact_factors.push("Snow conditions significantly increase difficulty".into());
            overall_difficulty += SNOW_DIFFICULTY;
        }

        // Humidity impact using physiological thresholds
        if let Some(humidity) = weather.humidity_percentage {
            if humidity > HIGH_HUMIDITY_THRESHOLD
                && weather.temperature_celsius > HUMIDITY_IMPACT_TEMP_THRESHOLD
            {
                impact_factors.push("High humidity makes cooling less efficient".into());
                overall_difficulty += HIGH_HUMIDITY_DIFFICULTY;
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
            performance_adjustment: safe_f64_to_f32(-overall_difficulty * 2.0), // Negative adjustment for difficulty
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
    /// Classified difficulty level based on weather conditions
    pub difficulty_level: WeatherDifficulty,
    /// List of specific factors affecting performance
    pub impact_factors: Vec<String>,
    /// Percentage adjustment to expected performance (negative = slower)
    pub performance_adjustment: f32,
}

/// Weather difficulty classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherDifficulty {
    /// Perfect conditions for optimal performance
    Ideal,
    /// Moderate challenges requiring minor adjustments
    Challenging,
    /// Significant obstacles affecting performance
    Difficult,
    /// Dangerous or extreme conditions
    Extreme,
}

/// Weather service errors
#[derive(Debug, thiserror::Error)]
pub enum WeatherError {
    /// Weather API request failed
    #[error("Weather API request failed: {0}")]
    ApiError(String),

    /// Invalid coordinate values provided
    #[error("Invalid coordinates: lat={lat}, lon={lon}")]
    InvalidCoordinates {
        /// Latitude value
        lat: f64,
        /// Longitude value
        lon: f64,
    },

    /// Weather data not available for the requested time
    #[error("Weather data unavailable for requested time")]
    DataUnavailable,

    /// Weather API is disabled in configuration
    #[error("Weather API is disabled")]
    ApiDisabled,

    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
