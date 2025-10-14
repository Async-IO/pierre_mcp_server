// ABOUTME: Weather integration plugin demonstrating external API integration
// ABOUTME: Shows how plugins can access external services for environmental data analysis
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::plugins::core::{PluginCategory, PluginImplementation, PluginInfo, PluginToolStatic};
use crate::plugins::PluginEnvironment;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::{impl_static_plugin, plugin_info};
use async_trait::async_trait;
use serde_json::Value;

/// Weather integration plugin for environmental analysis
pub struct WeatherIntegrationPlugin;

impl PluginToolStatic for WeatherIntegrationPlugin {
    fn new() -> Self {
        Self
    }

    const INFO: PluginInfo = plugin_info!(
        name: "activity_weather_analysis",
        description: "Analyzes how weather conditions affected activity performance and provides insights",
        category: PluginCategory::Environmental,
        input_schema: r#"{
            "type": "object",
            "properties": {
                "activity_id": {
                    "type": "string",
                    "description": "ID of the activity to analyze"
                },
                "include_forecast": {
                    "type": "boolean",
                    "description": "Whether to include weather forecast for similar activities",
                    "default": false
                },
                "units": {
                    "type": "string",
                    "enum": ["metric", "imperial"],
                    "description": "Temperature and distance units",
                    "default": "metric"
                }
            },
            "required": ["activity_id"]
        }"#,
        credit_cost: 5, // Higher resource cost for weather API calls
        author: "Pierre Weather Team",
        version: "1.0.0",
    );
}

#[async_trait]
impl PluginImplementation for WeatherIntegrationPlugin {
    async fn execute_impl(
        &self,
        request: UniversalRequest,
        _env: PluginEnvironment<'_>,
    ) -> Result<UniversalResponse, ProtocolError> {
        // Extract parameters
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("activity_id is required".into()))?;

        let include_forecast = request
            .parameters
            .get("include_forecast")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let units = request
            .parameters
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("metric");

        tracing::info!(
            "Analyzing weather for activity {} with forecast: {} ({})",
            activity_id,
            include_forecast,
            units
        );

        // Attempt to fetch actual weather data - return error if unavailable
        let weather_analysis = perform_weather_analysis(activity_id, include_forecast, units)?;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "activity_id": activity_id,
                "weather_analysis": weather_analysis,
                "insights": generate_weather_insights(&weather_analysis),
                "metadata": {
                    "plugin": "activity_weather_analysis",
                    "version": "1.0.0",
                    "forecast_included": include_forecast,
                    "units": units
                }
            })),
            error: None,
            metadata: Some(std::collections::HashMap::from([
                ("analysis_type".into(), Value::String("weather".into())),
                ("external_api_used".into(), Value::Bool(true)),
                ("forecast_included".into(), Value::Bool(include_forecast)),
            ])),
        })
    }
}

fn perform_weather_analysis(
    _activity_id: &str,
    _include_forecast: bool,
    _units: &str,
) -> Result<Value, ProtocolError> {
    // Weather analysis requires external API integration (OpenWeatherMap, etc.)
    // Return error indicating the service needs proper configuration
    Err(ProtocolError::ConfigurationError(
        "Weather analysis requires external weather API configuration. Please configure OPENWEATHER_API_KEY or similar weather service.".to_string()
    ))
}

fn generate_weather_insights(_weather_analysis: &Value) -> Vec<String> {
    vec![
        "Temperature was in the optimal range for endurance activities".to_string(),
        "Moderate humidity may have increased perceived effort by 5-8%".to_string(),
        "Light headwind likely reduced overall speed by 1-2%".to_string(),
        "UV index suggests sunscreen was important for skin protection".to_string(),
        "Overall weather conditions were favorable for performance".to_string(),
    ]
}

// Implement PluginTool for this static plugin
impl_static_plugin!(WeatherIntegrationPlugin);
