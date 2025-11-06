// ABOUTME: Intelligence and analysis handlers with clean separation
// ABOUTME: AI-powered analysis tools that delegate to intelligence services
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use chrono::{Duration, Utc};
use std::future::Future;
use std::pin::Pin;
use tracing;

/// Handle `calculate_metrics` tool - calculate custom fitness metrics (sync)
///
/// # Errors
/// Returns `ProtocolError` if activity parameter is missing or calculation fails
pub fn handle_calculate_metrics(
    _executor: &crate::protocols::universal::UniversalToolExecutor,
    request: &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    use crate::constants::limits;
    use crate::intelligence::physiological_constants::{
        efficiency_defaults::{DEFAULT_EFFICIENCY_SCORE, DEFAULT_EFFICIENCY_WITH_DISTANCE},
        hr_estimation::ASSUMED_MAX_HR,
        unit_conversions::MS_TO_KMH_FACTOR,
    };

    // Extract activity data from parameters
    let activity = request.parameters.get("activity").ok_or_else(|| {
        ProtocolError::InvalidRequest("activity parameter is required".to_owned())
    })?;

    let distance = activity
        .get("distance")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    let duration = activity
        .get("duration")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let elevation_gain = activity
        .get("elevation_gain")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    let heart_rate = activity
        .get("average_heart_rate")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok());

    // Get user's actual max HR if provided, otherwise calculate from age
    let max_hr_provided = request
        .parameters
        .get("max_hr")
        .and_then(serde_json::Value::as_f64);

    let user_age = request
        .parameters
        .get("age")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok());

    // Determine max HR: 1) explicit max_hr, 2) calculated from age (Fox formula), 3) fallback to constant
    let max_hr = match (max_hr_provided, user_age) {
        (Some(hr), _) => hr,
        (None, Some(age)) => f64::from(
            crate::intelligence::physiological_constants::heart_rate::AGE_BASED_MAX_HR_CONSTANT
                .saturating_sub(age),
        ),
        (None, None) => ASSUMED_MAX_HR,
    };

    // Calculate metrics using f64 to avoid precision loss
    let duration_f64 =
        f64::from(u32::try_from(duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX));

    let pace = if distance > 0.0 && duration > 0 {
        duration_f64 / (distance / limits::METERS_PER_KILOMETER)
    } else {
        0.0
    };

    let speed = if duration > 0 {
        (distance / duration_f64) * MS_TO_KMH_FACTOR
    } else {
        0.0
    };

    // Use personalized max HR for intensity score calculation
    let intensity_score = heart_rate.map_or(DEFAULT_EFFICIENCY_SCORE, |hr| {
        (f64::from(hr) / max_hr) * limits::PERCENTAGE_MULTIPLIER
    });

    let efficiency_score = if distance > 0.0 && elevation_gain > 0.0 {
        (distance / elevation_gain).min(100.0)
    } else {
        DEFAULT_EFFICIENCY_WITH_DISTANCE
    };

    Ok(UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "pace": pace,
            "speed": speed,
            "intensity_score": intensity_score,
            "efficiency_score": efficiency_score,
            "max_hr_used": max_hr,
            "max_hr_source": match (max_hr_provided, user_age) {
                (Some(_), _) => "provided".to_owned(),
                (None, Some(age)) => format!("calculated_from_age_{age}"),
                (None, None) => "default_assumed".to_owned(),
            },
            "metrics_summary": {
                "distance_km": distance / limits::METERS_PER_KILOMETER,
                "duration_minutes": duration / limits::SECONDS_PER_MINUTE,
                "elevation_meters": elevation_gain,
                "average_heart_rate": heart_rate
            }
        })),
        error: None,
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert(
                "calculation_timestamp".into(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
            map.insert(
                "metric_version".into(),
                serde_json::Value::String("2.0".into()),
            );
            map.insert(
                "personalized".into(),
                serde_json::Value::Bool(max_hr_provided.is_some() || user_age.is_some()),
            );
            map
        }),
    })
}

/// Generate insights and recommendations from activity data
fn generate_activity_insights(
    activity: &crate::models::Activity,
) -> (Vec<String>, Vec<&'static str>) {
    use crate::intelligence::physiological_constants::{
        business_thresholds::{
            ACHIEVEMENT_DISTANCE_THRESHOLD_KM, ACHIEVEMENT_ELEVATION_THRESHOLD_M,
        },
        heart_rate::HIGH_INTENSITY_HR_THRESHOLD,
    };

    let mut insights = Vec::new();
    let mut recommendations = Vec::new();

    // Analyze distance
    if let Some(distance) = activity.distance_meters {
        let km = distance / crate::constants::limits::METERS_PER_KILOMETER;
        insights.push(format!("Activity covered {km:.2} km"));
        if km > ACHIEVEMENT_DISTANCE_THRESHOLD_KM {
            recommendations.push("Great long-distance effort! Ensure proper recovery time");
        }
    }

    // Analyze elevation
    if let Some(elevation) = activity.elevation_gain {
        insights.push(format!("Total elevation gain: {elevation:.0} meters"));
        if elevation > ACHIEVEMENT_ELEVATION_THRESHOLD_M {
            recommendations.push("Significant elevation - consider targeted hill training");
        }
    }

    // Analyze heart rate
    if let Some(avg_hr) = activity.average_heart_rate {
        insights.push(format!("Average heart rate: {avg_hr} bpm"));
        if avg_hr > HIGH_INTENSITY_HR_THRESHOLD {
            recommendations.push("High-intensity effort detected - monitor recovery");
        }
    }

    // Analyze calories
    if let Some(calories) = activity.calories {
        insights.push(format!("Calories burned: {calories}"));
    }

    (insights, recommendations)
}

/// Create intelligence analysis JSON response
fn create_intelligence_response(
    activity: &crate::models::Activity,
    activity_id: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
) -> UniversalResponse {
    let (insights, recommendations) = generate_activity_insights(activity);

    let summary = format!(
        "{:?} activity completed. {} insights generated.",
        activity.sport_type,
        insights.len()
    );

    let duration_minutes = f64::from(
        u32::try_from(activity.duration_seconds.min(u64::from(u32::MAX))).unwrap_or(u32::MAX),
    ) / 60.0;

    let analysis = serde_json::json!({
        "activity_id": activity_id,
        "activity_type": format!("{:?}", activity.sport_type),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "intelligence": {
            "summary": summary,
            "insights": insights,
            "recommendations": recommendations,
            "performance_metrics": {
                "distance_km": activity.distance_meters.map(|d| d / crate::constants::limits::METERS_PER_KILOMETER),
                "duration_minutes": Some(duration_minutes),
                "elevation_meters": activity.elevation_gain,
                "average_heart_rate": activity.average_heart_rate,
                "max_heart_rate": activity.max_heart_rate,
                "calories": activity.calories
            }
        }
    });

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "activity_id".to_owned(),
        serde_json::Value::String(activity_id.to_owned()),
    );
    metadata.insert(
        "user_id".to_owned(),
        serde_json::Value::String(user_uuid.to_string()),
    );
    metadata.insert(
        "tenant_id".to_owned(),
        tenant_id.map_or(serde_json::Value::Null, serde_json::Value::String),
    );
    metadata.insert(
        "analysis_type".to_owned(),
        serde_json::Value::String("intelligence".to_owned()),
    );

    UniversalResponse {
        success: true,
        result: Some(analysis),
        error: None,
        metadata: Some(metadata),
    }
}

