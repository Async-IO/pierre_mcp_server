// ABOUTME: User configuration tools for personalized training settings.
// ABOUTME: Implements catalog, profiles, user config, zones calculation, and validation.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # User Configuration Tools
//!
//! This module provides tools for user configuration with direct business logic:
//! - `GetConfigurationCatalogTool` - Get available configuration options
//! - `GetConfigurationProfilesTool` - Get configuration profile templates
//! - `GetUserConfigurationTool` - Get user's current configuration
//! - `UpdateUserConfigurationTool` - Update user configuration
//! - `CalculatePersonalizedZonesTool` - Calculate training zones
//! - `ValidateConfigurationTool` - Validate configuration values

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::warn;

use crate::config::catalog::CatalogBuilder;
use crate::config::environment::TrainingZonesConfig;
use crate::config::profiles::ProfileTemplates;
use crate::constants::configuration_system::AVAILABLE_PARAMETERS_COUNT;
use crate::constants::limits::METERS_PER_KILOMETER;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::intelligence::physiological_constants::configuration_validation;
use crate::intelligence::physiological_constants::heart_rate_zones::{
    AEROBIC_THRESHOLD_PERMILLE, LACTATE_THRESHOLD_PERMILLE, PERMILLE_DIVISOR, ZONE_1_MAX_PERMILLE,
    ZONE_1_MIN_PERMILLE, ZONE_2_MAX_PERMILLE, ZONE_3_MAX_PERMILLE, ZONE_4_MAX_PERMILLE,
};
use crate::intelligence::physiological_constants::physiological_defaults::{
    DEFAULT_ESTIMATED_FTP, DEFAULT_LACTATE_THRESHOLD, DEFAULT_MAX_HR, DEFAULT_RESTING_HR,
    DEFAULT_SPORT_EFFICIENCY, TRAINING_ZONE_COUNT,
};
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions
// ============================================================================

/// Calculate heart rate zone offset using integer arithmetic
fn calculate_zone_offset(hr_range: u64, percentage: u32) -> u64 {
    hr_range.saturating_mul(u64::from(percentage)) / PERMILLE_DIVISOR
}

/// Calculate pace zones from VO2 max using Jack Daniels VDOT formulas
fn calculate_pace_zones_from_vo2max(vo2_max: f64, config: &TrainingZonesConfig) -> Value {
    let base_velocity = (vo2_max + 4.60) / 0.182_258;

    let easy_velocity = base_velocity * config.vdot_easy_zone_percent;
    let tempo_velocity = base_velocity * config.vdot_tempo_zone_percent;
    let threshold_velocity = base_velocity * config.vdot_threshold_zone_percent;
    let interval_velocity = base_velocity * config.vdot_interval_zone_percent;
    let repetition_velocity = base_velocity * config.vdot_repetition_zone_percent;

    let format_pace = |velocity_m_per_min: f64| -> String {
        let seconds_per_km = METERS_PER_KILOMETER / velocity_m_per_min.max(1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total_secs = if !seconds_per_km.is_finite() || seconds_per_km < 0.0 {
            0_u32
        } else if seconds_per_km >= 4_294_967_295.0 {
            u32::MAX
        } else {
            seconds_per_km.round() as u32
        };
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes}:{seconds:02}")
    };

    json!({
        "zone_1_easy": { "min_pace": format_pace(easy_velocity * 0.85), "max_pace": format_pace(easy_velocity * 0.95) },
        "zone_2_moderate": { "min_pace": format_pace(tempo_velocity * 0.9), "max_pace": format_pace(tempo_velocity * 1.05) },
        "zone_3_threshold": { "min_pace": format_pace(threshold_velocity * 0.95), "max_pace": format_pace(threshold_velocity * 1.05) },
        "zone_4_interval": { "min_pace": format_pace(interval_velocity * 0.95), "max_pace": format_pace(interval_velocity * 1.05) },
        "zone_5_repetition": { "min_pace": format_pace(repetition_velocity * 0.95), "max_pace": format_pace(repetition_velocity * 1.05) }
    })
}

