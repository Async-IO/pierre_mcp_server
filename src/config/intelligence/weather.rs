// ABOUTME: Weather analysis configuration for outdoor activity impact assessment
// ABOUTME: Configures temperature thresholds, weather condition impacts, and scoring weights
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Weather Analysis Configuration
//!
//! Provides configuration for weather impact analysis on outdoor activities
//! including temperature thresholds and condition-based performance adjustments.

use serde::{Deserialize, Serialize};

/// Weather Analysis Configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeatherAnalysisConfig {
    /// Temperature thresholds and ranges
    pub temperature: TemperatureConfig,
    /// Weather condition thresholds (humidity, wind, precipitation)
    pub conditions: WeatherConditionsConfig,
    /// Weights for combining different weather factors
    pub impact: WeatherImpactConfig,
}

/// Temperature thresholds for weather analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureConfig {
    /// Minimum ideal temperature in Celsius
    pub ideal_min_celsius: f32,
    /// Maximum ideal temperature in Celsius
    pub ideal_max_celsius: f32,
    /// Cold threshold temperature in Celsius
    pub cold_threshold_celsius: f32,
    /// Hot threshold temperature in Celsius
    pub hot_threshold_celsius: f32,
    /// Extreme cold threshold in Celsius
    pub extreme_cold_celsius: f32,
    /// Extreme hot threshold in Celsius
    pub extreme_hot_celsius: f32,
}

/// Weather condition thresholds for activity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditionsConfig {
    /// High humidity threshold (0.0-100.0)
    pub high_humidity_threshold: f64,
    /// Strong wind speed threshold in m/s
    pub strong_wind_threshold: f64,
    /// Impact factor for precipitation on performance
    pub precipitation_impact_factor: f64,
}

/// Weights for combining weather impacts on performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherImpactConfig {
    /// Weight for temperature impact (0.0-1.0)
    pub temperature_impact_weight: f64,
    /// Weight for humidity impact (0.0-1.0)
    pub humidity_impact_weight: f64,
    /// Weight for wind impact (0.0-1.0)
    pub wind_impact_weight: f64,
    /// Weight for precipitation impact (0.0-1.0)
    pub precipitation_impact_weight: f64,
}

impl Default for TemperatureConfig {
    fn default() -> Self {
        Self {
            ideal_min_celsius: 10.0,
            ideal_max_celsius: 20.0,
            cold_threshold_celsius: 5.0,
            hot_threshold_celsius: 25.0,
            extreme_cold_celsius: -5.0,
            extreme_hot_celsius: 35.0,
        }
    }
}

impl Default for WeatherConditionsConfig {
    fn default() -> Self {
        Self {
            high_humidity_threshold: 80.0,
            strong_wind_threshold: 20.0,
            precipitation_impact_factor: 0.8,
        }
    }
}

impl Default for WeatherImpactConfig {
    fn default() -> Self {
        Self {
            temperature_impact_weight: 0.4,
            humidity_impact_weight: 0.3,
            wind_impact_weight: 0.2,
            precipitation_impact_weight: 0.1,
        }
    }
}