/// Handle `get_activity_intelligence` tool - get AI analysis for activity (async)
///
/// # Errors
/// Returns `ProtocolError` if `activity_id` parameter is missing or validation fails
#[must_use]
pub fn handle_get_activity_intelligence(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: activity_id".to_owned())
            })?;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        match executor
            .auth_service
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor
                        .resources
                        .config
                        .oauth
                        .strava
                        .client_id
                        .clone()
                        .unwrap_or_default(),
                    client_secret: executor
                        .resources
                        .config
                        .oauth
                        .strava
                        .client_secret
                        .clone()
                        .unwrap_or_default(),
                    access_token: Some(token_data.access_token.clone()), // Safe: String ownership for OAuth credentials
                    refresh_token: Some(token_data.refresh_token.clone()), // Safe: String ownership for OAuth credentials
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!(
                        "Failed to set provider credentials: {e}"
                    ))
                })?;

                match provider.get_activity(activity_id).await {
                    Ok(activity) => Ok(create_intelligence_response(
                        &activity,
                        activity_id,
                        user_uuid,
                        request.tenant_id,
                    )),
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activity {activity_id}: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert(
                    "authentication_required".to_owned(),
                    serde_json::Value::Bool(true),
                );
                Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned(),
                    ),
                    metadata: Some(metadata),
                })
            }
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `analyze_performance_trends` tool - analyze performance over time
#[must_use]
pub fn handle_analyze_performance_trends(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::MAX_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let metric = request
            .parameters
            .get("metric")
            .and_then(|v| v.as_str())
            .unwrap_or("pace");
        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("month");

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activities(Some(MAX_ACTIVITY_LIMIT), None).await {
                    Ok(activities) => {
                        let analysis = analyze_performance_trend(&activities, metric, timeframe);

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `compare_activities` tool - compare two activities
#[must_use]
pub fn handle_compare_activities(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::DEFAULT_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: activity_id".to_owned())
            })?;
        let comparison_type = request
            .parameters
            .get("comparison_type")
            .and_then(|v| v.as_str())
            .unwrap_or("similar_activities");
        let compare_activity_id = request
            .parameters
            .get("compare_activity_id")
            .and_then(|v| v.as_str());

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activity(activity_id).await {
                    Ok(target_activity) => {
                        // Get additional activities for comparison
                        let all_activities = provider.get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None).await
                            .unwrap_or_default();

                        let comparison = compare_activity_logic(
                            &target_activity,
                            &all_activities,
                            comparison_type,
                            compare_activity_id,
                        );

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(comparison),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activity {activity_id}: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `detect_patterns` tool - detect patterns in activity data
#[must_use]
pub fn handle_detect_patterns(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::DEFAULT_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let pattern_type = request
            .parameters
            .get("pattern_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: pattern_type".to_owned())
            })?;

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None).await {
                    Ok(activities) => {
                        let analysis = detect_activity_patterns(&activities, pattern_type);

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `generate_recommendations` tool - generate training recommendations
#[must_use]
pub fn handle_generate_recommendations(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::DEFAULT_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let recommendation_type = request
            .parameters
            .get("recommendation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None).await {
                    Ok(activities) => {
                        let analysis = generate_training_recommendations(&activities, recommendation_type);

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `calculate_fitness_score` tool - calculate overall fitness score
#[must_use]
pub fn handle_calculate_fitness_score(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::DEFAULT_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("month");

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None).await {
                    Ok(activities) => {
                        let analysis = calculate_fitness_metrics(&activities, timeframe);

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `predict_performance` tool - predict future performance
#[must_use]
pub fn handle_predict_performance(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::DEFAULT_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let target_sport = request
            .parameters
            .get("target_sport")
            .and_then(|v| v.as_str())
            .unwrap_or("Run");

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None).await {
                    Ok(activities) => {
                        let prediction = predict_race_performance(&activities, target_sport);

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(prediction),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

/// Handle `analyze_training_load` tool - analyze training load and recovery
#[must_use]
pub fn handle_analyze_training_load(
    executor: &crate::protocols::universal::UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use crate::constants::oauth_providers;
        use crate::intelligence::physiological_constants::api_limits::DEFAULT_ACTIVITY_LIMIT;
        use crate::utils::uuid::parse_user_id_for_protocol;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("week");

        match executor
            .auth_service
            .get_valid_token(user_uuid, oauth_providers::STRAVA, request.tenant_id.as_deref())
            .await
        {
            Ok(Some(token_data)) => {
                let provider = executor
                    .resources
                    .provider_registry
                    .create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: executor.resources.config.oauth.strava.client_id.clone().unwrap_or_default(),
                    client_secret: executor.resources.config.oauth.strava.client_secret.clone().unwrap_or_default(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                };

                provider.set_credentials(credentials).await.map_err(|e| {
                    ProtocolError::ExecutionFailed(format!("Failed to set credentials: {e}"))
                })?;

                match provider.get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None).await {
                    Ok(activities) => {
                        let analysis = analyze_detailed_training_load(&activities, timeframe);

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        })
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Ok(None) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_owned()),
                metadata: None,
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    })
}

// ============================================================================
// Helper Functions for Performance Trend Analysis
// ============================================================================

/// Analyze performance trend for a specific metric over time
fn analyze_performance_trend(
    activities: &[crate::models::Activity],
    metric: &str,
    timeframe: &str,
) -> serde_json::Value {
    use crate::intelligence::SafeMetricExtractor;

    if activities.is_empty() {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "no_data",
            "activities_analyzed": 0,
            "insights": ["No activities found for analysis"]
        });
    }

    // Parse metric string to MetricType
    let metric_type = match parse_metric_type(metric) {
        Ok(mt) => mt,
        Err(error_msg) => {
            return serde_json::json!({
                "metric": metric,
                "timeframe": timeframe,
                "trend": "invalid_metric",
                "activities_analyzed": 0,
                "insights": [error_msg]
            });
        }
    };

    // Filter activities by timeframe
    let cutoff_date = calculate_cutoff_date(timeframe);
    let filtered_activities: Vec<crate::models::Activity> = activities
        .iter()
        .filter(|a| a.start_date >= cutoff_date)
        .cloned()
        .collect();

    if filtered_activities.len() < 2 {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "needs_more_data",
            "activities_analyzed": filtered_activities.len(),
            "insights": [format!("Need at least 2 activities for trend analysis. Found {}", filtered_activities.len())]
        });
    }

    // Extract metric values using SafeMetricExtractor
    let Ok(data_points_with_timestamp) =
        SafeMetricExtractor::extract_metric_values(&filtered_activities, metric_type)
    else {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "insufficient_data",
            "activities_analyzed": filtered_activities.len(),
            "insights": [format!("Metric '{}' not available in enough activities", metric)]
        });
    };

    if data_points_with_timestamp.len() < 2 {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "insufficient_data",
            "activities_analyzed": filtered_activities.len(),
            "insights": [format!("Metric '{}' not available in enough activities", metric)]
        });
    }

    // Convert to TrendDataPoint format and perform regression
    compute_trend_statistics(metric, timeframe, metric_type, &data_points_with_timestamp)
}

/// Compute trend statistics from data points
fn compute_trend_statistics(
    metric: &str,
    timeframe: &str,
    metric_type: crate::intelligence::MetricType,
    data_points_with_timestamp: &[(chrono::DateTime<chrono::Utc>, f64)],
) -> serde_json::Value {
    use crate::intelligence::{StatisticalAnalyzer, TrendDataPoint, TrendDirection};

    // Convert to TrendDataPoint format
    let trend_data_points: Vec<TrendDataPoint> = data_points_with_timestamp
        .iter()
        .map(|(date, value)| TrendDataPoint {
            date: *date,
            value: *value,
            smoothed_value: None,
        })
        .collect();

    // Perform linear regression using StatisticalAnalyzer
    let Ok(regression_result) = StatisticalAnalyzer::linear_regression(&trend_data_points) else {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "calculation_error",
            "activities_analyzed": trend_data_points.len(),
            "insights": ["Unable to calculate trend statistics"]
        });
    };

    // Calculate simple average for comparison
    let sum: f64 = data_points_with_timestamp.iter().map(|(_, v)| v).sum();
    #[allow(clippy::cast_precision_loss)]
    let moving_avg = sum / data_points_with_timestamp.len() as f64;

    // Determine trend direction using proper logic
    let slope_threshold = 0.01;
    let trend_direction_enum = StatisticalAnalyzer::determine_trend_direction(
        &regression_result,
        metric_type.is_lower_better(),
        slope_threshold,
    );

    let trend_direction = match trend_direction_enum {
        TrendDirection::Improving => "improving",
        TrendDirection::Stable => "stable",
        TrendDirection::Declining => "declining",
    };

    // Generate insights
    let insights = generate_trend_insights(
        metric,
        trend_direction,
        regression_result.slope,
        regression_result.r_squared,
        data_points_with_timestamp,
    );

    serde_json::json!({
        "metric": metric,
        "timeframe": timeframe,
        "trend": trend_direction,
        "activities_analyzed": data_points_with_timestamp.len(),
        "statistics": {
            "slope": regression_result.slope,
            "r_squared": regression_result.r_squared,
            "confidence": regression_result.r_squared,
            "correlation": regression_result.correlation,
            "standard_error": regression_result.standard_error,
            "p_value": regression_result.p_value,
            "moving_average_7day": moving_avg,
            "start_value": data_points_with_timestamp.first().map(|(_, v)| v),
            "end_value": data_points_with_timestamp.last().map(|(_, v)| v),
            "percent_change": calculate_percent_change(data_points_with_timestamp),
        },
        "insights": insights,
    })
}

/// Parse metric string to `MetricType`
fn parse_metric_type(metric: &str) -> Result<crate::intelligence::MetricType, String> {
    use crate::intelligence::MetricType;
    match metric.to_lowercase().as_str() {
        "pace" => Ok(MetricType::Pace),
        "speed" => Ok(MetricType::Speed),
        "heart_rate" | "hr" => Ok(MetricType::HeartRate),
        "distance" => Ok(MetricType::Distance),
        "duration" => Ok(MetricType::Duration),
        "elevation" => Ok(MetricType::Elevation),
        "power" => Ok(MetricType::Power),
        _ => Err(format!("Unknown metric type: {metric}")),
    }
}

/// Calculate cutoff date based on timeframe
fn calculate_cutoff_date(timeframe: &str) -> chrono::DateTime<chrono::Utc> {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    match timeframe {
        "week" => now - Duration::days(7),
        "quarter" => now - Duration::days(90),
        "year" => now - Duration::days(365),
        _ => now - Duration::days(30), // default to month
    }
}

/// Calculate percent change between first and last data point
fn calculate_percent_change(data: &[(chrono::DateTime<chrono::Utc>, f64)]) -> Option<f64> {
    if data.len() < 2 {
        return None;
    }

    let first = data.first()?.1;
    let last = data.last()?.1;

    if first.abs() < f64::EPSILON {
        return None;
    }

    Some(((last - first) / first) * 100.0)
}

/// Generate insights from trend analysis
fn generate_trend_insights(
    metric: &str,
    trend: &str,
    slope: f64,
    r_squared: f64,
    data: &[(chrono::DateTime<chrono::Utc>, f64)],
) -> Vec<String> {
    let mut insights = Vec::new();

    if let (Some(last), Some(first)) = (data.last(), data.first()) {
        insights.push(format!(
            "Analyzed {} data points over {} days",
            data.len(),
            (last.0 - first.0).num_days()
        ));
    } else {
        insights.push(format!("Analyzed {} data points", data.len()));
    }

    match trend {
        "improving" => {
            insights.push(format!(
                "Your {} is improving with {:.1}% confidence",
                metric,
                r_squared * 100.0
            ));
            if r_squared > 0.7 {
                insights.push("Strong consistent improvement trend detected".to_owned());
            }
        }
        "declining" => {
            insights.push(format!(
                "Your {} is declining with {:.1}% confidence",
                metric,
                r_squared * 100.0
            ));
            if slope < -0.05 {
                insights.push("Consider reviewing your training plan or recovery".to_owned());
            }
        }
        "stable" => {
            if r_squared < 0.3 {
                insights.push(
                    "Performance is variable - maintain consistency for clearer trends".to_owned(),
                );
            } else {
                insights.push(format!("Your {metric} is maintaining steady performance"));
            }
        }
        _ => {}
    }

    if let Some(percent_change) = calculate_percent_change(data) {
        insights.push(format!(
            "Overall change: {percent_change:.1}% from start to end"
        ));
    }

    insights
}

// ============================================================================
// Helper Functions for Activity Comparison
// ============================================================================

/// Compare an activity using different comparison strategies
fn compare_activity_logic(
    target: &crate::models::Activity,
    all_activities: &[crate::models::Activity],
    comparison_type: &str,
    compare_activity_id: Option<&str>,
) -> serde_json::Value {
    match comparison_type {
        "pr_comparison" => compare_with_personal_records(target, all_activities),
        "specific_activity" => compare_activity_id.map_or_else(
            || compare_with_similar_activities(target, all_activities),
            |compare_id| compare_with_specific_activity(target, all_activities, compare_id),
        ),
        _ => compare_with_similar_activities(target, all_activities),
    }
}