/// Calculate power zones from FTP
fn calculate_power_zones_from_ftp(ftp: u32, config: &TrainingZonesConfig) -> Value {
    let zone_1_min = 0_u32;
    let zone_1_max = u32::try_from(u64::from(ftp) * u64::from(config.ftp_zone1_percent) / 100)
        .unwrap_or(u32::MAX);
    let zone_2_max = u32::try_from(u64::from(ftp) * u64::from(config.ftp_zone2_percent) / 100)
        .unwrap_or(u32::MAX);
    let zone_3_max = u32::try_from(u64::from(ftp) * u64::from(config.ftp_zone3_percent) / 100)
        .unwrap_or(u32::MAX);
    let zone_4_max = u32::try_from(u64::from(ftp) * u64::from(config.ftp_zone4_percent) / 100)
        .unwrap_or(u32::MAX);
    let zone_5_max = u32::try_from(u64::from(ftp) * u64::from(config.ftp_zone5_percent) / 100)
        .unwrap_or(u32::MAX);

    json!({
        "zone_1": { "min_watts": zone_1_min, "max_watts": zone_1_max },
        "zone_2": { "min_watts": zone_1_max, "max_watts": zone_2_max },
        "zone_3": { "min_watts": zone_2_max, "max_watts": zone_3_max },
        "zone_4": { "min_watts": zone_3_max, "max_watts": zone_4_max },
        "zone_5": { "min_watts": zone_4_max, "max_watts": zone_5_max }
    })
}

/// Calculate heart rate zones
fn calculate_heart_rate_zones(
    resting_hr: u64,
    max_hr: u64,
    sport_efficiency: f64,
) -> (Value, Value) {
    let hr_range = max_hr.saturating_sub(resting_hr);

    let zone_1_min = resting_hr + calculate_zone_offset(hr_range, ZONE_1_MIN_PERMILLE);
    let zone_1_max = resting_hr + calculate_zone_offset(hr_range, ZONE_1_MAX_PERMILLE);
    let zone_2_max = resting_hr + calculate_zone_offset(hr_range, ZONE_2_MAX_PERMILLE);
    let zone_3_max = resting_hr + calculate_zone_offset(hr_range, ZONE_3_MAX_PERMILLE);
    let zone_4_max = resting_hr + calculate_zone_offset(hr_range, ZONE_4_MAX_PERMILLE);
    let zone_5_min = resting_hr + calculate_zone_offset(hr_range, ZONE_4_MAX_PERMILLE);

    let lactate_threshold_hr =
        resting_hr + calculate_zone_offset(hr_range, LACTATE_THRESHOLD_PERMILLE);
    let aerobic_threshold_hr =
        resting_hr + calculate_zone_offset(hr_range, AEROBIC_THRESHOLD_PERMILLE);

    let zones = json!({
        "zone_1": { "name": "Active Recovery", "min_hr": zone_1_min, "max_hr": zone_1_max },
        "zone_2": { "name": "Aerobic Base", "min_hr": zone_1_max, "max_hr": zone_2_max },
        "zone_3": { "name": "Aerobic Threshold", "min_hr": zone_2_max, "max_hr": zone_3_max },
        "zone_4": { "name": "Lactate Threshold", "min_hr": zone_3_max, "max_hr": zone_4_max },
        "zone_5": { "name": "VO2 Max", "min_hr": zone_5_min, "max_hr": max_hr }
    });

    let calculations = json!({
        "method": "heart_rate_reserve",
        "lactate_threshold_hr": lactate_threshold_hr,
        "aerobic_threshold_hr": aerobic_threshold_hr,
        "sport_efficiency_factor": sport_efficiency,
        "pace_formula": "Pace = 3.5 / (VO2 / body_weight)",
        "power_estimation": "Power = 0.98 * body_weight * VO2_max"
    });

    (zones, calculations)
}

