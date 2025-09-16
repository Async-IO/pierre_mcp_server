// ABOUTME: Basic fitness analysis plugin demonstrating community tool development
// ABOUTME: Provides simple metrics calculation and analysis for activities

use crate::plugins::core::{PluginCategory, PluginImplementation, PluginInfo, PluginToolStatic};
use crate::plugins::PluginEnvironment;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::{impl_static_plugin, plugin_info, register_plugin};
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
        let _ = env
            .provider_registry
            .create_provider("strava")
            .map_err(|e| {
                ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
            })?;

        // For demo purposes, we'll return mock analysis
        // In a real implementation, you would fetch the actual activity
        let analysis_result = perform_basic_analysis(activity_id, include_zones);

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

fn perform_basic_analysis(_activity_id: &str, include_zones: bool) -> Value {
    // Mock analysis for demonstration
    let mut analysis = serde_json::json!({
        "pace_metrics": {
            "average_pace_min_per_km": 5.2,
            "best_pace_min_per_km": 4.8,
            "pace_consistency": "good"
        },
        "effort_metrics": {
            "perceived_effort": 7,
            "efficiency_score": 82,
            "endurance_factor": 0.91
        },
        "distance_metrics": {
            "total_distance_km": 10.5,
            "elevation_gain_m": 156,
            "grade_adjusted_pace": 5.4
        }
    });

    if include_zones {
        analysis["heart_rate_zones"] = serde_json::json!({
            "zone_1_percentage": 15,
            "zone_2_percentage": 45,
            "zone_3_percentage": 30,
            "zone_4_percentage": 8,
            "zone_5_percentage": 2,
            "average_hr": 152,
            "max_hr": 178
        });

        analysis["power_zones"] = serde_json::json!({
            "average_power": 245,
            "normalized_power": 258,
            "intensity_factor": 0.76,
            "training_stress_score": 89
        });
    }

    analysis
}

// Implement PluginTool for this static plugin
impl_static_plugin!(BasicAnalysisPlugin);

// Register this plugin at compile time
register_plugin!(BasicAnalysisPlugin);