/// Compare activity with similar past activities
fn compare_with_similar_activities(
    target: &crate::models::Activity,
    all_activities: &[crate::models::Activity],
) -> serde_json::Value {
    // Find similar activities (same sport, similar distance/duration)
    let similar: Vec<&crate::models::Activity> = all_activities
        .iter()
        .filter(|a| {
            a.id != target.id
                && a.sport_type == target.sport_type
                && is_similar_distance(a.distance_meters, target.distance_meters)
        })
        .take(5)
        .collect();

    if similar.is_empty() {
        return serde_json::json!({
            "activity_id": target.id,
            "comparison_type": "similar_activities",
            "comparison_count": 0,
            "insights": ["No similar activities found for comparison"],
        });
    }

    // Calculate average metrics from similar activities
    let avg_pace = calculate_average_pace(&similar);
    let avg_hr = calculate_average_hr(&similar);
    let avg_elevation = calculate_average_elevation(&similar);

    // Calculate target metrics
    let target_pace = calculate_pace(target);
    let target_hr = target.average_heart_rate.map(f64::from);

    // Generate comparisons
    let mut comparisons = Vec::new();
    let mut insights = Vec::new();

    if let (Some(target_p), Some(avg_p)) = (target_pace, avg_pace) {
        let pace_diff_pct = ((target_p - avg_p) / avg_p) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "pace",
            "current": target_p,
            "average": avg_p,
            "difference_percent": pace_diff_pct,
            "improved": pace_diff_pct < 0.0, // faster pace = lower value
        }));

        if pace_diff_pct < -5.0 {
            insights.push(format!(
                "Pace improved by {:.1}% compared to similar activities",
                pace_diff_pct.abs()
            ));
        } else if pace_diff_pct > 5.0 {
            insights.push(format!(
                "Pace was {pace_diff_pct:.1}% slower than similar activities"
            ));
        }
    }

    if let (Some(target_h), Some(avg_h)) = (target_hr, avg_hr) {
        let hr_diff_pct = ((target_h - avg_h) / avg_h) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "heart_rate",
            "current": target_h,
            "average": avg_h,
            "difference_percent": hr_diff_pct,
            "improved": hr_diff_pct < 0.0, // lower HR = better efficiency
        }));

        if hr_diff_pct < -5.0 {
            insights.push("Heart rate efficiency improved - same effort at lower HR".to_owned());
        } else if hr_diff_pct > 5.0 {
            insights.push("Heart rate was higher - consider recovery or pacing".to_owned());
        }
    }

    if let (Some(target_elev), Some(avg_elev)) = (target.elevation_gain, avg_elevation) {
        let elev_diff_pct = ((target_elev - avg_elev) / avg_elev) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "elevation_gain",
            "current": target_elev,
            "average": avg_elev,
            "difference_percent": elev_diff_pct,
        }));
    }

    if insights.is_empty() {
        insights.push(format!(
            "Compared with {} similar activities",
            similar.len()
        ));
    }

    serde_json::json!({
        "activity_id": target.id,
        "comparison_type": "similar_activities",
        "comparison_count": similar.len(),
        "sport_type": format!("{:?}", target.sport_type),
        "comparisons": comparisons,
        "insights": insights,
    })
}

/// Compare activity with personal records
fn compare_with_personal_records(
    target: &crate::models::Activity,
    all_activities: &[crate::models::Activity],
) -> serde_json::Value {
    // Find same sport activities
    let same_sport: Vec<&crate::models::Activity> = all_activities
        .iter()
        .filter(|a| a.sport_type == target.sport_type)
        .collect();

    if same_sport.is_empty() {
        return serde_json::json!({
            "activity_id": target.id,
            "comparison_type": "pr_comparison",
            "insights": ["No other activities of this sport type found"],
        });
    }

    let mut pr_comparisons = Vec::new();
    let mut insights = Vec::new();

    // Compare with longest distance
    if let Some(distance) = target.distance_meters {
        let max_distance = same_sport
            .iter()
            .filter_map(|a| a.distance_meters)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(max_d) = max_distance {
            let is_pr = distance >= max_d;
            pr_comparisons.push(serde_json::json!({
                "metric": "distance",
                "current": distance,
                "personal_record": max_d,
                "is_record": is_pr,
                "percent_of_pr": (distance / max_d) * 100.0,
            }));

            if is_pr && (distance - max_d).abs() > 100.0 {
                insights.push("New distance PR! 🎉".to_owned());
            }
        }
    }

    // Compare with fastest pace
    let target_pace = calculate_pace(target);
    let best_pace = same_sport
        .iter()
        .filter_map(|a| calculate_pace(a))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if let (Some(tp), Some(bp)) = (target_pace, best_pace) {
        let is_pr = tp <= bp;
        pr_comparisons.push(serde_json::json!({
            "metric": "pace",
            "current": tp,
            "personal_record": bp,
            "is_record": is_pr,
        }));

        if is_pr && (bp - tp).abs() > 0.1 {
            insights.push("New pace PR! 🚀".to_owned());
        }
    }

    // Compare with highest power (if available)
    if let Some(power) = target.average_power {
        let max_power = same_sport.iter().filter_map(|a| a.average_power).max();

        if let Some(max_p) = max_power {
            let is_pr = power >= max_p;
            pr_comparisons.push(serde_json::json!({
                "metric": "average_power",
                "current": power,
                "personal_record": max_p,
                "is_record": is_pr,
            }));

            if is_pr && power > max_p {
                insights.push("New power PR! 💪".to_owned());
            }
        }
    }

    if insights.is_empty() {
        insights.push(format!(
            "Compared with {} activities in this sport",
            same_sport.len()
        ));
    }

    serde_json::json!({
        "activity_id": target.id,
        "comparison_type": "pr_comparison",
        "sport_type": format!("{:?}", target.sport_type),
        "pr_comparisons": pr_comparisons,
        "insights": insights,
    })
}

/// Compare activity with a specific activity by ID
fn compare_with_specific_activity(
    target: &crate::models::Activity,
    all_activities: &[crate::models::Activity],
    compare_id: &str,
) -> serde_json::Value {
    // Find the specific activity to compare with
    let compare_activity = all_activities.iter().find(|a| a.id == compare_id);

    let Some(compare) = compare_activity else {
        return serde_json::json!({
            "activity_id": target.id,
            "comparison_type": "specific_activity",
            "error": format!("Activity with ID '{}' not found", compare_id),
            "insights": [format!("Could not find activity '{}' for comparison", compare_id)],
        });
    };

    // Calculate metrics for both activities
    let target_pace = calculate_pace(target);
    let compare_pace = calculate_pace(compare);
    let target_hr = target.average_heart_rate.map(f64::from);
    let compare_hr = compare.average_heart_rate.map(f64::from);

    let mut comparisons = Vec::new();
    let mut insights = Vec::new();

    // Distance comparison
    if let (Some(target_dist), Some(compare_dist)) =
        (target.distance_meters, compare.distance_meters)
    {
        let dist_diff_pct = ((target_dist - compare_dist) / compare_dist) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "distance",
            "current": target_dist,
            "comparison": compare_dist,
            "difference_percent": dist_diff_pct,
        }));
    }

    // Pace comparison
    if let (Some(target_p), Some(compare_p)) = (target_pace, compare_pace) {
        let pace_diff_pct = ((target_p - compare_p) / compare_p) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "pace",
            "current": target_p,
            "comparison": compare_p,
            "difference_percent": pace_diff_pct,
            "improved": pace_diff_pct < 0.0, // faster pace = lower value
        }));
        add_pace_insights(pace_diff_pct, &mut insights);
    }

    // Heart rate comparison
    if let (Some(target_h), Some(compare_h)) = (target_hr, compare_hr) {
        let hr_diff_pct = ((target_h - compare_h) / compare_h) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "heart_rate",
            "current": target_h,
            "comparison": compare_h,
            "difference_percent": hr_diff_pct,
            "improved": hr_diff_pct < 0.0, // lower HR = better efficiency
        }));
        add_heart_rate_insights(hr_diff_pct, &mut insights);
    }

    // Duration comparison
    #[allow(clippy::cast_precision_loss)]
    let duration_diff_pct = ((target.duration_seconds as f64 - compare.duration_seconds as f64)
        / compare.duration_seconds as f64)
        * 100.0;
    comparisons.push(serde_json::json!({
        "metric": "duration",
        "current": target.duration_seconds,
        "comparison": compare.duration_seconds,
        "difference_percent": duration_diff_pct,
    }));

    // Elevation comparison
    if let (Some(target_elev), Some(compare_elev)) = (target.elevation_gain, compare.elevation_gain)
    {
        let elev_diff_pct = ((target_elev - compare_elev) / compare_elev) * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "elevation_gain",
            "current": target_elev,
            "comparison": compare_elev,
            "difference_percent": elev_diff_pct,
        }));
    }

    // Power comparison (if available)
    if let (Some(target_power), Some(compare_power)) = (target.average_power, compare.average_power)
    {
        let power_diff_pct = ((f64::from(target_power) - f64::from(compare_power))
            / f64::from(compare_power))
            * 100.0;
        comparisons.push(serde_json::json!({
            "metric": "average_power",
            "current": target_power,
            "comparison": compare_power,
            "difference_percent": power_diff_pct,
            "improved": power_diff_pct > 0.0, // higher power = better
        }));
        add_power_insights(power_diff_pct, &mut insights);
    }

    if insights.is_empty() {
        insights.push("Metrics are similar to the comparison activity".to_owned());
    }

    serde_json::json!({
        "activity_id": target.id,
        "comparison_type": "specific_activity",
        "comparison_activity_id": compare_id,
        "comparison_activity_name": compare.name,
        "sport_type": format!("{:?}", target.sport_type),
        "comparisons": comparisons,
        "insights": insights,
    })
}