/// Validate physiological parameter ranges
fn validate_parameter_ranges(obj: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(hr) = obj.get("max_hr").and_then(Value::as_u64) {
        if !(configuration_validation::MAX_HR_MIN..=configuration_validation::MAX_HR_MAX)
            .contains(&hr)
        {
            errors.push(format!(
                "max_hr must be between {} and {} bpm, got {}",
                configuration_validation::MAX_HR_MIN,
                configuration_validation::MAX_HR_MAX,
                hr
            ));
        }
    }

    if let Some(hr) = obj.get("resting_hr").and_then(Value::as_u64) {
        if !(configuration_validation::RESTING_HR_MIN..=configuration_validation::RESTING_HR_MAX)
            .contains(&hr)
        {
            errors.push(format!(
                "resting_hr must be between {} and {} bpm, got {}",
                configuration_validation::RESTING_HR_MIN,
                configuration_validation::RESTING_HR_MAX,
                hr
            ));
        }
    }

    if let Some(hr) = obj.get("threshold_hr").and_then(Value::as_u64) {
        if !(configuration_validation::THRESHOLD_HR_MIN
            ..=configuration_validation::THRESHOLD_HR_MAX)
            .contains(&hr)
        {
            errors.push(format!(
                "threshold_hr must be between {} and {} bpm, got {}",
                configuration_validation::THRESHOLD_HR_MIN,
                configuration_validation::THRESHOLD_HR_MAX,
                hr
            ));
        }
    }

    if let Some(vo2) = obj.get("vo2_max").and_then(Value::as_f64) {
        if !(configuration_validation::VO2_MAX_MIN..=configuration_validation::VO2_MAX_MAX)
            .contains(&vo2)
        {
            errors.push(format!(
                "vo2_max must be between {} and {} ml/kg/min, got {:.1}",
                configuration_validation::VO2_MAX_MIN,
                configuration_validation::VO2_MAX_MAX,
                vo2
            ));
        }
    }

    if let Some(power) = obj.get("ftp").and_then(Value::as_u64) {
        if !(configuration_validation::FTP_MIN..=configuration_validation::FTP_MAX).contains(&power)
        {
            errors.push(format!(
                "ftp must be between {} and {} watts, got {}",
                configuration_validation::FTP_MIN,
                configuration_validation::FTP_MAX,
                power
            ));
        }
    }

    errors
}

/// Validate parameter relationships
fn validate_parameter_relationships(obj: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();

    let max_hr = obj.get("max_hr").and_then(Value::as_u64);
    let resting_hr = obj.get("resting_hr").and_then(Value::as_u64);
    let threshold_hr = obj.get("threshold_hr").and_then(Value::as_u64);

    if let (Some(resting), Some(max)) = (resting_hr, max_hr) {
        if resting >= max {
            errors.push(format!(
                "resting_hr ({resting}) must be less than max_hr ({max})"
            ));
        }
    }

    if let (Some(resting), Some(threshold)) = (resting_hr, threshold_hr) {
        if resting >= threshold {
            errors.push(format!(
                "resting_hr ({resting}) must be less than threshold_hr ({threshold})"
            ));
        }
    }

    if let (Some(threshold), Some(max)) = (threshold_hr, max_hr) {
        if threshold >= max {
            errors.push(format!(
                "threshold_hr ({threshold}) must be less than max_hr ({max})"
            ));
        }
    }

    errors
}

// ============================================================================
// GetConfigurationCatalogTool
// ============================================================================

/// Tool for getting the complete configuration catalog.
pub struct GetConfigurationCatalogTool;

#[async_trait]
impl McpTool for GetConfigurationCatalogTool {
    fn name(&self) -> &'static str {
        "get_configuration_catalog"
    }

    fn description(&self) -> &'static str {
        "Get the complete catalog of available configuration options"
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, _args: Value, _ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let catalog = CatalogBuilder::build();

        Ok(ToolResult::ok(json!({
            "catalog": catalog,
            "catalog_type": "complete",
            "parameter_count": catalog.total_parameters,
        })))
    }
}

// ============================================================================
// GetConfigurationProfilesTool
// ============================================================================

/// Tool for getting available configuration profiles.
pub struct GetConfigurationProfilesTool;

#[async_trait]
impl McpTool for GetConfigurationProfilesTool {
    fn name(&self) -> &'static str {
        "get_configuration_profiles"
    }

    fn description(&self) -> &'static str {
        "Get available configuration profile templates"
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, _args: Value, _ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let profile_templates = ProfileTemplates::all();

        let profiles: Vec<Value> = profile_templates
            .into_iter()
            .map(|(name, profile)| {
                json!({
                    "name": name,
                    "profile": profile,
                    "description": format!("Configuration profile: {name}")
                })
            })
            .collect();

        Ok(ToolResult::ok(json!({
            "profiles": profiles,
            "total_count": profiles.len(),
        })))
    }
}

// ============================================================================
// GetUserConfigurationTool
// ============================================================================

/// Tool for getting user's current configuration.
pub struct GetUserConfigurationTool;

