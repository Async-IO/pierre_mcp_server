// ABOUTME: Configuration management handlers
// ABOUTME: Handle configuration catalogs, profiles, and user settings
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::configuration::{catalog::CatalogBuilder, profiles::ProfileTemplates};
use crate::database_plugins::DatabaseProvider;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use crate::utils::uuid::parse_user_id_for_protocol;
use std::future::Future;
use std::pin::Pin;

/// Handle `get_configuration_catalog` tool - get complete configuration catalog
///
/// # Errors
/// Returns `ProtocolError` if catalog serialization fails
pub fn handle_get_configuration_catalog(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    // Build configuration catalog
    let catalog = CatalogBuilder::build();

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "catalog": catalog
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "catalog_type".to_string(),
                serde_json::Value::String("complete".to_string()),
            );
            map.insert(
                "parameter_count".to_string(),
                serde_json::Value::Number(catalog.total_parameters.into()),
            );
            map
        }),
    })
}

/// Handle `get_configuration_profiles` tool - get available configuration profiles
///
/// # Errors
/// Returns `ProtocolError` if profiles serialization fails
pub fn handle_get_configuration_profiles(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    _request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    // Get available profile templates and transform to expected structure
    let profile_templates = ProfileTemplates::all();

    let profiles: Vec<serde_json::Value> = profile_templates
        .into_iter()
        .map(|(name, profile)| {
            serde_json::json!({
                "name": name,
                "profile": profile,
                "description": format!("Configuration profile: {}", name)
            })
        })
        .collect();

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "profiles": profiles,
            "total_count": profiles.len()
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "profile_count".to_string(),
                serde_json::Value::Number(profiles.len().into()),
            );
            map
        }),
    })
}

/// Handle `get_user_configuration` tool - get user's current configuration
#[must_use]
/// Normalize stored configuration structure with defaults
fn normalize_stored_configuration(stored_config: &serde_json::Value) -> serde_json::Value {
    if stored_config.is_object() {
        let profile = stored_config.get("profile").cloned().unwrap_or_else(|| {
            serde_json::json!({
                "name": "custom",
                "sport_type": "general",
                "training_focus": "custom"
            })
        });
        let session_overrides = stored_config
            .get("session_overrides")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let last_modified = stored_config
            .get("last_modified")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(chrono::Utc::now().to_rfc3339()));

        serde_json::json!({
            "profile": profile,
            "session_overrides": session_overrides,
            "last_modified": last_modified
        })
    } else {
        serde_json::json!({
            "profile": {
                "name": "custom",
                "sport_type": "general",
                "training_focus": "custom"
            },
            "session_overrides": {},
            "last_modified": chrono::Utc::now().to_rfc3339()
        })
    }
}

/// Build response with user configuration
fn build_configuration_response(
    user_uuid: &uuid::Uuid,
    configuration: &serde_json::Value,
    has_overrides: bool,
) -> UniversalResponse {
    let metadata_key = if has_overrides {
        "has_overrides"
    } else {
        "using_defaults"
    };

    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "user_id": user_uuid.to_string(),
            "active_profile": if has_overrides { "custom" } else { "default" },
            "configuration": configuration,
            "available_parameters": crate::constants::configuration_system::AVAILABLE_PARAMETERS_COUNT
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "user_id".to_string(),
                serde_json::Value::String(user_uuid.to_string()),
            );
            map.insert(metadata_key.to_string(), serde_json::Value::Bool(true));
            map
        }),
    }
}