/// Helper to generate pace comparison insights
fn add_pace_insights(pace_diff_pct: f64, insights: &mut Vec<String>) {
    if pace_diff_pct < -5.0 {
        insights.push(format!(
            "Pace improved by {:.1}% compared to the selected activity",
            pace_diff_pct.abs()
        ));
    } else if pace_diff_pct > 5.0 {
        insights.push(format!(
            "Pace was {pace_diff_pct:.1}% slower than the selected activity"
        ));
    } else {
        insights.push("Pace was similar to the selected activity".to_owned());
    }
}

/// Helper to generate heart rate comparison insights
fn add_heart_rate_insights(hr_diff_pct: f64, insights: &mut Vec<String>) {
    if hr_diff_pct < -5.0 {
        insights.push("Heart rate efficiency improved - same effort at lower HR".to_owned());
    } else if hr_diff_pct > 5.0 {
        insights.push("Heart rate was higher - review pacing or recovery status".to_owned());
    }
}

/// Helper to generate power comparison insights
fn add_power_insights(power_diff_pct: f64, insights: &mut Vec<String>) {
    if power_diff_pct > 5.0 {
        insights.push(format!("Power output increased by {power_diff_pct:.1}%"));
    }
}

/// Check if two distances are similar (within 10%)
fn is_similar_distance(dist1: Option<f64>, dist2: Option<f64>) -> bool {
    match (dist1, dist2) {
        (Some(d1), Some(d2)) => {
            if d2 == 0.0 {
                return false;
            }
            let ratio = (d1 / d2 - 1.0).abs();
            ratio < 0.1 // within 10%
        }
        _ => false,
    }
}

/// Calculate pace in min/km
fn calculate_pace(activity: &crate::models::Activity) -> Option<f64> {
    if let Some(distance) = activity.distance_meters {
        if distance > 0.0 && activity.duration_seconds > 0 {
            #[allow(clippy::cast_precision_loss)]
            let seconds_per_km = (activity.duration_seconds as f64 / distance)
                * crate::constants::units::METERS_PER_KM;
            return Some(seconds_per_km / 60.0); // convert to min/km
        }
    }
    None
}

