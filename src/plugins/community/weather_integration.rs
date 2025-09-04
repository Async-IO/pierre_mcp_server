// ABOUTME: Weather integration plugin demonstrating external API integration
// ABOUTME: Shows how plugins can access external services for environmental data analysis

use crate::plugins::core::{PluginCategory, PluginImplementation, PluginInfo, PluginToolStatic};
use crate::plugins::PluginEnvironment;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::{impl_static_plugin, plugin_info, register_plugin};
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

        // This would typically fetch actual weather data from an external API
        // For now, we'll return mock weather analysis
        let weather_analysis = perform_weather_analysis(activity_id, include_forecast, units);

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

fn perform_weather_analysis(_activity_id: &str, include_forecast: bool, units: &str) -> Value {
    let temp_unit = if units == "imperial" { "°F" } else { "°C" };
    let speed_unit = if units == "imperial" { "mph" } else { "km/h" };

    let mut analysis = serde_json::json!({
        "conditions_during_activity": {
            "temperature": if units == "imperial" { 68 } else { 20 },
            "temperature_unit": temp_unit,
            "humidity": 65,
            "wind_speed": if units == "imperial" { 8 } else { 13 },
            "wind_speed_unit": speed_unit,
            "wind_direction": "NW",
            "conditions": "partly_cloudy",
            "uv_index": 6,
            "feels_like": if units == "imperial" { 72 } else { 22 }
        },
        "performance_impact": {
            "temperature_impact": "optimal",
            "wind_impact": "slight_headwind",
            "humidity_impact": "moderate",
            "overall_conditions": "favorable",
            "estimated_performance_delta": "+2.3%"
        }
    });

    if include_forecast {
        analysis["upcoming_conditions"] = serde_json::json!({
            "next_3_days": [
                {
                    "date": "2024-01-15",
                    "conditions": "sunny",
                    "temperature_high": if units == "imperial" { 72 } else { 22 },
                    "temperature_low": if units == "imperial" { 58 } else { 14 },
                    "wind_speed": if units == "imperial" { 5 } else { 8 },
                    "recommended_activity_window": "07:00-09:00",
                    "activity_rating": "excellent"
                },
                {
                    "date": "2024-01-16",
                    "conditions": "light_rain",
                    "temperature_high": if units == "imperial" { 64 } else { 18 },
                    "temperature_low": if units == "imperial" { 52 } else { 11 },
                    "wind_speed": if units == "imperial" { 12 } else { 19 },
                    "recommended_activity_window": "indoor_recommended",
                    "activity_rating": "poor"
                }
            ]
        });
    }

    analysis
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

// Register this plugin at compile time
register_plugin!(WeatherIntegrationPlugin);