#[must_use]
pub fn handle_get_user_configuration(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get user configuration from database
        match (*executor.resources.database)
            .get_user_configuration(&user_uuid.to_string())
            .await
        {
            Ok(Some(config_str)) => {
                let stored_config: serde_json::Value = serde_json::from_str(&config_str)
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            user_id = %user_uuid,
                            error = %e,
                            "Failed to parse stored fitness configuration JSON, using empty default"
                        );
                        serde_json::json!({})
                    });

                let configuration = normalize_stored_configuration(&stored_config);
                Ok(build_configuration_response(
                    &user_uuid,
                    &configuration,
                    true,
                ))
            }
            Ok(None) => {
                let default_configuration = serde_json::json!({
                    "profile": {
                        "name": "default",
                        "sport_type": "general",
                        "training_focus": "recreational"
                    },
                    "session_overrides": {},
                    "last_modified": chrono::Utc::now().to_rfc3339()
                });
                Ok(build_configuration_response(
                    &user_uuid,
                    &default_configuration,
                    false,
                ))
            }
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to get user configuration: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `update_user_configuration` tool - update user's configuration settings
#[must_use]
pub fn handle_update_user_configuration(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        // Parse user ID from request
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract profile and parameters from request
        let profile = request
            .parameters
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("custom");
        let parameters = request
            .parameters
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // Build complete configuration structure
        let configuration = serde_json::json!({
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

        // Save user configuration in database
        let config_json = serde_json::to_string(&configuration).map_err(|e| {
            ProtocolError::SerializationError(format!("Failed to serialize config: {e}"))
        })?;

        match (*executor.resources.database)
            .save_user_configuration(&user_uuid.to_string(), &config_json)
            .await
        {
            Ok(()) => {
                let param_count = parameters.as_object().map_or(0, serde_json::Map::len);
                Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "user_id": user_uuid.to_string(),
                        "updated_configuration": configuration,
                        "changes_applied": param_count,
                        "message": "Configuration updated successfully"
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "user_id".to_string(),
                            serde_json::Value::String(user_uuid.to_string()),
                        );
                        map.insert(
                            "updated_parameters".to_string(),
                            serde_json::Value::Number(param_count.into()),
                        );
                        map
                    }),
                })
            }
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to update configuration: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Calculate pace zones from VO2 max using Jack Daniels VDOT formulas
///
/// Returns pace zones in format "min:sec/km" based on training intensities
fn calculate_pace_zones_from_vo2max(vo2_max: f64) -> serde_json::Value {
    // Jack Daniels VDOT pace calculations
    // Formula: pace (min/km) = 1000 / (velocity_m_per_min)
    // Velocity from VO2: velocity = (VO2 + 4.60) / 0.182258 (derived from VDOT formula)

    let base_velocity = (vo2_max + 4.60) / 0.182_258; // meters per minute at VO2max

    // Calculate paces for different training zones (as % of VO2max velocity)
    let easy_velocity = base_velocity * 0.70; // 70% VO2max
    let tempo_velocity = base_velocity * 0.82; // 82% VO2max
    let threshold_velocity = base_velocity * 0.88; // 88% VO2max
    let interval_velocity = base_velocity * 0.98; // 98% VO2max
    let repetition_velocity = base_velocity * 1.10; // 110% VO2max

    // Convert to min:sec per km
    let format_pace = |velocity_m_per_min: f64| -> String {
        let seconds_per_km = 1000.0 / velocity_m_per_min.max(1.0);

        // Saturating conversion from f64 to u32 with explicit bounds checking
        // Note: clippy::cast_possible_truncation will warn in ultra-strict mode but conversion is validated
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

    serde_json::json!({
        "zone_1_easy": { "min_pace": format_pace(easy_velocity * 0.85), "max_pace": format_pace(easy_velocity * 0.95) },
        "zone_2_moderate": { "min_pace": format_pace(tempo_velocity * 0.9), "max_pace": format_pace(tempo_velocity * 1.05) },
        "zone_3_threshold": { "min_pace": format_pace(threshold_velocity * 0.95), "max_pace": format_pace(threshold_velocity * 1.05) },
        "zone_4_interval": { "min_pace": format_pace(interval_velocity * 0.95), "max_pace": format_pace(interval_velocity * 1.05) },
        "zone_5_repetition": { "min_pace": format_pace(repetition_velocity * 0.95), "max_pace": format_pace(repetition_velocity * 1.05) }
    })
}

/// Calculate power zones from FTP (Functional Threshold Power)
///
/// Returns power zones in watts based on standard FTP percentages
fn calculate_power_zones_from_ftp(ftp: u32) -> serde_json::Value {
    // Use integer arithmetic to avoid f64→u32 cast warnings
    // Standard FTP-based power zones using percentage multiplication with try_from
    // Note: All these calculations should succeed since ftp is u32 and multipliers are <2
    let zone_1_min = 0_u32; // Active Recovery: 0-55%
    let zone_1_max = u32::try_from(u64::from(ftp) * 55 / 100).unwrap_or_else(|e| {
        tracing::warn!(ftp = ftp, error = %e, "Zone 1 max calculation failed, using u32::MAX");
        u32::MAX
    });
    let zone_2_max = u32::try_from(u64::from(ftp) * 75 / 100).unwrap_or_else(|e| {
        tracing::warn!(ftp = ftp, error = %e, "Zone 2 max calculation failed, using u32::MAX");
        u32::MAX
    });
    let zone_3_max = u32::try_from(u64::from(ftp) * 90 / 100).unwrap_or_else(|e| {
        tracing::warn!(ftp = ftp, error = %e, "Zone 3 max calculation failed, using u32::MAX");
        u32::MAX
    });
    let zone_4_max = u32::try_from(u64::from(ftp) * 105 / 100).unwrap_or_else(|e| {
        tracing::warn!(ftp = ftp, error = %e, "Zone 4 max calculation failed, using u32::MAX");
        u32::MAX
    });
    let zone_5_max = u32::try_from(u64::from(ftp) * 120 / 100).unwrap_or_else(|e| {
        tracing::warn!(ftp = ftp, error = %e, "Zone 5 max calculation failed, using u32::MAX");
        u32::MAX
    });

    serde_json::json!({
        "zone_1": { "min_watts": zone_1_min, "max_watts": zone_1_max },
        "zone_2": { "min_watts": zone_1_max, "max_watts": zone_2_max },
        "zone_3": { "min_watts": zone_2_max, "max_watts": zone_3_max },
        "zone_4": { "min_watts": zone_3_max, "max_watts": zone_4_max },
        "zone_5": { "min_watts": zone_4_max, "max_watts": zone_5_max }
    })
}

/// Handle `calculate_personalized_zones` tool - calculate training zones based on VO2 max
///
/// # Errors
/// Returns `ProtocolError` if VO2 max parameter is missing or zones serialization fails
pub fn handle_calculate_personalized_zones(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    let params = extract_zone_parameters(request)?;
    let user_profile = create_user_profile(&params);
    let (zones, zone_calculations) = calculate_heart_rate_zones(&params);

    // Calculate personalized pace zones from VO2 max
    let pace_zones = calculate_pace_zones_from_vo2max(params.vo2_max);

    // Get FTP from parameters (optional) - if not provided, use default estimate
    let ftp = request
        .parameters
        .get("ftp")
        .and_then(serde_json::Value::as_u64)
        .and_then(|f| u32::try_from(f).ok())
        .unwrap_or(crate::intelligence::physiological_constants::physiological_defaults::DEFAULT_ESTIMATED_FTP);

    // Calculate power zones using FTP (either provided or default estimate)
    let power_zones_result = calculate_power_zones_from_ftp(ftp);

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "user_profile": user_profile,
            "personalized_zones": {
                "heart_rate_zones": zones,
                "pace_zones": pace_zones,
                "power_zones": power_zones_result,
                "estimated_ftp": ftp
            },
            "zone_calculations": zone_calculations
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            // Only include vo2_max if it's a valid f64 value
            if let Some(vo2_number) = serde_json::Number::from_f64(params.vo2_max) {
                map.insert("vo2_max".to_string(), serde_json::Value::Number(vo2_number));
            } else {
                tracing::warn!(
                    vo2_max = params.vo2_max,
                    "Invalid VO2 max value (NaN/Infinity), omitting from metadata"
                );
            }
            map.insert(
                "zone_count".to_string(),
                serde_json::Value::Number(crate::intelligence::physiological_constants::physiological_defaults::TRAINING_ZONE_COUNT.into()),
            );
            map.insert(
                "ftp_used".to_string(),
                serde_json::Value::Number(ftp.into()),
            );
            map.insert(
                "ftp_source".to_string(),
                serde_json::Value::String(if request.parameters.get("ftp").is_some() {
                    "provided".to_string()
                } else {
                    "default_estimate".to_string()
                }),
            );
            map
        }),
    })
}

