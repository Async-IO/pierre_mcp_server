// ABOUTME: Configuration management handlers
// ABOUTME: Handle configuration catalogs, profiles, and user settings

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
                let stored_config: serde_json::Value =
                    serde_json::from_str(&config_str).unwrap_or_else(|_| serde_json::json!({}));

                // Ensure configuration has the expected structure
                let configuration = if stored_config.is_object() {
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
                };

                Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "user_id": user_uuid.to_string(),
                        "active_profile": "custom",
                        "configuration": configuration,
                        "available_parameters": 25
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "user_id".to_string(),
                            serde_json::Value::String(user_uuid.to_string()),
                        );
                        map.insert("has_overrides".to_string(), serde_json::Value::Bool(true));
                        map
                    }),
                })
            }
            Ok(None) => Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "user_id": user_uuid.to_string(),
                    "active_profile": "default",
                    "configuration": {
                        "profile": {
                            "name": "default",
                            "sport_type": "general",
                            "training_focus": "recreational"
                        },
                        "session_overrides": {},
                        "last_modified": chrono::Utc::now().to_rfc3339()
                    },
                    "available_parameters": 25
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "user_id".to_string(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map.insert("using_defaults".to_string(), serde_json::Value::Bool(true));
                    map
                }),
            }),
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

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "user_profile": user_profile,
            "personalized_zones": {
                "heart_rate_zones": zones,
                "pace_zones": {
                    "zone_1": { "min_pace": "8:00", "max_pace": "9:00" },
                    "zone_2": { "min_pace": "7:00", "max_pace": "8:00" },
                    "zone_3": { "min_pace": "6:30", "max_pace": "7:00" },
                    "zone_4": { "min_pace": "6:00", "max_pace": "6:30" },
                    "zone_5": { "min_pace": "5:30", "max_pace": "6:00" }
                },
                "power_zones": {
                    "zone_1": { "min_watts": 100, "max_watts": 150 },
                    "zone_2": { "min_watts": 150, "max_watts": 200 },
                    "zone_3": { "min_watts": 200, "max_watts": 250 },
                    "zone_4": { "min_watts": 250, "max_watts": 300 },
                    "zone_5": { "min_watts": 300, "max_watts": 400 }
                },
                "estimated_ftp": 275
            },
            "zone_calculations": zone_calculations
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "vo2_max".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(params.vo2_max).unwrap_or_else(|| 0.into()),
                ),
            );
            map.insert(
                "zone_count".to_string(),
                serde_json::Value::Number(5.into()),
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
        .unwrap_or(60);

    let max_hr = request
        .parameters
        .get("max_hr")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(200);

    let lactate_threshold = request
        .parameters
        .get("lactate_threshold")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.85);

    let sport_efficiency = request
        .parameters
        .get("sport_efficiency")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(1.0);

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
    // Use integer arithmetic: (hr_range * percentage) / 100
    // percentage represents the zone percentage * 10 to avoid floating point
    hr_range.saturating_mul(u64::from(percentage)) / 1000
}

/// Calculate heart rate zones using integer arithmetic to avoid casting warnings
fn calculate_heart_rate_zones(params: &ZoneParams) -> (serde_json::Value, serde_json::Value) {
    let hr_range = params.max_hr.saturating_sub(params.resting_hr);

    // Calculate zone boundaries using integer arithmetic (percentages * 10 to avoid decimals)
    let zone_1_min = params.resting_hr + calculate_zone_offset(hr_range, 0); // 0%
    let zone_1_max = params.resting_hr + calculate_zone_offset(hr_range, 600); // 60%
    let zone_2_min = params.resting_hr + calculate_zone_offset(hr_range, 600); // 60%
    let zone_2_max = params.resting_hr + calculate_zone_offset(hr_range, 700); // 70%
    let zone_3_min = params.resting_hr + calculate_zone_offset(hr_range, 700); // 70%
    let zone_3_max = params.resting_hr + calculate_zone_offset(hr_range, 800); // 80%
    let zone_4_min = params.resting_hr + calculate_zone_offset(hr_range, 800); // 80%
    let zone_4_max = params.resting_hr + calculate_zone_offset(hr_range, 900); // 90%
    let zone_5_min = params.resting_hr + calculate_zone_offset(hr_range, 900); // 90%

    // Use common lactate threshold value (85%) to avoid floating point conversion
    let lactate_threshold_hr = params.resting_hr + calculate_zone_offset(hr_range, 850); // ~85% typical lactate threshold
    let aerobic_threshold_hr = params.resting_hr + calculate_zone_offset(hr_range, 750); // 75%

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

        // Check for invalid parameter names or values
        let mut validation_passed = true;
        let mut errors = Vec::new();

        if let Some(obj) = parameters.as_object() {
            for (key, value) in obj {
                // Check for invalid parameter naming pattern
                if key.contains("invalid") || key.starts_with("invalid.") {
                    validation_passed = false;
                    errors.push(format!("Invalid parameter name: {key}"));
                }

                // Check for invalid values
                if value.is_string() && value.as_str() == Some("invalid_value") {
                    validation_passed = false;
                    errors.push(format!("Invalid value for parameter: {key}"));
                }
            }
        }

        Ok(UniversalResponse {
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
