// ABOUTME: Unit tests for intelligence weather functionality
// ABOUTME: Validates intelligence weather behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::config::fitness_config::WeatherApiConfig;
use pierre_mcp_server::intelligence::weather::{WeatherDifficulty, WeatherService};
use pierre_mcp_server::intelligence::WeatherConditions;

#[test]
fn test_weather_service_creation() {
    let config = WeatherApiConfig::default();
    let service = WeatherService::new(config, None);
    // Verify service is created with correct configuration
    assert!(service.get_config().enabled);
}

#[test]
fn test_analyze_weather_impact_cold() {
    let config = WeatherApiConfig::default();
    let service = WeatherService::new(config, None);
    let cold_weather = WeatherConditions {
        temperature_celsius: -10.0,
        humidity_percentage: Some(50.0),
        wind_speed_kmh: Some(10.0),
        conditions: "snow".into(),
    };

    let impact = service.analyze_weather_impact(&cold_weather);
    assert!(matches!(
        impact.difficulty_level,
        WeatherDifficulty::Difficult | WeatherDifficulty::Extreme
    ));
    assert!(!impact.impact_factors.is_empty());
    assert!(impact.performance_adjustment < 0.0);
}

#[test]
fn test_analyze_weather_impact_ideal() {
    let config = WeatherApiConfig::default();
    let service = WeatherService::new(config, None);
    let ideal_weather = WeatherConditions {
        temperature_celsius: 15.0,
        humidity_percentage: Some(50.0),
        wind_speed_kmh: Some(5.0),
        conditions: "sunny".into(),
    };

    let impact = service.analyze_weather_impact(&ideal_weather);
    assert!(matches!(impact.difficulty_level, WeatherDifficulty::Ideal));
}

#[test]
fn test_analyze_weather_impact_hot_humid() {
    let config = WeatherApiConfig::default();
    let service = WeatherService::new(config, None);
    let hot_humid_weather = WeatherConditions {
        temperature_celsius: 32.0,
        humidity_percentage: Some(85.0),
        wind_speed_kmh: Some(2.0),
        conditions: "sunny".into(),
    };

    let impact = service.analyze_weather_impact(&hot_humid_weather);
    assert!(matches!(
        impact.difficulty_level,
        WeatherDifficulty::Challenging | WeatherDifficulty::Difficult
    ));
    assert!(impact.performance_adjustment < 0.0);
}

#[tokio::test]
async fn test_get_weather_at_time_disabled() {
    let config = WeatherApiConfig {
        enabled: false,
        ..Default::default()
    };
    let mut service = WeatherService::new(config, None);
    let result = service
        .get_weather_at_time(45.5017, -73.5673, Utc::now())
        .await; // Montreal coords

    assert!(result.is_err());
}