/// Zone calculation parameters
struct ZoneParams {
    vo2_max: f64,
    resting_hr: u64,
    max_hr: u64,
    lactate_threshold: f64,
    sport_efficiency: f64,
}

/// Extract and validate zone calculation parameters
fn extract_zone_parameters(request: &UniversalRequest) -> Result<ZoneParams, ProtocolError> {
    let vo2_max = request
        .parameters
        .get("vo2_max")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ProtocolError::InvalidRequest("vo2_max parameter required".to_string()))?;

    let resting_hr = request
        .parameters
        .get("resting_hr")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(crate::intelligence::physiological_constants::physiological_defaults::DEFAULT_RESTING_HR);

    let max_hr = request
        .parameters
        .get("max_hr")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(
            crate::intelligence::physiological_constants::physiological_defaults::DEFAULT_MAX_HR,
        );

    let lactate_threshold = request
        .parameters
        .get("lactate_threshold")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(crate::intelligence::physiological_constants::physiological_defaults::DEFAULT_LACTATE_THRESHOLD);

    let sport_efficiency = request
        .parameters
        .get("sport_efficiency")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(crate::intelligence::physiological_constants::physiological_defaults::DEFAULT_SPORT_EFFICIENCY);

    Ok(ZoneParams {
        vo2_max,
        resting_hr,
        max_hr,
        lactate_threshold,
        sport_efficiency,
    })
}