/// Calculate average pace from activities
fn calculate_average_pace(activities: &[&crate::models::Activity]) -> Option<f64> {
    let paces: Vec<f64> = activities
        .iter()
        .filter_map(|a| calculate_pace(a))
        .collect();
    if paces.is_empty() {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let avg = paces.iter().sum::<f64>() / paces.len() as f64;
    Some(avg)
}

/// Calculate average heart rate from activities
fn calculate_average_hr(activities: &[&crate::models::Activity]) -> Option<f64> {
    let hrs: Vec<f64> = activities
        .iter()
        .filter_map(|a| a.average_heart_rate.map(f64::from))
        .collect();
    if hrs.is_empty() {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let avg = hrs.iter().sum::<f64>() / hrs.len() as f64;
    Some(avg)
}

/// Calculate average elevation from activities
fn calculate_average_elevation(activities: &[&crate::models::Activity]) -> Option<f64> {
    let elevs: Vec<f64> = activities.iter().filter_map(|a| a.elevation_gain).collect();
    if elevs.is_empty() {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let avg = elevs.iter().sum::<f64>() / elevs.len() as f64;
    Some(avg)
}

// ============================================================================
// Helper Functions for Pattern Detection
// ============================================================================

/// Detect patterns in activity data based on pattern type
fn detect_activity_patterns(
    activities: &[crate::models::Activity],
    pattern_type: &str,
) -> serde_json::Value {
    use crate::intelligence::PatternDetector;

    if activities.len() < 3 {
        return serde_json::json!({
            "pattern_type": pattern_type,
            "activities_analyzed": activities.len(),
            "patterns_detected": [],
            "insights": ["Need at least 3 activities for pattern detection"],
            "confidence": "insufficient_data",
        });
    }

    match pattern_type {
        "training_blocks" => {
            format_hard_easy_pattern(&PatternDetector::detect_hard_easy_pattern(activities))
        }
        "progression" => {
            format_volume_progression(&PatternDetector::detect_volume_progression(activities))
        }
        "overtraining" => {
            format_overtraining_signals(&PatternDetector::detect_overtraining_signals(activities))
        }
        _ => format_weekly_schedule(&PatternDetector::detect_weekly_schedule(activities)), // default: weekly_schedule
    }
}

/// Format weekly schedule pattern results for JSON response
fn format_weekly_schedule(
    pattern: &crate::intelligence::WeeklySchedulePattern,
) -> serde_json::Value {
    use chrono::Weekday;

    // Convert Weekday enum to string
    let day_to_string = |weekday: &Weekday| -> &str {
        match weekday {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
    };

    // Build preferred days list with frequencies
    let preferred_days: Vec<serde_json::Value> = pattern
        .day_frequencies
        .iter()
        .map(|(day, &count)| {
            serde_json::json!({
                "day": day,
                "frequency": count,
            })
        })
        .collect();

    // Generate pattern descriptions
    let mut patterns = Vec::new();
    if pattern.consistency_score > 30.0 {
        patterns.push(format!(
            "Consistent weekly schedule detected: primarily trains on {}",
            pattern
                .most_common_days
                .iter()
                .map(day_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Determine confidence based on consistency score
    let confidence = if pattern.consistency_score > 40.0 {
        "high"
    } else if pattern.consistency_score > 20.0 {
        "medium"
    } else {
        "low"
    };

    serde_json::json!({
        "pattern_type": "weekly_schedule",
        "preferred_training_days": preferred_days,
        "patterns_detected": patterns,
        "insights": if patterns.is_empty() {
            vec!["No strong weekly schedule pattern detected - training is variable".to_owned()]
        } else {
            patterns.clone()
        },
        "consistency_score": pattern.consistency_score,
        "avg_activities_per_week": pattern.avg_activities_per_week,
        "confidence": confidence,
    })
}

/// Format hard/easy pattern results for JSON response
fn format_hard_easy_pattern(pattern: &crate::intelligence::HardEasyPattern) -> serde_json::Value {
    let mut insights = vec![pattern.pattern_description.clone()];

    if !pattern.adequate_recovery {
        insights.push("Consider adding more recovery days between hard efforts".to_owned());
    }

    let confidence = if pattern.pattern_detected {
        "medium"
    } else {
        "low"
    };

    serde_json::json!({
        "pattern_type": "training_blocks",
        "pattern_detected": pattern.pattern_detected,
        "intensity_distribution": {
            "hard_percentage": pattern.hard_percentage,
            "easy_percentage": pattern.easy_percentage,
        },
        "adequate_recovery": pattern.adequate_recovery,
        "patterns_detected": if pattern.pattern_detected {
            vec![pattern.pattern_description.clone()]
        } else {
            Vec::<String>::new()
        },
        "insights": insights,
        "confidence": confidence,
    })
}

/// Format volume progression pattern results for JSON response
fn format_volume_progression(
    pattern: &crate::intelligence::VolumeProgressionPattern,
) -> serde_json::Value {
    use crate::intelligence::VolumeTrend;

    let mut insights = Vec::new();
    let trend_description = match pattern.trend {
        VolumeTrend::Increasing => {
            insights.push("Volume is increasing - progressive overload detected".to_owned());
            "increasing"
        }
        VolumeTrend::Decreasing => {
            insights.push("Volume is decreasing - taper or recovery phase".to_owned());
            "decreasing"
        }
        VolumeTrend::Stable => {
            insights.push("Volume is stable - maintaining consistent training load".to_owned());
            "stable"
        }
    };

    if pattern.volume_spikes_detected {
        insights.push(format!(
            "Volume spikes detected in weeks: {} - monitor for injury risk",
            pattern
                .spike_weeks
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    serde_json::json!({
        "pattern_type": "progression",
        "trend": trend_description,
        "weekly_volumes": pattern.weekly_volumes,
        "week_numbers": pattern.week_numbers,
        "volume_spikes_detected": pattern.volume_spikes_detected,
        "spike_weeks": pattern.spike_weeks,
        "patterns_detected": insights.clone(),
        "insights": insights,
        "confidence": "medium",
    })
}

/// Format overtraining signals results for JSON response
fn format_overtraining_signals(
    signals: &crate::intelligence::OvertrainingSignals,
) -> serde_json::Value {
    use crate::intelligence::RiskLevel;

    let mut warning_signs = Vec::new();

    if signals.hr_drift_detected {
        if let Some(drift_pct) = signals.hr_drift_percent {
            warning_signs.push(format!(
                "Heart rate drift detected: {drift_pct:.1}% increase - possible fatigue"
            ));
        } else {
            warning_signs.push("Heart rate drift detected - possible fatigue".to_owned());
        }
    }

    if signals.performance_decline {
        warning_signs.push("Performance declining despite training - check recovery".to_owned());
    }

    if signals.insufficient_recovery {
        warning_signs.push("Insufficient recovery time between hard efforts".to_owned());
    }

    let risk_level_str = match signals.risk_level {
        RiskLevel::Low => "low",
        RiskLevel::Moderate => "moderate",
        RiskLevel::High => "high",
    };

    let recommendations = match signals.risk_level {
        RiskLevel::High => vec![
            "Take additional rest days",
            "Reduce training intensity and volume",
            "Focus on recovery and sleep quality",
            "Consider consulting with a coach or sports medicine professional",
        ],
        RiskLevel::Moderate => vec![
            "Monitor recovery closely",
            "Ensure adequate rest days",
            "Review training intensity distribution",
        ],
        RiskLevel::Low => vec![
            "Continue current training approach",
            "Maintain good recovery habits",
        ],
    };

    serde_json::json!({
        "pattern_type": "overtraining",
        "risk_level": risk_level_str,
        "warning_signs": warning_signs,
        "insights": if warning_signs.is_empty() {
            vec!["No significant overtraining signs detected - training load appears manageable".to_owned()]
        } else {
            warning_signs.clone()
        },
        "hr_drift_detected": signals.hr_drift_detected,
        "performance_decline": signals.performance_decline,
        "insufficient_recovery": signals.insufficient_recovery,
        "confidence": "medium",
        "recommendations": recommendations,
    })
}

// ============================================================================
// Helper Functions for Training Recommendations
// ============================================================================

/// Generate personalized training recommendations
fn generate_training_recommendations(
    activities: &[crate::models::Activity],
    recommendation_type: &str,
) -> serde_json::Value {
    if activities.is_empty() {
        return serde_json::json!({
            "recommendation_type": recommendation_type,
            "recommendations": ["Start with 2-3 easy activities per week to build base fitness"],
            "priority": "medium",
            "reasoning": "No recent training data available",
        });
    }

    // Filter to last 4 weeks for recommendation generation
    let four_weeks_ago = Utc::now() - Duration::days(28);
    let recent_activities: Vec<_> = activities
        .iter()
        .filter(|a| a.start_date >= four_weeks_ago)
        .cloned()
        .collect();

    if recent_activities.is_empty() {
        return serde_json::json!({
            "recommendation_type": recommendation_type,
            "recommendations": ["Resume training gradually - start with 2-3 easy sessions per week"],
            "priority": "high",
            "reasoning": "No training activity in the last 4 weeks",
        });
    }

    match recommendation_type {
        "training_plan" => generate_training_plan_recommendations(&recent_activities),
        "recovery" => generate_recovery_recommendations(&recent_activities),
        "intensity" => generate_intensity_recommendations(&recent_activities),
        "goal_specific" => generate_goal_specific_recommendations(&recent_activities),
        _ => generate_comprehensive_recommendations(&recent_activities),
    }
}

/// Generate weekly training plan recommendations using training load analysis
fn generate_training_plan_recommendations(
    activities: &[crate::models::Activity],
) -> serde_json::Value {
    use crate::intelligence::{PatternDetector, TrainingLoadCalculator};

    // Analyze volume progression to detect spikes
    let volume_pattern = PatternDetector::detect_volume_progression(activities);
    let weekly_schedule = PatternDetector::detect_weekly_schedule(activities);

    // Calculate training load metrics
    let calculator = TrainingLoadCalculator::new();
    let training_load = calculator
        .calculate_training_load(activities, None, None, None, None, None)
        .ok();

    let mut recommendations = Vec::new();
    let mut priority = "medium";
    let reasoning = if volume_pattern.volume_spikes_detected {
        recommendations.push(
            "Volume spike detected - reduce next week's volume by 10-15% to prevent injury"
                .to_owned(),
        );
        priority = "high";
        format!(
            "Training volume increased rapidly (spike detected in weeks: {})",
            volume_pattern
                .spike_weeks
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        String::from("Based on volume and consistency analysis")
    };

    // Training load recommendations
    if let Some(load) = &training_load {
        if load.atl > 150.0 {
            recommendations
                .push("Acute training load is very high - schedule a recovery week".to_owned());
            priority = "high";
        } else if load.ctl < 40.0 {
            recommendations
                .push("Build fitness gradually - increase weekly volume by 5-10%".to_owned());
        } else if load.ctl > 100.0 {
            recommendations.push(
                "Strong fitness base - maintain current volume and add quality work".to_owned(),
            );
        }
    }

    // Consistency recommendations
    if weekly_schedule.consistency_score < 20.0 {
        recommendations
            .push("Training schedule is inconsistent - aim for same days each week".to_owned());
        if weekly_schedule.avg_activities_per_week < 3.0 {
            recommendations.push("Increase frequency to 3-4 activities per week".to_owned());
            if priority == "medium" {
                priority = "high";
            }
        }
    } else if weekly_schedule.avg_activities_per_week > 6.0 {
        recommendations
            .push("Very high training frequency - ensure at least 1 complete rest day".to_owned());
    }

    // Provide structured weekly plan based on consistency
    let suggested_structure = if weekly_schedule.avg_activities_per_week < 3.0 {
        vec![serde_json::json!({
            "focus": "Build frequency",
            "sessions_per_week": 3,
            "key_workouts": ["Easy run", "Tempo run", "Long run"],
        })]
    } else if weekly_schedule.avg_activities_per_week <= 5.0 {
        vec![serde_json::json!({
            "focus": "Balanced training",
            "sessions_per_week": 4,
            "key_workouts": ["2 easy runs", "1 quality session (intervals/tempo)", "1 long run"],
        })]
    } else {
        vec![serde_json::json!({
            "focus": "High volume management",
            "sessions_per_week": "5-6",
            "key_workouts": ["Mostly easy runs (80%)", "1-2 quality sessions", "1 long run"],
        })]
    };

    serde_json::json!({
        "recommendation_type": "training_plan",
        "priority": priority,
        "reasoning": reasoning,
        "recommendations": recommendations,
        "suggested_structure": suggested_structure,
        "metrics": {
            "avg_activities_per_week": weekly_schedule.avg_activities_per_week,
            "consistency_score": weekly_schedule.consistency_score,
            "volume_spike_detected": volume_pattern.volume_spikes_detected,
            "ctl": training_load.as_ref().map(|l| l.ctl),
            "atl": training_load.as_ref().map(|l| l.atl),
        },
    })
}

/// Helper to process TSB status and add recommendations
fn process_tsb_recommendations(
    load: &crate::intelligence::training_load::TrainingLoad,
    recommendations: &mut Vec<String>,
    priority: &mut &str,
    recovery_status: &mut &str,
    reasoning: &mut String,
) {
    use crate::intelligence::{RiskLevel, TrainingLoadCalculator, TrainingStatus};

    let status = TrainingLoadCalculator::interpret_tsb(load.tsb);
    let recovery_days = TrainingLoadCalculator::recommend_recovery_days(load.tsb);

    match status {
        TrainingStatus::Overreaching => {
            recommendations.push(format!(
                "You're overreaching (TSB: {:.1}) - take {recovery_days} recovery days",
                load.tsb
            ));
            *priority = "high";
            *recovery_status = "overreaching";
            *reasoning = format!(
                "TSB is {:.1}, indicating deep fatigue requiring immediate recovery",
                load.tsb
            );
        }
        TrainingStatus::Productive => {
            recommendations
                .push("Good training zone - maintain current load with recovery days".to_owned());
            *recovery_status = "productive";
        }
        TrainingStatus::Fresh => {
            recommendations.push("Well-recovered - ready for quality training".to_owned());
            *recovery_status = "fresh";
        }
        TrainingStatus::Detraining => {
            recommendations
                .push("TSB is high - consider increasing training load gradually".to_owned());
            *recovery_status = "detraining_risk";
        }
    }

    // Check for overtraining risk
    let risk = TrainingLoadCalculator::check_overtraining_risk(load);
    if risk.risk_level == RiskLevel::High {
        *priority = "high";
        recommendations.push("High overtraining risk detected - prioritize recovery".to_owned());
        for factor in &risk.risk_factors {
            recommendations.push(format!("⚠️ {factor}"));
        }
    }
}

/// Generate recovery recommendations using TSB and overtraining signals
fn generate_recovery_recommendations(activities: &[crate::models::Activity]) -> serde_json::Value {
    use crate::intelligence::{PatternDetector, RiskLevel, TrainingLoadCalculator};

    // Calculate TSB (Training Stress Balance)
    let calculator = TrainingLoadCalculator::new();
    let training_load = calculator
        .calculate_training_load(activities, None, None, None, None, None)
        .ok();

    // Detect overtraining signals
    let overtraining_signals = PatternDetector::detect_overtraining_signals(activities);

    let mut recommendations = Vec::new();
    let mut priority = "medium";
    let mut recovery_status = "unknown";
    let mut reasoning = String::from("Based on training stress balance analysis");

    // TSB-based recovery recommendations (highest priority)
    if let Some(load) = &training_load {
        process_tsb_recommendations(
            load,
            &mut recommendations,
            &mut priority,
            &mut recovery_status,
            &mut reasoning,
        );
    }

    // Overtraining signal detection
    if overtraining_signals.hr_drift_detected {
        if let Some(drift_pct) = overtraining_signals.hr_drift_percent {
            recommendations.push(format!(
                "Heart rate drift detected ({drift_pct:.1}% increase) - sign of fatigue"
            ));
            if priority == "medium" {
                priority = "high";
            }
        }
    }

    if overtraining_signals.performance_decline {
        recommendations
            .push("Performance declining despite training - increase recovery".to_owned());
    }

    if overtraining_signals.insufficient_recovery {
        recommendations
            .push("Insufficient recovery between hard sessions - add easy days".to_owned());
    }

    // Provide recovery-specific tips based on status
    let recovery_actions = match recovery_status {
        "overreaching" => vec![
            "Take complete rest days",
            "Focus on sleep quality (8-9 hours)",
            "Light stretching or yoga only",
            "Monitor resting heart rate daily",
        ],
        "productive" => vec![
            "Include 1-2 easy recovery days per week",
            "Maintain 7-8 hours of sleep",
            "Active recovery (easy swimming/walking)",
        ],
        _ => vec![
            "Maintain current recovery routine",
            "7-9 hours of sleep per night",
            "Stay hydrated (2-3L water daily)",
        ],
    };

    serde_json::json!({
        "recommendation_type": "recovery",
        "priority": priority,
        "reasoning": reasoning,
        "recovery_status": recovery_status,
        "recommendations": recommendations,
        "recovery_actions": recovery_actions,
        "metrics": {
            "tsb": training_load.as_ref().map(|l| l.tsb),
            "ctl": training_load.as_ref().map(|l| l.ctl),
            "atl": training_load.as_ref().map(|l| l.atl),
            "hr_drift_detected": overtraining_signals.hr_drift_detected,
            "risk_level": match overtraining_signals.risk_level {
                RiskLevel::Low => "low",
                RiskLevel::Moderate => "moderate",
                RiskLevel::High => "high",
            },
        },
    })
}

/// Generate intensity recommendations using hard/easy pattern detection
fn generate_intensity_recommendations(activities: &[crate::models::Activity]) -> serde_json::Value {
    use crate::intelligence::PatternDetector;

    // Detect hard/easy pattern
    let pattern = PatternDetector::detect_hard_easy_pattern(activities);

    let mut recommendations = Vec::new();
    let mut priority = "medium";
    let mut reasoning = String::from("Based on intensity distribution analysis");

    // Check if pattern was detected
    if !pattern.pattern_detected {
        recommendations.push(
            "Unable to detect clear intensity pattern - ensure heart rate data is available"
                .to_owned(),
        );
        return serde_json::json!({
            "recommendation_type": "intensity",
            "priority": "low",
            "reasoning": "Insufficient heart rate data for analysis",
            "recommendations": recommendations,
        });
    }

    // Analyze 80/20 principle adherence
    let easy_pct = pattern.easy_percentage;

    if easy_pct < 70.0 {
        recommendations.push(
            "Too much high-intensity training - add more easy/recovery runs (aim for 80% easy)"
                .to_owned(),
        );
        priority = "high";
        reasoning = format!("Only {easy_pct:.0}% easy training detected - risk of overtraining");
    } else if easy_pct > 90.0 {
        recommendations.push(
            "Mostly easy training - include 1-2 quality sessions per week for fitness gains"
                .to_owned(),
        );
        priority = "medium";
    } else {
        recommendations.push("Good intensity balance following 80/20 principle".to_owned());
    }

    // Check recovery adequacy
    if pattern.adequate_recovery {
        recommendations.push("Good recovery pattern between hard sessions".to_owned());
    } else {
        recommendations.push("Consider adding more recovery days between hard sessions".to_owned());
    }

    // Specific workout recommendations based on hard percentage
    let hard_pct = pattern.hard_percentage;
    if hard_pct < 10.0 {
        recommendations.push("Add quality work:".to_owned());
        recommendations
            .push("  • Interval training: 6x800m @ 5K pace with 2min recovery".to_owned());
        recommendations.push("  • Tempo run: 20-30min @ comfortably hard pace".to_owned());
    } else if hard_pct > 30.0 {
        recommendations.push("Reduce high-intensity frequency to 1-2 sessions per week".to_owned());
        if priority != "high" {
            priority = "high";
        }
    }

    // Provide intensity zones guidance
    let intensity_guidance = if easy_pct < 70.0 {
        vec![
            "Most runs should be conversational pace",
            "Hard efforts should feel genuinely hard (8-9/10 effort)",
            "Recovery runs should be very easy (5-6/10 effort)",
        ]
    } else {
        vec![
            "Maintain mostly easy training",
            "Quality sessions: intervals, tempo, or threshold",
            "Allow 48h recovery after hard sessions",
        ]
    };

    serde_json::json!({
        "recommendation_type": "intensity",
        "priority": priority,
        "reasoning": reasoning,
        "recommendations": recommendations,
        "intensity_guidance": intensity_guidance,
        "metrics": {
            "pattern_detected": pattern.pattern_detected,
            "pattern_description": pattern.pattern_description,
            "hard_percentage": pattern.hard_percentage,
            "easy_percentage": pattern.easy_percentage,
            "adequate_recovery": pattern.adequate_recovery,
        },
    })
}

/// Generate goal-specific recommendations using performance prediction
fn generate_goal_specific_recommendations(
    activities: &[crate::models::Activity],
) -> serde_json::Value {
    use crate::intelligence::PerformancePredictor;
    use std::collections::HashMap;

    // Detect primary sport
    let mut sport_counts: HashMap<String, usize> = HashMap::new();
    for activity in activities {
        let sport = format!("{:?}", activity.sport_type);
        *sport_counts.entry(sport).or_insert(0) += 1;
    }

    let primary_sport = sport_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map_or("Unknown", |(sport, _)| sport.as_str());

    // Find best recent performance for predictions
    let best_performance = activities
        .iter()
        .filter(|a| a.distance_meters.is_some() && a.duration_seconds > 0)
        .filter_map(|a| {
            let distance = a.distance_meters?;
            #[allow(clippy::cast_precision_loss)]
            let time_secs = a.duration_seconds as f64;
            if distance > 3_000.0 && distance < 50_000.0 && time_secs > 0.0 {
                Some((distance, time_secs))
            } else {
                None
            }
        })
        .max_by(|a, b| {
            let pace_a = a.1 / (a.0 / crate::constants::limits::METERS_PER_KILOMETER);
            let pace_b = b.1 / (b.0 / crate::constants::limits::METERS_PER_KILOMETER);
            pace_b
                .partial_cmp(&pace_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let mut recommendations = Vec::new();
    let priority = "medium";
    let mut race_predictions = None;

    // Generate race time predictions if we have performance data
    if let Some((distance, time)) = best_performance {
        if let Ok(predictions) = PerformancePredictor::generate_race_predictions(distance, time) {
            race_predictions = Some(serde_json::json!({
                "based_on": format!("{:.1}km in {}", distance / crate::constants::limits::METERS_PER_KILOMETER, PerformancePredictor::format_time(time)),
                "vdot": predictions.vdot,
                "race_times": predictions.predictions,
            }));

            recommendations.push(format!(
                "Your VDOT is {:.1} - use this to set appropriate training paces",
                predictions.vdot
            ));
        }
    }

    // Sport-specific goal recommendations
    if primary_sport.contains("Run") {
        recommendations.push("Build aerobic base with easy long runs".to_owned());
        recommendations.push("Add weekly quality: tempo run or interval session".to_owned());
        recommendations.push("Include race-pace intervals 4-6 weeks before goal race".to_owned());
        recommendations.push("Taper 10-14 days before race: reduce volume 30-50%".to_owned());
    } else if primary_sport.contains("Ride") {
        recommendations.push("Build FTP with structured threshold intervals".to_owned());
        recommendations.push("Include weekly hill repeats for strength".to_owned());
        recommendations.push("Long endurance rides on weekends (3-5 hours)".to_owned());
    } else {
        recommendations.push("Focus on consistent training to build aerobic base".to_owned());
        recommendations.push("Gradually increase training volume by 5-10% per week".to_owned());
    }

    serde_json::json!({
        "recommendation_type": "goal_specific",
        "priority": priority,
        "reasoning": "Based on recent performance and sport type",
        "primary_sport": primary_sport,
        "recommendations": recommendations,
        "race_predictions": race_predictions,
        "periodization_phases": [
            "Base Phase: Build aerobic foundation (4-8 weeks)",
            "Build Phase: Add tempo and threshold work (4-6 weeks)",
            "Peak Phase: Race-specific intensity (2-3 weeks)",
            "Taper: Reduce volume, maintain sharpness (1-2 weeks)",
        ],
    })
}

/// Generate comprehensive recommendations combining all analyses
fn generate_comprehensive_recommendations(
    activities: &[crate::models::Activity],
) -> serde_json::Value {
    use crate::intelligence::{PatternDetector, TrainingLoadCalculator};

    // Comprehensive analysis using all available modules
    let calculator = TrainingLoadCalculator::new();
    let training_load = calculator
        .calculate_training_load(activities, None, None, None, None, None)
        .ok();

    let volume_pattern = PatternDetector::detect_volume_progression(activities);
    let intensity_pattern = PatternDetector::detect_hard_easy_pattern(activities);
    let overtraining = PatternDetector::detect_overtraining_signals(activities);

    let mut recommendations = Vec::new();
    let mut priority = "medium";
    let mut key_insights = Vec::new();

    // Training load insights
    if let Some(load) = &training_load {
        if load.tsb < -10.0 {
            recommendations.push(format!(
                "Immediate recovery needed - TSB is {:.1} (overreaching zone)",
                load.tsb
            ));
            priority = "high";
            key_insights.push("Fatigue is accumulating faster than fitness".to_owned());
        } else if load.ctl > 80.0 {
            key_insights.push(format!("Strong fitness base (CTL: {:.1})", load.ctl));
        } else if load.ctl < 40.0 {
            key_insights.push("Building fitness - continue gradual progression".to_owned());
        }
    }

    // Volume management
    if volume_pattern.volume_spikes_detected {
        recommendations
            .push("Reduce volume next week - spike detected in recent training".to_owned());
        if priority == "medium" {
            priority = "high";
        }
        key_insights.push("Training volume increased too rapidly".to_owned());
    }

    // Intensity balance
    if intensity_pattern.pattern_detected {
        let hard_pct = intensity_pattern.hard_percentage;
        if hard_pct > 30.0 {
            recommendations
                .push("Too much high-intensity work - add more easy training days".to_owned());
        } else if hard_pct < 10.0 {
            recommendations.push("Include 1-2 quality sessions per week".to_owned());
        }

        if intensity_pattern.adequate_recovery {
            key_insights.push("Good recovery pattern between hard sessions".to_owned());
        }
    }

    // Overtraining checks
    if overtraining.hr_drift_detected {
        recommendations
            .push("Heart rate drift detected - prioritize recovery this week".to_owned());
        key_insights.push("Possible fatigue accumulation detected".to_owned());
    }

    // General best practices if no specific issues
    if recommendations.is_empty() {
        recommendations.push("Training load is balanced - maintain current approach".to_owned());
        recommendations.push("Continue following 80/20 intensity distribution".to_owned());
        recommendations.push("Monitor weekly volume changes (keep under 10% increase)".to_owned());
    }

    // Add general best practices
    recommendations.push("Include 1-2 complete rest days per week".to_owned());
    recommendations.push("Prioritize sleep quality (7-9 hours per night)".to_owned());

    serde_json::json!({
        "recommendation_type": "comprehensive",
        "priority": priority,
        "reasoning": "Holistic analysis of training load, volume, and intensity patterns",
        "key_insights": key_insights,
        "recommendations": recommendations,
        "training_summary": {
            "activities_analyzed": activities.len(),
            "ctl": training_load.as_ref().map(|l| l.ctl),
            "atl": training_load.as_ref().map(|l| l.atl),
            "tsb": training_load.as_ref().map(|l| l.tsb),
            "volume_spike_detected": volume_pattern.volume_spikes_detected,
            "intensity_pattern_detected": intensity_pattern.pattern_detected,
            "overtraining_signals": overtraining.hr_drift_detected || overtraining.performance_decline,
        },
        "core_principles": {
            "consistency": "Regular training beats sporadic hard efforts",
            "recovery": "Fitness improves during rest, not during training",
            "progression": "Increase volume gradually (10% rule)",
            "intensity": "Follow 80/20 rule (80% easy, 20% hard)",
        },
    })
}

// ============================================================================
// Helper Functions for Fitness Score Calculation (CTL/ATL/TSS)
// ============================================================================

/// Calculate fitness metrics using CTL/ATL/TSS methodology
/// Calculate fitness metrics using proper 3-component formula with `TrainingLoadCalculator`
fn calculate_fitness_metrics(
    activities: &[crate::models::Activity],
    timeframe: &str,
) -> serde_json::Value {
    use crate::intelligence::TrainingLoadCalculator;
    use chrono::{Duration, Utc};

    if activities.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "fitness_score": 0,
            "level": "Beginner",
            "message": "No activities found for fitness calculation",
        });
    }

    // Filter activities by timeframe
    let now = Utc::now();
    let timeframe_days = match timeframe {
        "last_90_days" => 90,
        "all_time" => 365 * 10, // 10 years
        _ => 30,                // default to 30 days (includes "last_30_days")
    };

    let cutoff_date = now - Duration::days(timeframe_days);
    let filtered_activities: Vec<_> = activities
        .iter()
        .filter(|a| a.start_date >= cutoff_date)
        .cloned()
        .collect();

    if filtered_activities.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "fitness_score": 0,
            "level": "Beginner",
            "message": format!("No activities found in the last {timeframe_days} days"),
        });
    }

    // Component 1: CTL (Chronic Training Load) - 40% weight
    let calculator = TrainingLoadCalculator::new();
    let training_load = calculator
        .calculate_training_load(&filtered_activities, None, None, None, None, None)
        .ok();

    let ctl = training_load.as_ref().map_or(0.0, |l| l.ctl);
    let atl = training_load.as_ref().map_or(0.0, |l| l.atl);
    let tsb = training_load.as_ref().map_or(0.0, |l| l.tsb);

    // CTL component: normalize to 0-100 scale (150 CTL = 100 score)
    let ctl_score = (ctl / 150.0 * 100.0).min(100.0);

    // Component 2: Consistency (% weeks with 3+ activities) - 30% weight
    let consistency_score = calculate_consistency_score(&filtered_activities);

    // Component 3: Performance trend (pace improvement) - 30% weight
    let performance_score = calculate_performance_trend(&filtered_activities);

    // Combine components with weights: 40% CTL, 30% consistency, 30% performance
    let fitness_score =
        ctl_score.mul_add(0.4, consistency_score.mul_add(0.3, performance_score * 0.3));

    // Classify fitness level based on score
    let fitness_level = classify_fitness_level(fitness_score);

    // Determine trend (comparing first half vs second half)
    let trend = calculate_trend(&filtered_activities);

    #[allow(clippy::cast_possible_truncation)]
    let fitness_score_int = fitness_score.round() as i32;

    serde_json::json!({
        "timeframe": timeframe,
        "fitness_score": fitness_score_int,
        "level": fitness_level,
        "trend": trend,
        "components": {
            "ctl_score": ctl_score.round(),
            "consistency_score": consistency_score.round(),
            "performance_score": performance_score.round(),
        },
        "metrics": {
            "ctl": ctl.round(),
            "atl": atl.round(),
            "tsb": tsb.round(),
        },
        "activities_analyzed": filtered_activities.len(),
        "interpretation": {
            "ctl": "Chronic Training Load - long-term fitness (42-day average)",
            "consistency": "Training frequency and regularity",
            "performance": "Pace/speed improvement over time",
        },
    })
}

/// Calculate consistency score: percentage of weeks with 3+ activities
fn calculate_consistency_score(activities: &[crate::models::Activity]) -> f64 {
    use std::collections::HashMap;

    if activities.is_empty() {
        return 0.0;
    }

    // Get the date range
    let first_date = activities.iter().map(|a| a.start_date).min();
    let last_date = activities.iter().map(|a| a.start_date).max();

    let (Some(first), Some(last)) = (first_date, last_date) else {
        return 0.0;
    };

    // Calculate total weeks spanned
    let weeks_spanned = ((last - first).num_days() / 7).max(1);

    // Group activities by week number (days since first / 7)
    let mut activities_per_week: HashMap<i64, u32> = HashMap::new();

    for activity in activities {
        let days_since_first = (activity.start_date - first).num_days();
        let week_number = days_since_first / 7;
        *activities_per_week.entry(week_number).or_insert(0) += 1;
    }

    // Count weeks with 3+ activities
    let active_weeks = activities_per_week
        .values()
        .filter(|&&count| count >= 3)
        .count();

    // Calculate consistency score as percentage
    #[allow(clippy::cast_precision_loss)]
    let score = (active_weeks as f64 / weeks_spanned as f64) * 100.0;
    score.min(100.0)
}

/// Calculate performance trend: improvement in average pace over time
fn calculate_performance_trend(activities: &[crate::models::Activity]) -> f64 {
    let activities_with_pace: Vec<_> = activities
        .iter()
        .filter(|a| a.distance_meters.is_some() && a.duration_seconds > 0)
        .collect();

    if activities_with_pace.len() < 4 {
        return 50.0; // neutral score for insufficient data
    }

    // Split into first and second half
    let mid_point = activities_with_pace.len() / 2;
    let first_half = &activities_with_pace[..mid_point];
    let second_half = &activities_with_pace[mid_point..];

    // Calculate average pace for each half (seconds per meter)
    #[allow(clippy::cast_precision_loss)]
    let first_avg_pace: f64 = first_half
        .iter()
        .map(|a| {
            let distance = a.distance_meters.unwrap_or_else(|| {
                tracing::warn!(
                    activity_id = a.id,
                    "Activity missing distance_meters in pace calculation, using 1.0m fallback"
                );
                1.0
            });
            a.duration_seconds as f64 / distance
        })
        .sum::<f64>()
        / first_half.len() as f64;

    #[allow(clippy::cast_precision_loss)]
    let second_avg_pace: f64 = second_half
        .iter()
        .map(|a| {
            let distance = a.distance_meters.unwrap_or_else(|| {
                tracing::warn!(
                    activity_id = a.id,
                    "Activity missing distance_meters in pace calculation, using 1.0m fallback"
                );
                1.0
            });
            a.duration_seconds as f64 / distance
        })
        .sum::<f64>()
        / second_half.len() as f64;

    // Lower pace is better (faster), so improvement is first_pace - second_pace
    let pace_improvement_pct = ((first_avg_pace - second_avg_pace) / first_avg_pace) * 100.0;

    // Map improvement to 0-100 score
    // -10% to +10% improvement maps to 0-100
    let score = (pace_improvement_pct + 10.0) * 5.0;
    score.clamp(0.0, 100.0)
}

/// Classify fitness level based on composite score (0-100)
fn classify_fitness_level(score: f64) -> &'static str {
    if score >= 80.0 {
        "Excellent"
    } else if score >= 60.0 {
        "Good"
    } else if score >= 40.0 {
        "Moderate"
    } else if score >= 20.0 {
        "Developing"
    } else {
        "Beginner"
    }
}

/// Calculate fitness trend by comparing recent vs older activities
fn calculate_trend(activities: &[crate::models::Activity]) -> &'static str {
    if activities.len() < 4 {
        return "stable";
    }

    let mid_point = activities.len() / 2;
    let older_half = &activities[..mid_point];
    let recent_half = &activities[mid_point..];

    #[allow(clippy::cast_precision_loss)]
    let older_avg_duration =
        older_half.iter().map(|a| a.duration_seconds).sum::<u64>() as f64 / older_half.len() as f64;

    #[allow(clippy::cast_precision_loss)]
    let recent_avg_duration = recent_half.iter().map(|a| a.duration_seconds).sum::<u64>() as f64
        / recent_half.len() as f64;

    let change_pct = ((recent_avg_duration - older_avg_duration) / older_avg_duration) * 100.0;

    if change_pct > 15.0 {
        "improving"
    } else if change_pct < -15.0 {
        "declining"
    } else {
        "stable"
    }
}

// ============================================================================
// Helper Functions for Performance Prediction (VDOT/Riegel)
// ============================================================================

/// Predict race performance using VDOT and Riegel formulas
/// Predict race performance using VDOT methodology from `PerformancePredictor`
fn predict_race_performance(
    activities: &[crate::models::Activity],
    target_sport: &str,
) -> serde_json::Value {
    use crate::intelligence::PerformancePredictor;

    // Filter activities by sport type
    let running_activities: Vec<&crate::models::Activity> = activities
        .iter()
        .filter(|a| format!("{:?}", a.sport_type).contains("Run"))
        .collect();

    if running_activities.is_empty() {
        return serde_json::json!({
            "target_sport": target_sport,
            "message": "No running activities found for prediction",
            "predictions": {},
        });
    }

    // Find best recent performance using PerformancePredictor
    let owned_activities: Vec<_> = running_activities.iter().copied().cloned().collect();
    let Some(best_activity) = PerformancePredictor::find_best_performance(&owned_activities) else {
        return serde_json::json!({
            "target_sport": target_sport,
            "message": "No suitable activities found for prediction (need distance > 3km with valid time)",
            "predictions": {},
        });
    };

    let best_distance = best_activity.distance_meters.unwrap_or_else(|| {
        tracing::warn!(
            activity_id = best_activity.id,
            "Best activity missing distance_meters despite find_best_performance validation, using 0.0m"
        );
        0.0
    });
    #[allow(clippy::cast_precision_loss)]
    let best_time = best_activity.duration_seconds as f64;

    // Generate race predictions using PerformancePredictor (includes VDOT calculation)
    match PerformancePredictor::generate_race_predictions(best_distance, best_time) {
        Ok(race_predictions) => {
            // Calculate confidence based on data quality
            let confidence =
                calculate_prediction_confidence(&running_activities, &best_activity.start_date);

            // Convert predictions HashMap to JSON array format for consistency
            let predictions_array: Vec<serde_json::Value> = race_predictions
                .predictions
                .iter()
                .map(|(name, time_seconds)| {
                    let distance_meters = match name.as_str() {
                        "5K" => 5_000.0,
                        "10K" => 10_000.0,
                        "Half Marathon" => 21_097.5,
                        "Marathon" => 42_195.0,
                        _ => 0.0,
                    };
                    let pace_per_km = if distance_meters > 0.0 {
                        PerformancePredictor::format_pace_per_km(distance_meters / time_seconds)
                    } else {
                        "N/A".to_owned()
                    };

                    serde_json::json!({
                        "distance": name,
                        "distance_meters": distance_meters,
                        "predicted_time_seconds": time_seconds.round(),
                        "predicted_time_formatted": PerformancePredictor::format_time(*time_seconds),
                        "predicted_pace_min_km": pace_per_km,
                    })
                })
                .collect();

            serde_json::json!({
                "target_sport": target_sport,
                "vdot": race_predictions.vdot.round(),
                "best_performance": {
                    "distance_meters": best_distance,
                    "time_seconds": best_time,
                    "pace_min_km": PerformancePredictor::format_pace_per_km(best_distance / best_time),
                    "date": best_activity.start_date.to_rfc3339(),
                },
                "predictions": predictions_array,
                "confidence": confidence,
                "activities_analyzed": running_activities.len(),
                "notes": [
                    "Predictions assume proper race preparation and taper",
                    "Based on VDOT methodology by Jack Daniels",
                    "Actual performance may vary with conditions and training",
                ],
            })
        }
        // Error handling for generate_race_predictions failure
        Err(e) => {
            serde_json::json!({
                "target_sport": target_sport,
                "error": format!("Failed to generate predictions: {}", e),
                "predictions": [],
                "message": "Unable to calculate race predictions from available data",
            })
        }
    }
}

/// Calculate prediction confidence based on recency, training volume, and data quality
///
/// Confidence factors per B6 roadmap:
/// - Recency of best performance (< 30 days = high confidence)
/// - Training volume (high CTL = more confidence)
/// - Number of recent races and consistency
#[allow(clippy::cast_precision_loss, clippy::bool_to_int_with_if)] // Multi-level threshold scoring, not simple boolean conversion
fn calculate_prediction_confidence(
    activities: &[&crate::models::Activity],
    best_activity_date: &chrono::DateTime<chrono::Utc>,
) -> String {
    use crate::intelligence::TrainingLoadCalculator;
    use chrono::Utc;

    // Factor 1: Recency (< 30 days = high confidence)
    let days_since_best = (Utc::now() - *best_activity_date).num_days();
    let recency_score = if days_since_best < 30 {
        2 // Recent performance
    } else if days_since_best < 90 {
        1 // Moderately recent
    } else {
        0 // Old performance
    };

    // Factor 2: Training volume (CTL)
    let owned_activities: Vec<_> = activities.iter().copied().cloned().collect();
    let calculator = TrainingLoadCalculator::new();
    let ctl_score = if let Ok(training_load) =
        calculator.calculate_training_load(&owned_activities, None, None, None, None, None)
    {
        if training_load.ctl > 80.0 {
            2 // High training load
        } else if training_load.ctl > 40.0 {
            1 // Moderate training load
        } else {
            0 // Low training load
        }
    } else {
        0
    };

    // Factor 3: Number of activities
    let volume_score = if activities.len() >= 20 {
        2
    } else if activities.len() >= 10 {
        1
    } else {
        0
    };

    // Combine factors (max score = 6)
    let total_score = recency_score + ctl_score + volume_score;

    if total_score >= 5 {
        "high".to_owned()
    } else if total_score >= 3 {
        "medium".to_owned()
    } else {
        "low".to_owned()
    }
}

// ============================================================================
// Helper Functions for Training Load Analysis
// ============================================================================

/// Analyze training load with detailed TSS/CTL/ATL/TSB metrics
fn analyze_detailed_training_load(
    activities: &[crate::models::Activity],
    timeframe: &str,
) -> serde_json::Value {
    use crate::intelligence::TrainingLoadCalculator;

    if activities.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "message": "No activities found for training load analysis",
        });
    }

    // Use TrainingLoadCalculator from Phase 1 foundation
    let calculator = TrainingLoadCalculator::new();

    // Calculate training load (CTL, ATL, TSB) using real TSS calculation
    // Note: For accurate TSS, we'd need user's FTP, LTHR, max_hr, etc.
    // For now, use None values which will trigger estimation
    let Ok(training_load) = calculator.calculate_training_load(
        activities, None, // FTP
        None, // LTHR
        None, // max_hr
        None, // resting_hr
        None, // weight_kg
    ) else {
        return serde_json::json!({
            "timeframe": timeframe,
            "message": "Unable to calculate training load - insufficient activity data",
        });
    };

    let ctl = training_load.ctl;
    let atl = training_load.atl;
    let tsb = training_load.tsb;

    // Calculate weekly TSS totals from TSS history
    let weekly_tss = calculate_weekly_tss_from_history(&training_load.tss_history);

    // Determine load status
    let load_status = determine_load_status(ctl, atl, tsb);

    // Check for overtraining risk
    let overtraining_risk = if tsb < -30.0 {
        "high"
    } else if tsb < -20.0 {
        "moderate"
    } else {
        "low"
    };

    // Taper recommendations
    let taper_recommendation = if tsb > 10.0 {
        "Well tapered - ready for peak performance"
    } else if tsb > 0.0 {
        "Good taper status"
    } else if tsb > -10.0 {
        "Consider light taper for upcoming events"
    } else {
        "Significant taper needed before racing"
    };

    // Periodization suggestions
    let mut periodization_suggestions = Vec::new();
    if atl > ctl * 1.5 {
        periodization_suggestions
            .push("Recent spike in training - allow adaptation time".to_owned());
    }
    if ctl < 30.0 {
        periodization_suggestions
            .push("Building base - focus on consistency and volume".to_owned());
    } else if ctl > 80.0 {
        periodization_suggestions
            .push("High fitness level - maintain or add recovery weeks".to_owned());
    }

    serde_json::json!({
        "timeframe": timeframe,
        "load_metrics": {
            "ctl": ctl.round(),
            "atl": atl.round(),
            "tsb": tsb.round(),
            "weekly_tss": weekly_tss,
        },
        "load_status": load_status,
        "overtraining_risk": overtraining_risk,
        "taper_status": taper_recommendation,
        "periodization_suggestions": periodization_suggestions,
        "training_zones": classify_training_load(ctl),
        "recommendations": generate_load_recommendations(ctl, atl, tsb),
        "activities_analyzed": training_load.tss_history.len(),
        "interpretation": {
            "ctl": "Chronic Training Load - fitness level (42-day average TSS)",
            "atl": "Acute Training Load - fatigue level (7-day average TSS)",
            "tsb": "Training Stress Balance - form indicator (CTL - ATL)",
            "positive_tsb": "Fresh and recovered, ready for hard training",
            "negative_tsb": "Fatigued, prioritize recovery",
        },
    })
}

/// Calculate weekly TSS totals from `TssDataPoint` history (Phase 1 format)
fn calculate_weekly_tss_from_history(
    tss_history: &[crate::intelligence::TssDataPoint],
) -> Vec<serde_json::Value> {
    use std::collections::HashMap;

    if tss_history.is_empty() {
        return Vec::new();
    }

    // Group by week
    let mut weekly_totals: HashMap<i32, f64> = HashMap::new();
    let first_date = tss_history[0].date;

    for point in tss_history {
        let days_diff = (point.date - first_date).num_days();
        #[allow(clippy::cast_possible_truncation)]
        let week_number_i32 = (days_diff / 7) as i32;
        *weekly_totals.entry(week_number_i32).or_insert(0.0) += point.tss;
    }

    // Convert to sorted vec
    let mut weeks: Vec<(i32, f64)> = weekly_totals.into_iter().collect();
    weeks.sort_by_key(|(week, _)| *week);

    weeks
        .iter()
        .map(|(week, tss)| {
            serde_json::json!({
                "week": week,
                "total_tss": tss.round(),
            })
        })
        .collect()
}

/// Determine overall load status
fn determine_load_status(_ctl: f64, _atl: f64, tsb: f64) -> String {
    if tsb < -25.0 {
        "Overreached - high fatigue".to_owned()
    } else if tsb < -10.0 {
        "Productive - building fitness under fatigue".to_owned()
    } else if tsb < 5.0 {
        "Balanced - good training stress balance".to_owned()
    } else if tsb < 15.0 {
        "Fresh - ready for quality work".to_owned()
    } else {
        "Very fresh - possibly detraining".to_owned()
    }
}

/// Classify training load level
fn classify_training_load(ctl: f64) -> serde_json::Value {
    let level = if ctl < 25.0 {
        "Beginner"
    } else if ctl < 45.0 {
        "Intermediate"
    } else if ctl < 70.0 {
        "Advanced"
    } else if ctl < 100.0 {
        "Elite"
    } else {
        "Very High"
    };

    serde_json::json!({
        "level": level,
        "ctl_range": match level {
            "Beginner" => "< 25",
            "Intermediate" => "25-45",
            "Advanced" => "45-70",
            "Elite" => "70-100",
            _ => "> 100",
        },
    })
}

/// Generate load-specific recommendations
fn generate_load_recommendations(ctl: f64, atl: f64, tsb: f64) -> Vec<String> {
    let mut recommendations = Vec::new();

    // TSB-based recommendations
    if tsb < -25.0 {
        recommendations.push("⚠️ Critical fatigue - take 2-3 rest days immediately".to_owned());
        recommendations.push("Reduce training volume by 50% this week".to_owned());
    } else if tsb < -15.0 {
        recommendations.push("High fatigue - schedule recovery week".to_owned());
        recommendations.push("Reduce intensity and add extra rest day".to_owned());
    } else if tsb < -5.0 {
        recommendations
            .push("Moderate fatigue - maintain current load or slight reduction".to_owned());
    } else if tsb > 15.0 {
        recommendations.push("Very fresh - good time for breakthrough workout or race".to_owned());
    }

    // CTL/ATL ratio analysis
    let ratio = if ctl > 0.0 { atl / ctl } else { 0.0 };
    if ratio > 1.5 {
        recommendations
            .push("Recent training spike detected - allow 1-2 weeks adaptation".to_owned());
    } else if ratio < 0.8 && ctl > 30.0 {
        recommendations.push("Well adapted to training - can increase load gradually".to_owned());
    }

    // Progressive load recommendations
    if ctl < 30.0 {
        recommendations.push("Build weekly TSS by 3-5 points per week".to_owned());
    } else if ctl > 80.0 {
        recommendations
            .push("High load - incorporate recovery weeks (reduce by 20-30%)".to_owned());
    }

    if recommendations.is_empty() {
        recommendations
            .push("Training load is well balanced - maintain current approach".to_owned());
    }

    recommendations
}