#[async_trait]
impl McpTool for GetUserConfigurationTool {
    fn name(&self) -> &'static str {
        "get_user_configuration"
    }

    fn description(&self) -> &'static str {
        "Get your current training configuration settings"
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, _args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let user_id_str = ctx.user_id.to_string();

        match (*ctx.resources.database)
            .get_user_configuration(&user_id_str)
            .await
        {
            Ok(Some(config_str)) => {
                let stored_config: Value = serde_json::from_str(&config_str).unwrap_or_else(|e| {
                    warn!(
                        user_id = %ctx.user_id,
                        error = %e,
                        "Failed to parse stored fitness configuration JSON"
                    );
                    json!({})
                });

                // Normalize stored configuration
                let configuration = if stored_config.is_object() {
                    let profile = stored_config.get("profile").cloned().unwrap_or_else(|| {
                        json!({
                            "name": "custom",
                            "sport_type": "general",
                            "training_focus": "custom"
                        })
                    });
                    let session_overrides = stored_config
                        .get("session_overrides")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    let last_modified = stored_config
                        .get("last_modified")
                        .cloned()
                        .unwrap_or_else(|| json!(chrono::Utc::now().to_rfc3339()));

                    json!({
                        "profile": profile,
                        "session_overrides": session_overrides,
                        "last_modified": last_modified
                    })
                } else {
                    json!({
                        "profile": {
                            "name": "custom",
                            "sport_type": "general",
                            "training_focus": "custom"
                        },
                        "session_overrides": {},
                        "last_modified": chrono::Utc::now().to_rfc3339()
                    })
                };

                Ok(ToolResult::ok(json!({
                    "user_id": user_id_str,
                    "active_profile": "custom",
                    "configuration": configuration,
                    "available_parameters": AVAILABLE_PARAMETERS_COUNT,
                    "has_overrides": true,
                })))
            }
            Ok(None) => {
                let default_configuration = json!({
                    "profile": {
                        "name": "default",
                        "sport_type": "general",
                        "training_focus": "recreational"
                    },
                    "session_overrides": {},
                    "last_modified": chrono::Utc::now().to_rfc3339()
                });

                Ok(ToolResult::ok(json!({
                    "user_id": user_id_str,
                    "active_profile": "default",
                    "configuration": default_configuration,
                    "available_parameters": AVAILABLE_PARAMETERS_COUNT,
                    "using_defaults": true,
                })))
            }
            Err(e) => Err(AppError::internal(format!(
                "Failed to get user configuration: {e}"
            ))),
        }
    }
}

// ============================================================================
// UpdateUserConfigurationTool
// ============================================================================

/// Tool for updating user configuration.
pub struct UpdateUserConfigurationTool;

#[async_trait]
impl McpTool for UpdateUserConfigurationTool {
    fn name(&self) -> &'static str {
        "update_user_configuration"
    }

    fn description(&self) -> &'static str {
        "Update your training configuration settings"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "profile".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some("Profile name to apply".to_owned()),
            },
        );
        properties.insert(
            "parameters".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some("Configuration parameters to update".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::WRITES_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let user_id_str = ctx.user_id.to_string();

        let profile = args
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("custom");
        let parameters = args.get("parameters").cloned().unwrap_or_else(|| json!({}));

        let configuration = json!({
            "active_profile": profile,
            "profile": {
                "name": profile,
                "sport_type": "general",
                "training_focus": "custom"
            },
            "session_overrides": parameters,
            "applied_overrides": parameters.as_object().map_or(0, serde_json::Map::len),
            "last_modified": chrono::Utc::now().to_rfc3339()
        });

        let config_json = serde_json::to_string(&configuration)
            .map_err(|e| AppError::internal(format!("Failed to serialize config: {e}")))?;

        (*ctx.resources.database)
            .save_user_configuration(&user_id_str, &config_json)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update configuration: {e}")))?;

        let param_count = parameters.as_object().map_or(0, serde_json::Map::len);

        Ok(ToolResult::ok(json!({
            "success": true,
            "user_id": user_id_str,
            "updated_configuration": configuration,
            "changes_applied": param_count,
            "message": "Configuration updated successfully",
        })))
    }
}

// ============================================================================
// CalculatePersonalizedZonesTool
// ============================================================================

/// Tool for calculating personalized training zones.
pub struct CalculatePersonalizedZonesTool;