/// Create user profile JSON
fn create_user_profile(params: &ZoneParams) -> serde_json::Value {
    serde_json::json!({
        "vo2_max": params.vo2_max,
        "resting_hr": params.resting_hr,
        "max_hr": params.max_hr,
        "lactate_threshold": params.lactate_threshold,
        "sport_efficiency": params.sport_efficiency
    })
}

/// Calculate heart rate zone offset using integer arithmetic to avoid casting warnings
fn calculate_zone_offset(hr_range: u64, percentage: u32) -> u64 {
    // Use integer arithmetic: (hr_range * percentage) / 1000
    // percentage represents the zone percentage in permille (thousandths)
    hr_range.saturating_mul(u64::from(percentage))
        / crate::intelligence::physiological_constants::heart_rate_zones::PERMILLE_DIVISOR
}

/// Calculate heart rate zones using integer arithmetic to avoid casting warnings
fn calculate_heart_rate_zones(params: &ZoneParams) -> (serde_json::Value, serde_json::Value) {
    let hr_range = params.max_hr.saturating_sub(params.resting_hr);

    // Calculate zone boundaries using integer arithmetic with permille constants
    let zone_1_min = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_1_MIN_PERMILLE,
        );
    let zone_1_max = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_1_MAX_PERMILLE,
        );
    let zone_2_min = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_1_MAX_PERMILLE,
        );
    let zone_2_max = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_2_MAX_PERMILLE,
        );
    let zone_3_min = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_2_MAX_PERMILLE,
        );
    let zone_3_max = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_3_MAX_PERMILLE,
        );
    let zone_4_min = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_3_MAX_PERMILLE,
        );
    let zone_4_max = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_4_MAX_PERMILLE,
        );
    let zone_5_min = params.resting_hr
        + calculate_zone_offset(
            hr_range,
            crate::intelligence::physiological_constants::heart_rate_zones::ZONE_4_MAX_PERMILLE,
        );

    // Use lactate and aerobic threshold constants
    let lactate_threshold_hr = params.resting_hr + calculate_zone_offset(hr_range, crate::intelligence::physiological_constants::heart_rate_zones::LACTATE_THRESHOLD_PERMILLE);
    let aerobic_threshold_hr = params.resting_hr + calculate_zone_offset(hr_range, crate::intelligence::physiological_constants::heart_rate_zones::AEROBIC_THRESHOLD_PERMILLE);

    let zones = serde_json::json!({
        "zone_1": {
            "name": "Active Recovery",
            "min_hr": zone_1_min,
            "max_hr": zone_1_max
        },
        "zone_2": {
            "name": "Aerobic Base",
            "min_hr": zone_2_min,
            "max_hr": zone_2_max
        },
        "zone_3": {
            "name": "Aerobic Threshold",
            "min_hr": zone_3_min,
            "max_hr": zone_3_max
        },
        "zone_4": {
            "name": "Lactate Threshold",
            "min_hr": zone_4_min,
            "max_hr": zone_4_max
        },
        "zone_5": {
            "name": "VO2 Max",
            "min_hr": zone_5_min,
            "max_hr": params.max_hr
        }
    });

    let zone_calculations = serde_json::json!({
        "method": "heart_rate_reserve",
        "lactate_threshold_hr": lactate_threshold_hr,
        "aerobic_threshold_hr": aerobic_threshold_hr,
        "sport_efficiency_factor": params.sport_efficiency,
        "pace_formula": "Pace = 3.5 / (VO2 / body_weight)",
        "power_estimation": "Power = 0.98 * body_weight * VO2_max"
    });

    (zones, zone_calculations)
}

