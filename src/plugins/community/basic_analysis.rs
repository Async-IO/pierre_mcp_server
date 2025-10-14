// ABOUTME: Basic fitness analysis plugin demonstrating community tool development
// ABOUTME: Provides simple metrics calculation and analysis for activities
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

/// Basic analysis plugin for community use
pub struct BasicAnalysisPlugin;

impl PluginToolStatic for BasicAnalysisPlugin {
    fn new() -> Self {
        Self
    }

    const INFO: PluginInfo = plugin_info!(
        name: "basic_activity_analysis",
        description: "Performs basic analysis on fitness activities including pace, speed, and efficiency calculations",
        category: PluginCategory::Community,
        input_schema: r#"{
            "type": "object",
            "properties": {
                "activity_id": {
                    "type": "string",
                    "description": "ID of the activity to analyze"
                },
                "include_zones": {
                    "type": "boolean", 
                    "description": "Whether to include heart rate and power zones",
                    "default": false
                }
            },
            "required": ["activity_id"]
        }"#,
        credit_cost: 1,
        author: "Pierre Community",
        version: "1.0.0",
    );
}

#[async_trait]
impl PluginImplementation for BasicAnalysisPlugin {
    async fn execute_impl(
        &self,
        request: UniversalRequest,
        env: PluginEnvironment<'_>,
    ) -> Result<UniversalResponse, ProtocolError> {
        // Extract parameters
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("activity_id is required".into()))?;

        let include_zones = request
            .parameters
            .get("include_zones")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        tracing::info!(
            "Analyzing activity {} with zones: {}",
            activity_id,
            include_zones
        );

        // Verify Strava provider is available in the registry
        let provider = env
            .provider_registry
            .create_provider("strava")
            .map_err(|e| {
                ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
            })?;

        // Attempt to fetch and analyze real activity data
        let analysis_result =
            perform_basic_analysis(activity_id, include_zones, provider.as_ref()).await?;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "activity_id": activity_id,
                "analysis": analysis_result,
                "metadata": {
                    "plugin": "basic_activity_analysis",
                    "version": "1.0.0",
                    "zones_included": include_zones
                }
            })),
            error: None,
            metadata: Some(std::collections::HashMap::from([
                ("analysis_type".into(), Value::String("basic".into())),
                ("zones_included".into(), Value::Bool(include_zones)),
            ])),
        })
    }
}

#[allow(clippy::cast_precision_loss)]
async fn perform_basic_analysis(
    activity_id: &str,
    include_zones: bool,
    provider: &dyn crate::providers::core::FitnessProvider,
) -> Result<Value, ProtocolError> {
    // Fetch actual activity data from the provider
    let activity = provider.get_activity(activity_id).await.map_err(|e| {
        ProtocolError::ExecutionFailed(format!("Failed to fetch activity {activity_id}: {e}"))
    })?;

    // Calculate real pace metrics if distance and duration are available
    let pace_metrics = if let (Some(distance), Some(duration)) =
        (activity.distance_meters, Some(activity.duration_seconds))
    {
        let distance_km = distance / 1000.0; // Convert meters to km
                                             // Safe: duration_seconds represents activity time, precision loss acceptable for human-readable metrics
        let duration_hours = duration as f64 / 3600.0;
        let average_pace_min_per_km = if distance_km > 0.0 {
            // Safe: duration_seconds represents activity time, precision loss acceptable for human-readable metrics
            let duration_minutes = duration as f64 / 60.0;
            duration_minutes / distance_km
        } else {
            0.0
        };

        serde_json::json!({
            "average_pace_min_per_km": average_pace_min_per_km,
            "average_speed_kmh": if duration_hours > 0.0 { distance_km / duration_hours } else { 0.0 },
            "total_distance_km": distance_km,
            // Safe: duration_seconds represents activity time, precision loss acceptable for human-readable metrics
            "duration_minutes": duration as f64 / 60.0
        })
    } else {
        serde_json::json!({
            "error": "Insufficient data: distance or duration missing"
        })
    };

    // Calculate effort metrics from available data
    let effort_metrics = serde_json::json!({
        "average_heart_rate": activity.average_heart_rate,
        "max_heart_rate": activity.max_heart_rate,
        "average_power": activity.average_power,
        "max_power": activity.max_power,
        "elevation_gain_m": activity.elevation_gain
    });

    let mut analysis = serde_json::json!({
        "activity_name": activity.name,
        "activity_type": format!("{:?}", activity.sport_type),
        "pace_metrics": pace_metrics,
        "effort_metrics": effort_metrics
    });

    // Add zone analysis only if specifically requested and heart rate data is available
    if include_zones && activity.average_heart_rate.is_some() {
        analysis["zones_note"] =
            serde_json::json!("Zone analysis requires additional fitness configuration data");
    }

    Ok(analysis)
}

// Implement PluginTool for this static plugin
impl_static_plugin!(BasicAnalysisPlugin);