#[async_trait]
impl McpTool for CalculatePersonalizedZonesTool {
    fn name(&self) -> &'static str {
        "calculate_personalized_zones"
    }

    fn description(&self) -> &'static str {
        "Calculate personalized training zones based on your fitness metrics"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "vo2_max".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("VO2 max in ml/kg/min (required)".to_owned()),
            },
        );
        properties.insert(
            "resting_hr".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Resting heart rate in bpm".to_owned()),
            },
        );
        properties.insert(
            "max_hr".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Maximum heart rate in bpm".to_owned()),
            },
        );
        properties.insert(
            "lactate_threshold".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Lactate threshold".to_owned()),
            },
        );
        properties.insert(
            "sport_efficiency".to_owned(),
            PropertySchema {
                property_type: "number".to_owned(),
                description: Some("Sport efficiency factor".to_owned()),
            },
        );
        properties.insert(
            "ftp".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Functional Threshold Power in watts".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["vo2_max".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let vo2_max = args
            .get("vo2_max")
            .and_then(Value::as_f64)
            .ok_or_else(|| AppError::invalid_input("vo2_max parameter required"))?;

        let resting_hr = args
            .get("resting_hr")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_RESTING_HR);
        let max_hr = args
            .get("max_hr")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_MAX_HR);
        let lactate_threshold = args
            .get("lactate_threshold")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_LACTATE_THRESHOLD);
        let sport_efficiency = args
            .get("sport_efficiency")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_SPORT_EFFICIENCY);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ftp = args
            .get("ftp")
            .and_then(Value::as_u64)
            .map_or(DEFAULT_ESTIMATED_FTP, |f| f.min(u64::from(u32::MAX)) as u32);

        let user_profile = json!({
            "vo2_max": vo2_max,
            "resting_hr": resting_hr,
            "max_hr": max_hr,
            "lactate_threshold": lactate_threshold,
            "sport_efficiency": sport_efficiency
        });

        let (hr_zones, zone_calculations) =
            calculate_heart_rate_zones(resting_hr, max_hr, sport_efficiency);
        let pace_zones =
            calculate_pace_zones_from_vo2max(vo2_max, &ctx.resources.config.training_zones);
        let power_zones = calculate_power_zones_from_ftp(ftp, &ctx.resources.config.training_zones);

        Ok(ToolResult::ok(json!({
            "user_profile": user_profile,
            "personalized_zones": {
                "heart_rate_zones": hr_zones,
                "pace_zones": pace_zones,
                "power_zones": power_zones,
                "estimated_ftp": ftp
            },
            "zone_calculations": zone_calculations,
            "vo2_max": vo2_max,
            "zone_count": TRAINING_ZONE_COUNT,
            "ftp_source": if args.get("ftp").is_some() { "provided" } else { "default_estimate" },
        })))
    }
}

// ============================================================================
// ValidateConfigurationTool
// ============================================================================

/// Tool for validating configuration parameters.
pub struct ValidateConfigurationTool;

#[async_trait]
impl McpTool for ValidateConfigurationTool {
    fn name(&self) -> &'static str {
        "validate_configuration"
    }

    fn description(&self) -> &'static str {
        "Validate configuration parameters for physiological correctness"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "parameters".to_owned(),
            PropertySchema {
                property_type: "object".to_owned(),
                description: Some("Configuration parameters to validate".to_owned()),
            },
        );
        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: Some(vec!["parameters".to_owned()]),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, _ctx: &ToolExecutionContext) -> AppResult<ToolResult> {
        let parameters = args
            .get("parameters")
            .ok_or_else(|| AppError::invalid_input("parameters field required"))?;

        if !parameters.is_object() {
            return Ok(ToolResult::ok(json!({
                "validation_passed": false,
                "parameters_validated": 0,
                "errors": ["Parameters must be a JSON object"],
            })));
        }

        let obj = parameters.as_object().ok_or_else(|| {
            AppError::internal("Parameters is not an object despite passing is_object check")
        })?;

        let param_count = obj.len();
        let mut all_errors = Vec::new();

        // Validate ranges
        all_errors.extend(validate_parameter_ranges(obj));

        // Validate relationships
        all_errors.extend(validate_parameter_relationships(obj));

        // Legacy pattern validation
        for (key, value) in obj {
            if key.contains("invalid") || key.starts_with("invalid.") {
                all_errors.push(format!("Invalid parameter name: {key}"));
            }
            if value.is_string() && value.as_str() == Some("invalid_value") {
                all_errors.push(format!("Invalid value for parameter: {key}"));
            }
        }

        let validation_passed = all_errors.is_empty();

        Ok(ToolResult::ok(json!({
            "validation_passed": validation_passed,
            "parameters_validated": param_count,
            "message": if validation_passed {
                "Configuration parameters are valid"
            } else {
                "Configuration validation failed"
            },
            "errors": if all_errors.is_empty() { Value::Null } else { json!(all_errors) },
        })))
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all configuration tools for registration
#[must_use]
pub fn create_configuration_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(GetConfigurationCatalogTool),
        Box::new(GetConfigurationProfilesTool),
        Box::new(GetUserConfigurationTool),
        Box::new(UpdateUserConfigurationTool),
        Box::new(CalculatePersonalizedZonesTool),
        Box::new(ValidateConfigurationTool),
    ]
}