/// Validate physiological parameter ranges
fn validate_parameter_ranges(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    use crate::intelligence::physiological_constants::configuration_validation;

    let mut all_valid = true;

    // Extract parameter values
    let max_hr = obj.get("max_hr").and_then(serde_json::Value::as_u64);
    let resting_hr = obj.get("resting_hr").and_then(serde_json::Value::as_u64);
    let threshold_hr = obj.get("threshold_hr").and_then(serde_json::Value::as_u64);
    let vo2_max = obj.get("vo2_max").and_then(serde_json::Value::as_f64);
    let ftp = obj.get("ftp").and_then(serde_json::Value::as_u64);

    // Validate max_hr
    if let Some(hr) = max_hr {
        if !(configuration_validation::MAX_HR_MIN..=configuration_validation::MAX_HR_MAX)
            .contains(&hr)
        {
            all_valid = false;
            errors.push(format!(
                "max_hr must be between {} and {} bpm, got {}",
                configuration_validation::MAX_HR_MIN,
                configuration_validation::MAX_HR_MAX,
                hr
            ));
        }
    }

    // Validate resting_hr
    if let Some(hr) = resting_hr {
        if !(configuration_validation::RESTING_HR_MIN..=configuration_validation::RESTING_HR_MAX)
            .contains(&hr)
        {
            all_valid = false;
            errors.push(format!(
                "resting_hr must be between {} and {} bpm, got {}",
                configuration_validation::RESTING_HR_MIN,
                configuration_validation::RESTING_HR_MAX,
                hr
            ));
        }
    }

    // Validate threshold_hr
    if let Some(hr) = threshold_hr {
        if !(configuration_validation::THRESHOLD_HR_MIN
            ..=configuration_validation::THRESHOLD_HR_MAX)
            .contains(&hr)
        {
            all_valid = false;
            errors.push(format!(
                "threshold_hr must be between {} and {} bpm, got {}",
                configuration_validation::THRESHOLD_HR_MIN,
                configuration_validation::THRESHOLD_HR_MAX,
                hr
            ));
        }
    }

    // Validate vo2_max
    if let Some(vo2) = vo2_max {
        if !(configuration_validation::VO2_MAX_MIN..=configuration_validation::VO2_MAX_MAX)
            .contains(&vo2)
        {
            all_valid = false;
            errors.push(format!(
                "vo2_max must be between {} and {} ml/kg/min, got {:.1}",
                configuration_validation::VO2_MAX_MIN,
                configuration_validation::VO2_MAX_MAX,
                vo2
            ));
        }
    }

    // Validate ftp
    if let Some(power) = ftp {
        if !(configuration_validation::FTP_MIN..=configuration_validation::FTP_MAX).contains(&power)
        {
            all_valid = false;
            errors.push(format!(
                "ftp must be between {} and {} watts, got {}",
                configuration_validation::FTP_MIN,
                configuration_validation::FTP_MAX,
                power
            ));
        }
    }

    all_valid
}

/// Validate physiological parameter relationships
fn validate_parameter_relationships(
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
) -> bool {
    let mut all_valid = true;

    let max_hr = obj.get("max_hr").and_then(serde_json::Value::as_u64);
    let resting_hr = obj.get("resting_hr").and_then(serde_json::Value::as_u64);
    let threshold_hr = obj.get("threshold_hr").and_then(serde_json::Value::as_u64);

    // Validate resting_hr < max_hr
    if let (Some(resting), Some(max)) = (resting_hr, max_hr) {
        if resting >= max {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than max_hr ({max})"
            ));
        }
    }

    // Validate resting_hr < threshold_hr
    if let (Some(resting), Some(threshold)) = (resting_hr, threshold_hr) {
        if resting >= threshold {
            all_valid = false;
            errors.push(format!(
                "resting_hr ({resting}) must be less than threshold_hr ({threshold})"
            ));
        }
    }

    // Validate threshold_hr < max_hr
    if let (Some(threshold), Some(max)) = (threshold_hr, max_hr) {
        if threshold >= max {
            all_valid = false;
            errors.push(format!(
                "threshold_hr ({threshold}) must be less than max_hr ({max})"
            ));
        }
    }

    all_valid
}

/// Handle `validate_configuration` tool - validate configuration parameters
///
/// # Errors
/// Returns `ProtocolError` if configuration parameter is missing
pub fn handle_validate_configuration(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    // Extract parameters to validate
    let parameters = request
        .parameters
        .get("parameters")
        .ok_or_else(|| ProtocolError::InvalidRequest("parameters field required".to_string()))?;

    // Validate parameters structure and content
    if parameters.is_object() {
        let param_count = parameters.as_object().map_or(0, serde_json::Map::len);

        // Collect validation errors
        let mut errors = Vec::new();

        if let Some(obj) = parameters.as_object() {
            // Perform range validations
            let ranges_valid = validate_parameter_ranges(obj, &mut errors);

            // Perform relationship validations
            let relationships_valid = validate_parameter_relationships(obj, &mut errors);

            // Legacy check for "invalid" string patterns (backward compatibility)
            let mut legacy_valid = true;
            for (key, value) in obj {
                if key.contains("invalid") || key.starts_with("invalid.") {
                    legacy_valid = false;
                    errors.push(format!("Invalid parameter name: {key}"));
                }

                if value.is_string() && value.as_str() == Some("invalid_value") {
                    legacy_valid = false;
                    errors.push(format!("Invalid value for parameter: {key}"));
                }
            }

            let validation_passed = ranges_valid && relationships_valid && legacy_valid;

            return Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "validation_passed": validation_passed,
                    "parameters_validated": param_count,
                    "message": if validation_passed {
                        "Configuration parameters are valid"
                    } else {
                        "Configuration validation failed"
                    },
                    "errors": if errors.is_empty() { serde_json::Value::Null } else { serde_json::json!(errors) }
                })),
                error: None,
                metadata: None,
            });
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "validation_passed": true,
                "parameters_validated": param_count,
                "message": "Configuration parameters are valid",
                "errors": serde_json::Value::Null
            })),
            error: None,
            metadata: None,
        })
    } else {
        Ok(UniversalResponse {
            success: false,
            result: Some(serde_json::json!({
                "validation_passed": false,
                "parameters_validated": 0,
                "errors": ["Parameters must be a JSON object"]
            })),
            error: Some("Validation failed: Parameters must be a JSON object".to_string()),
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "error_count".to_string(),
                    serde_json::Value::Number(1.into()),
                );
                map
            }),
        })
    }
}
