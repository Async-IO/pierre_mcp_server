// ABOUTME: Intelligence and analysis handlers with clean separation
// ABOUTME: AI-powered analysis tools that delegate to intelligence services
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::future::Future;
use std::pin::Pin;

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
        ProtocolError::InvalidRequest("activity parameter is required".to_string())
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
                (Some(_), _) => "provided".to_string(),
                (None, Some(age)) => format!("calculated_from_age_{age}"),
                (None, None) => "default_assumed".to_string(),
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
    let mut insights = Vec::new();
    let mut recommendations = Vec::new();

    // Analyze distance
    if let Some(distance) = activity.distance_meters {
        let km = distance / 1000.0;
        insights.push(format!("Activity covered {km:.2} km"));
        if km > 10.0 {
            recommendations.push("Great long-distance effort! Ensure proper recovery time");
        }
    }

    // Analyze elevation
    if let Some(elevation) = activity.elevation_gain {
        insights.push(format!("Total elevation gain: {elevation:.0} meters"));
        if elevation > 500.0 {
            recommendations.push("Significant elevation - consider targeted hill training");
        }
    }

    // Analyze heart rate
    if let Some(avg_hr) = activity.average_heart_rate {
        insights.push(format!("Average heart rate: {avg_hr} bpm"));
        if avg_hr > 160 {
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
                "distance_km": activity.distance_meters.map(|d| d / 1000.0),
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
        "activity_id".to_string(),
        serde_json::Value::String(activity_id.to_string()),
    );
    metadata.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_uuid.to_string()),
    );
    metadata.insert(
        "tenant_id".to_string(),
        tenant_id.map_or(serde_json::Value::Null, serde_json::Value::String),
    );
    metadata.insert(
        "analysis_type".to_string(),
        serde_json::Value::String("intelligence".to_string()),
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
                ProtocolError::InvalidRequest("Missing required parameter: activity_id".to_string())
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
                        .map(str::to_string)
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
                    "authentication_required".to_string(),
                    serde_json::Value::Bool(true),
                );
                Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string(),
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
                        .map(str::to_string)
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
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
                ProtocolError::InvalidRequest("Missing required parameter: activity_id".to_string())
            })?;
        let comparison_type = request
            .parameters
            .get("comparison_type")
            .and_then(|v| v.as_str())
            .unwrap_or("similar_activities");

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
                        .map(str::to_string)
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
                        );

                        Ok(UniversalResponse {
                            success: true,
                            result: Some(comparison),
                            error: None,
                            metadata: Some({
                                let mut map = std::collections::HashMap::new();
                                map.insert(
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
                ProtocolError::InvalidRequest(
                    "Missing required parameter: pattern_type".to_string(),
                )
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
                        .map(str::to_string)
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
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
                        .map(str::to_string)
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
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
                        .map(str::to_string)
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
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
                        .map(str::to_string)
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
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
                        .map(str::to_string)
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
                                    "user_id".to_string(),
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
                error: Some("No valid Strava token found. Please connect your Strava account using the connect_provider tool with provider='strava'.".to_string()),
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
    use chrono::{DateTime, Utc};

    if activities.is_empty() {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "no_data",
            "activities_analyzed": 0,
            "insights": ["No activities found for analysis"]
        });
    }

    // Filter activities by timeframe
    let cutoff_date = calculate_cutoff_date(timeframe);
    let filtered_activities: Vec<&crate::models::Activity> = activities
        .iter()
        .filter(|a| a.start_date >= cutoff_date)
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

    // Extract metric values with timestamps
    let data_points: Vec<(DateTime<Utc>, f64)> = filtered_activities
        .iter()
        .filter_map(|activity| {
            extract_metric_value(activity, metric).map(|value| (activity.start_date, value))
        })
        .collect();

    if data_points.len() < 2 {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "insufficient_data",
            "activities_analyzed": filtered_activities.len(),
            "insights": [format!("Metric '{}' not available in enough activities", metric)]
        });
    }

    // Perform linear regression
    let regression_result = calculate_linear_regression(&data_points);

    // Calculate moving average (7-day window)
    let moving_avg = calculate_moving_average(&data_points, 7);

    // Determine trend direction and confidence
    let trend_direction =
        determine_trend_direction(regression_result.slope, regression_result.r_squared);

    // Generate insights
    let insights = generate_trend_insights(
        metric,
        &trend_direction,
        regression_result.slope,
        regression_result.r_squared,
        &data_points,
    );

    serde_json::json!({
        "metric": metric,
        "timeframe": timeframe,
        "trend": trend_direction,
        "activities_analyzed": data_points.len(),
        "statistics": {
            "slope": regression_result.slope,
            "r_squared": regression_result.r_squared,
            "confidence": regression_result.r_squared,
            "moving_average_7day": moving_avg,
            "start_value": data_points.first().map(|(_, v)| v),
            "end_value": data_points.last().map(|(_, v)| v),
            "percent_change": calculate_percent_change(&data_points),
        },
        "insights": insights,
    })
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

/// Extract metric value from activity
fn extract_metric_value(activity: &crate::models::Activity, metric: &str) -> Option<f64> {
    match metric {
        "pace" => {
            // pace in min/km
            if let Some(distance) = activity.distance_meters {
                if distance > 0.0 && activity.duration_seconds > 0 {
                    #[allow(clippy::cast_precision_loss)]
                    let seconds_per_km = (activity.duration_seconds as f64 / distance) * 1000.0;
                    return Some(seconds_per_km / 60.0); // convert to min/km
                }
            }
            None
        }
        "speed" => activity.average_speed,
        "distance" => activity.distance_meters,
        "duration" => {
            #[allow(clippy::cast_precision_loss)]
            let duration_f64 = activity.duration_seconds as f64;
            Some(duration_f64)
        }
        "heart_rate" | "hr" => activity.average_heart_rate.map(f64::from),
        "power" => activity.average_power.map(f64::from),
        "elevation" => activity.elevation_gain,
        _ => None,
    }
}

#[derive(Debug)]
struct RegressionResult {
    slope: f64,
    r_squared: f64,
}

/// Calculate linear regression on time-series data
fn calculate_linear_regression(data: &[(chrono::DateTime<chrono::Utc>, f64)]) -> RegressionResult {
    if data.len() < 2 {
        return RegressionResult {
            slope: 0.0,
            r_squared: 0.0,
        };
    }

    // Convert timestamps to days since first activity
    let first_timestamp = data[0].0.timestamp();
    #[allow(clippy::cast_precision_loss)]
    let x_values: Vec<f64> = data
        .iter()
        .map(|(date, _)| (date.timestamp() - first_timestamp) as f64 / 86400.0) // days
        .collect();
    let y_values: Vec<f64> = data.iter().map(|(_, value)| *value).collect();

    #[allow(clippy::cast_precision_loss)]
    let n = x_values.len() as f64;

    // Calculate means
    let x_mean = x_values.iter().sum::<f64>() / n;
    let y_mean = y_values.iter().sum::<f64>() / n;

    // Calculate slope and intercept
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for (x, y) in x_values.iter().zip(y_values.iter()) {
        numerator += (x - x_mean) * (y - y_mean);
        denominator += (x - x_mean) * (x - x_mean);
    }

    let slope = if denominator.abs() > f64::EPSILON {
        numerator / denominator
    } else {
        0.0
    };

    // Calculate R²
    let y_variance: f64 = y_values.iter().map(|y| (y - y_mean).powi(2)).sum();
    let residual_variance: f64 = x_values
        .iter()
        .zip(y_values.iter())
        .map(|(x, y)| {
            let predicted = slope.mul_add(x - x_mean, y_mean);
            (y - predicted).powi(2)
        })
        .sum();

    let r_squared = if y_variance.abs() > f64::EPSILON {
        1.0 - (residual_variance / y_variance)
    } else {
        0.0
    };

    RegressionResult {
        slope,
        r_squared: r_squared.clamp(0.0, 1.0),
    }
}

/// Calculate moving average over window
fn calculate_moving_average(
    data: &[(chrono::DateTime<chrono::Utc>, f64)],
    _window_days: u32,
) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    // Simple average for now (could be enhanced to use actual window)
    let sum: f64 = data.iter().map(|(_, v)| v).sum();
    #[allow(clippy::cast_precision_loss)]
    let avg = sum / data.len() as f64;
    avg
}

/// Determine trend direction from slope and confidence
fn determine_trend_direction(slope: f64, r_squared: f64) -> String {
    const IMPROVEMENT_THRESHOLD: f64 = 0.01;
    const CONFIDENCE_THRESHOLD: f64 = 0.3;

    if r_squared < CONFIDENCE_THRESHOLD {
        return "stable".to_string();
    }

    if slope.abs() < IMPROVEMENT_THRESHOLD {
        "stable".to_string()
    } else if slope > 0.0 {
        "improving".to_string()
    } else {
        "declining".to_string()
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
                insights.push("Strong consistent improvement trend detected".to_string());
            }
        }
        "declining" => {
            insights.push(format!(
                "Your {} is declining with {:.1}% confidence",
                metric,
                r_squared * 100.0
            ));
            if slope < -0.05 {
                insights.push("Consider reviewing your training plan or recovery".to_string());
            }
        }
        "stable" => {
            if r_squared < 0.3 {
                insights.push(
                    "Performance is variable - maintain consistency for clearer trends".to_string(),
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
) -> serde_json::Value {
    match comparison_type {
        "pr_comparison" => compare_with_personal_records(target, all_activities),
        "specific_activity" => {
            // For specific activity, we'd need a second activity_id parameter
            // For now, compare with most similar recent activity
            compare_with_similar_activities(target, all_activities)
        }
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
            insights.push("Heart rate efficiency improved - same effort at lower HR".to_string());
        } else if hr_diff_pct > 5.0 {
            insights.push("Heart rate was higher - consider recovery or pacing".to_string());
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
                insights.push("New distance PR! 🎉".to_string());
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
            insights.push("New pace PR! 🚀".to_string());
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
                insights.push("New power PR! 💪".to_string());
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

/// Check if two distances are similar (within 20%)
fn is_similar_distance(dist1: Option<f64>, dist2: Option<f64>) -> bool {
    match (dist1, dist2) {
        (Some(d1), Some(d2)) => {
            if d2 == 0.0 {
                return false;
            }
            let ratio = (d1 / d2 - 1.0).abs();
            ratio < 0.2 // within 20%
        }
        _ => false,
    }
}

/// Calculate pace in min/km
fn calculate_pace(activity: &crate::models::Activity) -> Option<f64> {
    if let Some(distance) = activity.distance_meters {
        if distance > 0.0 && activity.duration_seconds > 0 {
            #[allow(clippy::cast_precision_loss)]
            let seconds_per_km = (activity.duration_seconds as f64 / distance) * 1000.0;
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
        "training_blocks" => detect_training_blocks(activities),
        "progression" => detect_progression_pattern(activities),
        "overtraining" => detect_overtraining_signs(activities),
        _ => detect_weekly_schedule_pattern(activities), // default
    }
}

/// Detect weekly training schedule patterns (which days user trains)
fn detect_weekly_schedule_pattern(activities: &[crate::models::Activity]) -> serde_json::Value {
    use chrono::Datelike;
    use std::collections::HashMap;

    // Count activities by day of week
    let mut day_counts: HashMap<u32, usize> = HashMap::new();
    for activity in activities {
        let weekday = activity.start_date.weekday().num_days_from_monday();
        *day_counts.entry(weekday).or_insert(0) += 1;
    }

    // Find preferred training days
    let total = activities.len();
    let mut preferred_days = Vec::new();
    let mut patterns = Vec::new();

    for (day, count) in &day_counts {
        #[allow(clippy::cast_precision_loss)]
        let percentage = (*count as f64 / total as f64) * 100.0;
        if percentage > 20.0 {
            // Trains on this day more than 20% of the time
            let day_name = match day {
                0 => "Monday",
                1 => "Tuesday",
                2 => "Wednesday",
                3 => "Thursday",
                4 => "Friday",
                5 => "Saturday",
                6 => "Sunday",
                _ => "Unknown",
            };
            preferred_days.push(serde_json::json!({
                "day": day_name,
                "frequency": count,
                "percentage": percentage,
            }));
        }
    }

    // Detect patterns
    if preferred_days.len() >= 3 && preferred_days.len() <= 5 {
        patterns.push("Consistent weekly training schedule detected".to_string());
    }

    let weekend_count = day_counts.get(&5).unwrap_or(&0) + day_counts.get(&6).unwrap_or(&0);
    #[allow(clippy::cast_precision_loss)]
    let weekend_pct = (weekend_count as f64 / total as f64) * 100.0;
    if weekend_pct > 40.0 {
        patterns.push("Weekend warrior pattern - most training on weekends".to_string());
    }

    let weekday_count = (0..5)
        .map(|d| day_counts.get(&d).unwrap_or(&0))
        .sum::<usize>();
    #[allow(clippy::cast_precision_loss)]
    let weekday_pct = (weekday_count as f64 / total as f64) * 100.0;
    if weekday_pct > 70.0 {
        patterns.push("Weekday training pattern - primarily trains during work week".to_string());
    }

    serde_json::json!({
        "pattern_type": "weekly_schedule",
        "activities_analyzed": activities.len(),
        "preferred_training_days": preferred_days,
        "patterns_detected": patterns,
        "insights": if patterns.is_empty() {
            vec!["No strong weekly schedule pattern detected - training is variable".to_string()]
        } else {
            patterns.clone()
        },
        "confidence": if preferred_days.len() >= 2 { "high" } else { "low" },
    })
}

/// Detect hard/easy training blocks
fn detect_training_blocks(activities: &[crate::models::Activity]) -> serde_json::Value {
    if activities.len() < 7 {
        return serde_json::json!({
            "pattern_type": "training_blocks",
            "activities_analyzed": activities.len(),
            "insights": ["Need at least 7 activities to detect training block patterns"],
            "confidence": "insufficient_data",
        });
    }

    // Calculate intensity for each activity (based on HR if available)
    let intensities: Vec<(String, f64)> = activities
        .iter()
        .filter_map(|a| a.average_heart_rate.map(|hr| (a.id.clone(), f64::from(hr))))
        .collect();

    if intensities.len() < 5 {
        return serde_json::json!({
            "pattern_type": "training_blocks",
            "activities_analyzed": activities.len(),
            "insights": ["Need heart rate data for training block detection"],
            "confidence": "insufficient_data",
        });
    }

    // Calculate average intensity
    #[allow(clippy::cast_precision_loss)]
    let avg_intensity =
        intensities.iter().map(|(_, hr)| hr).sum::<f64>() / intensities.len() as f64;

    // Detect hard/easy patterns
    let mut patterns = Vec::new();
    let mut hard_count = 0;
    let mut easy_count = 0;

    for (_, hr) in &intensities {
        if *hr > avg_intensity * 1.1 {
            hard_count += 1;
        } else if *hr < avg_intensity * 0.9 {
            easy_count += 1;
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let hard_pct = (f64::from(hard_count) / intensities.len() as f64) * 100.0;
    #[allow(clippy::cast_precision_loss)]
    let easy_pct = (f64::from(easy_count) / intensities.len() as f64) * 100.0;

    if hard_pct > 30.0 && easy_pct > 30.0 {
        patterns.push("Good hard/easy training balance detected".to_string());
    } else if hard_pct > 50.0 {
        patterns
            .push("High intensity pattern - consider adding more recovery workouts".to_string());
    } else if easy_pct > 70.0 {
        patterns.push("Low intensity pattern - consider adding harder efforts".to_string());
    }

    serde_json::json!({
        "pattern_type": "training_blocks",
        "activities_analyzed": intensities.len(),
        "intensity_distribution": {
            "hard_workouts": hard_count,
            "easy_workouts": easy_count,
            "hard_percentage": hard_pct,
            "easy_percentage": easy_pct,
        },
        "patterns_detected": patterns.clone(),
        "insights": if patterns.is_empty() {
            vec!["Training intensity is relatively uniform".to_string()]
        } else {
            patterns
        },
        "confidence": "medium",
    })
}

/// Detect progression patterns (gradual improvements)
fn detect_progression_pattern(activities: &[crate::models::Activity]) -> serde_json::Value {
    if activities.len() < 5 {
        return serde_json::json!({
            "pattern_type": "progression",
            "activities_analyzed": activities.len(),
            "insights": ["Need at least 5 activities to detect progression patterns"],
            "confidence": "insufficient_data",
        });
    }

    // Analyze distance progression
    let distances: Vec<f64> = activities
        .iter()
        .filter_map(|a| a.distance_meters)
        .collect();

    let mut patterns = Vec::new();

    if distances.len() >= 5 {
        // Calculate moving average to smooth out variations
        #[allow(clippy::cast_precision_loss)]
        let first_half_avg =
            distances[..distances.len() / 2].iter().sum::<f64>() / (distances.len() / 2) as f64;
        #[allow(clippy::cast_precision_loss)]
        let second_half_avg = distances[distances.len() / 2..].iter().sum::<f64>()
            / (distances.len() - distances.len() / 2) as f64;

        let change_pct = ((second_half_avg - first_half_avg) / first_half_avg) * 100.0;

        if change_pct > 10.0 {
            patterns.push(format!(
                "Progressive distance increase detected: {change_pct:.1}% growth"
            ));
        } else if change_pct < -10.0 {
            patterns.push(format!(
                "Distance reduction pattern: {:.1}% decrease (taper or recovery?)",
                change_pct.abs()
            ));
        } else {
            patterns.push("Stable distance pattern - maintaining consistent volume".to_string());
        }

        #[allow(clippy::items_after_statements)]
        // Check for sudden spikes (injury risk)
        for window in distances.windows(2) {
            let increase = ((window[1] - window[0]) / window[0]) * 100.0;
            if increase > 25.0 {
                patterns
                    .push("⚠️ Warning: Sudden distance spike detected - injury risk".to_string());
                break;
            }
        }
    }

    serde_json::json!({
        "pattern_type": "progression",
        "activities_analyzed": activities.len(),
        "patterns_detected": patterns.clone(),
        "insights": patterns,
        "confidence": if distances.len() >= 5 { "medium" } else { "low" },
    })
}

/// Detect overtraining warning signs
fn detect_overtraining_signs(activities: &[crate::models::Activity]) -> serde_json::Value {
    use chrono::Duration;

    if activities.len() < 7 {
        return serde_json::json!({
            "pattern_type": "overtraining",
            "activities_analyzed": activities.len(),
            "insights": ["Need at least 7 activities to detect overtraining patterns"],
            "confidence": "insufficient_data",
        });
    }

    let mut warning_signs = Vec::new();
    let mut risk_score = 0;

    // Check 1: High activity frequency (no rest days)
    let mut consecutive_days = 0;
    let mut max_consecutive = 0;

    let mut sorted = activities.to_vec();
    sorted.sort_by(|a, b| a.start_date.cmp(&b.start_date));

    for window in sorted.windows(2) {
        let gap = window[1].start_date - window[0].start_date;
        if gap < Duration::days(2) {
            consecutive_days += 1;
            max_consecutive = max_consecutive.max(consecutive_days);
        } else {
            consecutive_days = 0;
        }
    }

    if max_consecutive >= 7 {
        warning_signs.push("⚠️ Extended period without rest days detected".to_string());
        risk_score += 2;
    }

    // Check 2: Declining performance with maintained volume
    let recent_activities = if activities.len() > 10 {
        &activities[..10]
    } else {
        activities
    };

    let avg_speeds: Vec<f64> = recent_activities
        .iter()
        .filter_map(|a| a.average_speed)
        .collect();

    if avg_speeds.len() >= 4 {
        #[allow(clippy::cast_precision_loss)]
        let first_half_avg =
            avg_speeds[..avg_speeds.len() / 2].iter().sum::<f64>() / (avg_speeds.len() / 2) as f64;
        #[allow(clippy::cast_precision_loss)]
        let second_half_avg = avg_speeds[avg_speeds.len() / 2..].iter().sum::<f64>()
            / (avg_speeds.len() - avg_speeds.len() / 2) as f64;

        let speed_change = ((second_half_avg - first_half_avg) / first_half_avg) * 100.0;

        if speed_change < -10.0 {
            warning_signs
                .push("Performance declining despite training - possible fatigue".to_string());
            risk_score += 2;
        }
    }

    // Check 3: Elevated resting heart rate trend
    let hrs: Vec<u32> = recent_activities
        .iter()
        .filter_map(|a| a.average_heart_rate)
        .collect();

    if hrs.len() >= 5 {
        #[allow(clippy::cast_possible_truncation)]
        let first_avg = hrs[..hrs.len() / 2].iter().sum::<u32>() / (hrs.len() / 2) as u32;
        #[allow(clippy::cast_possible_truncation)]
        let recent_avg =
            hrs[hrs.len() / 2..].iter().sum::<u32>() / (hrs.len() - hrs.len() / 2) as u32;

        if recent_avg > first_avg + 5 {
            warning_signs.push("Elevated heart rate pattern detected - check recovery".to_string());
            risk_score += 1;
        }
    }

    let risk_level = match risk_score {
        0..=1 => "low",
        2..=3 => "moderate",
        _ => "high",
    };

    serde_json::json!({
        "pattern_type": "overtraining",
        "activities_analyzed": activities.len(),
        "risk_level": risk_level,
        "risk_score": risk_score,
        "warning_signs": warning_signs.clone(),
        "insights": if warning_signs.is_empty() {
            vec!["No significant overtraining signs detected - training load appears manageable".to_string()]
        } else {
            warning_signs
        },
        "confidence": "medium",
        "recommendations": if risk_score >= 2 {
            vec![
                "Consider scheduling rest days",
                "Review training intensity and volume",
                "Monitor sleep quality and recovery",
            ]
        } else {
            vec!["Continue monitoring training load and recovery"]
        },
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
            "rationale": "No recent training data available",
        });
    }

    match recommendation_type {
        "training_plan" => generate_training_plan_recommendations(activities),
        "recovery" => generate_recovery_recommendations(activities),
        "intensity" => generate_intensity_recommendations(activities),
        "goal_specific" => generate_goal_specific_recommendations(activities),
        _ => generate_comprehensive_recommendations(activities),
    }
}

/// Generate weekly training plan recommendations
fn generate_training_plan_recommendations(
    activities: &[crate::models::Activity],
) -> serde_json::Value {
    // Analyze current training frequency
    let days_with_activities: std::collections::HashSet<_> = activities
        .iter()
        .map(|a| a.start_date.date_naive())
        .collect();

    let weekly_frequency = days_with_activities.len();
    let total_volume: f64 = activities.iter().filter_map(|a| a.distance_meters).sum();

    let mut recommendations = Vec::new();
    let mut weekly_structure = Vec::new();

    // Determine training level
    if weekly_frequency < 3 {
        recommendations.push(
            "Increase training frequency to 3-4 days per week for consistent progress".to_string(),
        );
        weekly_structure.push(serde_json::json!({
            "day": "Monday",
            "workout_type": "Easy run",
            "duration": "30-40 minutes",
        }));
        weekly_structure.push(serde_json::json!({
            "day": "Wednesday",
            "workout_type": "Tempo or threshold",
            "duration": "35-45 minutes",
        }));
        weekly_structure.push(serde_json::json!({
            "day": "Saturday",
            "workout_type": "Long run",
            "duration": "45-60 minutes",
        }));
    } else if weekly_frequency <= 5 {
        recommendations
            .push("Good training frequency - follow 80/20 rule (80% easy, 20% hard)".to_string());
        weekly_structure.push(serde_json::json!({
            "day": "Monday",
            "workout_type": "Easy run",
            "duration": "40-50 minutes",
        }));
        weekly_structure.push(serde_json::json!({
            "day": "Tuesday",
            "workout_type": "Intervals or speed work",
            "duration": "45 minutes with warm-up/cool-down",
        }));
        weekly_structure.push(serde_json::json!({
            "day": "Thursday",
            "workout_type": "Tempo run",
            "duration": "40-50 minutes",
        }));
        weekly_structure.push(serde_json::json!({
            "day": "Saturday",
            "workout_type": "Long run",
            "duration": "60-90 minutes",
        }));
        weekly_structure.push(serde_json::json!({
            "day": "Sunday",
            "workout_type": "Easy recovery run",
            "duration": "30-40 minutes",
        }));
    } else {
        recommendations.push(
            "High training frequency - ensure adequate recovery between sessions".to_string(),
        );
        recommendations.push("Consider periodization: alternate hard/easy weeks".to_string());
    }

    // Volume recommendations
    #[allow(clippy::cast_precision_loss)]
    let avg_distance = total_volume / activities.len() as f64;
    if avg_distance < 5_000.0 {
        recommendations.push("Gradually increase run distance by 10% per week".to_string());
    } else if avg_distance > 15_000.0 {
        recommendations
            .push("Monitor for overtraining - high average distance detected".to_string());
    }

    serde_json::json!({
        "recommendation_type": "training_plan",
        "current_frequency": weekly_frequency,
        "recommendations": recommendations,
        "suggested_weekly_structure": weekly_structure,
        "principles": [
            "Follow the 10% rule - increase volume by max 10% per week",
            "Include 1-2 rest days per week",
            "Build aerobic base before adding intensity",
        ],
    })
}

/// Generate recovery recommendations
fn generate_recovery_recommendations(activities: &[crate::models::Activity]) -> serde_json::Value {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let mut recommendations = Vec::new();
    let mut recovery_status = "unknown";

    // Check time since last activity
    if let Some(last_activity) = activities.first() {
        let days_since = (now - last_activity.start_date).num_days();

        if days_since == 0 {
            recommendations
                .push("You trained today - ensure proper nutrition and hydration".to_string());
            recovery_status = "active";
        } else if days_since == 1 {
            recommendations.push("Consider an easy recovery day or complete rest".to_string());
            recovery_status = "recovering";
        } else if days_since >= 3 {
            recommendations
                .push("Good recovery time - ready for next training session".to_string());
            recovery_status = "recovered";
        }
    }

    // Check for consecutive training days
    let mut sorted = activities.to_vec();
    sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date));

    let mut consecutive_days = 0;
    for window in sorted.windows(2) {
        let gap = window[0].start_date - window[1].start_date;
        if gap < Duration::days(2) {
            consecutive_days += 1;
        } else {
            break;
        }
    }

    if consecutive_days >= 3 {
        recommendations.push(format!(
            "⚠️ {consecutive_days} consecutive training days detected - schedule a rest day"
        ));
        recovery_status = "needs_rest";
    }

    // HR-based recovery check
    let recent_hrs: Vec<u32> = sorted
        .iter()
        .take(5)
        .filter_map(|a| a.average_heart_rate)
        .collect();

    if recent_hrs.len() >= 3 {
        #[allow(clippy::cast_possible_truncation)]
        let recent_avg = recent_hrs.iter().sum::<u32>() / recent_hrs.len() as u32;

        let older_hrs: Vec<u32> = sorted
            .iter()
            .skip(5)
            .take(5)
            .filter_map(|a| a.average_heart_rate)
            .collect();

        if let Some(older_avg) = older_hrs.first() {
            if recent_avg > older_avg + 5 {
                recommendations
                    .push("Elevated heart rate trend - prioritize recovery this week".to_string());
            }
        }
    }

    if recommendations.is_empty() {
        recommendations.push("Recovery status looks good - maintain current routine".to_string());
    }

    serde_json::json!({
        "recommendation_type": "recovery",
        "recovery_status": recovery_status,
        "consecutive_training_days": consecutive_days,
        "recommendations": recommendations,
        "recovery_tips": [
            "Aim for 7-9 hours of sleep per night",
            "Stay hydrated - drink 2-3L water daily",
            "Include protein within 30min post-workout",
            "Consider foam rolling or massage for muscle recovery",
        ],
    })
}

/// Generate intensity recommendations
fn generate_intensity_recommendations(activities: &[crate::models::Activity]) -> serde_json::Value {
    let hrs_with_values: Vec<&crate::models::Activity> = activities
        .iter()
        .filter(|a| a.average_heart_rate.is_some())
        .collect();

    if hrs_with_values.len() < 3 {
        return serde_json::json!({
            "recommendation_type": "intensity",
            "recommendations": ["Need more activities with heart rate data for intensity analysis"],
        });
    }

    // Calculate intensity distribution
    #[allow(clippy::cast_precision_loss)]
    let avg_hr: f64 = hrs_with_values
        .iter()
        .filter_map(|a| a.average_heart_rate.map(f64::from))
        .sum::<f64>()
        / hrs_with_values.len() as f64;

    let mut easy_count = 0;
    let mut moderate_count = 0;
    let mut hard_count = 0;

    for activity in &hrs_with_values {
        if let Some(hr) = activity.average_heart_rate {
            let hr_f64 = f64::from(hr);
            if hr_f64 < avg_hr * 0.85 {
                easy_count += 1;
            } else if hr_f64 < avg_hr * 1.05 {
                moderate_count += 1;
            } else {
                hard_count += 1;
            }
        }
    }

    let total = hrs_with_values.len();
    #[allow(clippy::cast_precision_loss)]
    let easy_pct = (f64::from(easy_count) / total as f64) * 100.0;
    #[allow(clippy::cast_precision_loss)]
    let hard_pct = (f64::from(hard_count) / total as f64) * 100.0;

    let mut recommendations = Vec::new();

    // Check 80/20 principle
    if easy_pct < 70.0 {
        recommendations.push("Add more easy/recovery runs - aim for 80% easy effort".to_string());
    } else if easy_pct > 90.0 {
        recommendations.push("Include 1-2 harder efforts per week to improve fitness".to_string());
    } else {
        recommendations.push("Good intensity balance following 80/20 principle".to_string());
    }

    // Specific workout recommendations
    if hard_count == 0 {
        recommendations
            .push("Add interval training: 5x800m @ 5K pace with 90s recovery".to_string());
        recommendations.push("Add tempo run: 20min @ comfortably hard pace".to_string());
    } else if hard_count >= 3 {
        recommendations.push("Reduce high-intensity frequency to prevent overtraining".to_string());
    }

    serde_json::json!({
        "recommendation_type": "intensity",
        "intensity_distribution": {
            "easy": easy_count,
            "moderate": moderate_count,
            "hard": hard_count,
            "easy_percentage": easy_pct,
            "hard_percentage": hard_pct,
        },
        "recommendations": recommendations,
        "target_distribution": "80% easy, 10% moderate, 10% hard (80/20 rule)",
    })
}

/// Generate goal-specific recommendations
fn generate_goal_specific_recommendations(
    activities: &[crate::models::Activity],
) -> serde_json::Value {
    // Detect primary sport
    use std::collections::HashMap;
    let mut sport_counts: HashMap<String, usize> = HashMap::new();
    for activity in activities {
        let sport = format!("{:?}", activity.sport_type);
        *sport_counts.entry(sport).or_insert(0) += 1;
    }

    let primary_sport = sport_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map_or("Unknown", |(sport, _)| sport.as_str());

    let mut recommendations = Vec::new();

    // Sport-specific recommendations
    if primary_sport.contains("Run") {
        recommendations.push("For 5K goal: Include weekly speed work (intervals)".to_string());
        recommendations.push("For 10K goal: Build weekly long run to 12-15K".to_string());
        recommendations.push("For half marathon: Progressive long runs up to 18-20K".to_string());
        recommendations.push("Add strides 2x/week to improve running economy".to_string());
    } else if primary_sport.contains("Ride") {
        recommendations.push("Build FTP with 2x20min threshold intervals".to_string());
        recommendations.push("Include hill repeats for strength".to_string());
        recommendations.push("Long endurance rides on weekends".to_string());
    } else {
        recommendations.push("Continue building aerobic base with consistent training".to_string());
    }

    serde_json::json!({
        "recommendation_type": "goal_specific",
        "primary_sport": primary_sport,
        "recommendations": recommendations,
        "training_phases": [
            "Phase 1 (Base): Build aerobic endurance - 4-6 weeks",
            "Phase 2 (Build): Add tempo and threshold work - 4-6 weeks",
            "Phase 3 (Peak): Race-specific intensity - 2-3 weeks",
            "Phase 4 (Taper): Reduce volume, maintain intensity - 1-2 weeks",
        ],
    })
}

/// Generate comprehensive recommendations (all types)
fn generate_comprehensive_recommendations(
    activities: &[crate::models::Activity],
) -> serde_json::Value {
    let frequency_check = if activities.len() < 12 {
        "Aim for 3-4 activities per week for consistent progress"
    } else {
        "Good training consistency maintained"
    };

    serde_json::json!({
        "recommendation_type": "comprehensive",
        "recommendations": [
            frequency_check,
            "Follow progressive overload: gradually increase distance/intensity",
            "Include strength training 2x/week for injury prevention",
            "Listen to your body - rest when fatigued",
            "Track sleep and recovery metrics",
        ],
        "key_principles": {
            "consistency": "Regular training beats sporadic hard efforts",
            "recovery": "Adaptation happens during rest, not training",
            "progression": "Follow 10% rule for volume increases",
            "variety": "Mix easy, moderate, and hard efforts",
        },
        "activities_analyzed": activities.len(),
    })
}

// ============================================================================
// Helper Functions for Fitness Score Calculation (CTL/ATL/TSS)
// ============================================================================

/// Calculate fitness metrics using CTL/ATL/TSS methodology
fn calculate_fitness_metrics(
    activities: &[crate::models::Activity],
    timeframe: &str,
) -> serde_json::Value {
    if activities.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "fitness_score": 0,
            "message": "No activities found for fitness calculation",
        });
    }

    // Calculate TSS for each activity
    let mut activity_tss: Vec<(chrono::DateTime<chrono::Utc>, f64)> = activities
        .iter()
        .filter_map(|a| {
            let tss = estimate_tss(a);
            if tss > 0.0 {
                Some((a.start_date, tss))
            } else {
                None
            }
        })
        .collect();

    activity_tss.sort_by(|a, b| a.0.cmp(&b.0));

    if activity_tss.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "fitness_score": 0,
            "message": "Insufficient data for TSS calculation",
        });
    }

    // Calculate CTL (42-day rolling average)
    let ctl = calculate_ctl(&activity_tss);

    // Calculate ATL (7-day rolling average)
    let atl = calculate_atl(&activity_tss);

    // Calculate TSB (Training Stress Balance) = CTL - ATL
    let tsb = ctl - atl;

    // Form/freshness interpretation
    let form_status = if tsb > 10.0 {
        "fresh"
    } else if tsb > -10.0 {
        "balanced"
    } else if tsb > -30.0 {
        "fatigued"
    } else {
        "very_fatigued"
    };

    // Fitness level interpretation
    let fitness_level = if ctl > 100.0 {
        "elite"
    } else if ctl > 70.0 {
        "advanced"
    } else if ctl > 40.0 {
        "intermediate"
    } else if ctl > 20.0 {
        "beginner"
    } else {
        "novice"
    };

    #[allow(clippy::cast_possible_truncation)]
    let fitness_score = ctl.round() as i32;

    serde_json::json!({
        "timeframe": timeframe,
        "fitness_score": fitness_score,
        "metrics": {
            "ctl": ctl.round(),
            "atl": atl.round(),
            "tsb": tsb.round(),
        },
        "fitness_level": fitness_level,
        "form_status": form_status,
        "interpretation": {
            "ctl": "Chronic Training Load - long-term fitness (42-day average)",
            "atl": "Acute Training Load - recent fatigue (7-day average)",
            "tsb": "Training Stress Balance - form/freshness indicator",
        },
        "recommendations": generate_fitness_recommendations(ctl, atl, tsb),
        "activities_analyzed": activity_tss.len(),
    })
}

/// Estimate Training Stress Score (TSS) for an activity
fn estimate_tss(activity: &crate::models::Activity) -> f64 {
    // TSS formula: (duration_hours * IF^2 * 100)
    // IF (Intensity Factor) estimated from heart rate or pace

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_precision_loss)]
    let duration_hours = activity.duration_seconds as f64 / 3600.0;

    // Estimate intensity factor
    let intensity_factor = activity.average_heart_rate.map_or_else(
        || {
            activity.distance_meters.map_or(0.7, |distance| {
                if distance > 0.0 && activity.duration_seconds > 0 {
                    #[allow(clippy::cast_precision_loss)]
                    let pace_mps = distance / activity.duration_seconds as f64;
                    // Typical running pace: 3-5 m/s, cycling: 5-10 m/s
                    (pace_mps / 4.0).clamp(0.5, 1.5)
                } else {
                    0.7 // default moderate intensity
                }
            })
        },
        |hr| {
            // Assume max HR around 190, threshold around 170
            let hr_ratio = f64::from(hr) / 170.0;
            hr_ratio.clamp(0.5, 1.2)
        },
    );

    duration_hours * intensity_factor.powi(2) * 100.0
}

/// Calculate CTL (Chronic Training Load) - 42-day exponentially weighted average
fn calculate_ctl(activity_tss: &[(chrono::DateTime<chrono::Utc>, f64)]) -> f64 {
    const CTL_TIME_CONSTANT: f64 = 42.0;

    if activity_tss.is_empty() {
        return 0.0;
    }

    // Exponential decay constant for 42 days
    let decay_factor = 1.0 / CTL_TIME_CONSTANT;

    let mut ctl = 0.0;
    let mut previous_date = activity_tss[0].0;

    for (date, tss) in activity_tss {
        #[allow(clippy::cast_precision_loss)]
        let days_gap = (*date - previous_date).num_days() as f64;

        // Apply exponential decay
        if days_gap > 0.0 {
            ctl *= (1.0 - decay_factor).powf(days_gap);
        }

        // Add today's TSS
        ctl = ctl.mul_add(1.0 - decay_factor, tss * decay_factor);
        previous_date = *date;
    }

    ctl
}

/// Calculate ATL (Acute Training Load) - 7-day exponentially weighted average
fn calculate_atl(activity_tss: &[(chrono::DateTime<chrono::Utc>, f64)]) -> f64 {
    const ATL_TIME_CONSTANT: f64 = 7.0;

    if activity_tss.is_empty() {
        return 0.0;
    }

    // Exponential decay constant for 7 days
    let decay_factor = 1.0 / ATL_TIME_CONSTANT;

    let mut atl = 0.0;
    let mut previous_date = activity_tss[0].0;

    for (date, tss) in activity_tss {
        #[allow(clippy::cast_precision_loss)]
        let days_gap = (*date - previous_date).num_days() as f64;

        // Apply exponential decay
        if days_gap > 0.0 {
            atl *= (1.0 - decay_factor).powf(days_gap);
        }

        // Add today's TSS
        atl = atl.mul_add(1.0 - decay_factor, tss * decay_factor);
        previous_date = *date;
    }

    atl
}

/// Generate recommendations based on fitness metrics
fn generate_fitness_recommendations(ctl: f64, atl: f64, tsb: f64) -> Vec<String> {
    let mut recommendations = Vec::new();

    // TSB-based recommendations
    if tsb > 15.0 {
        recommendations
            .push("You're very fresh - good time for hard workouts or races".to_string());
    } else if tsb > 5.0 {
        recommendations.push("Good form - ready for quality training".to_string());
    } else if tsb > -10.0 {
        recommendations.push("Balanced training stress - maintain current load".to_string());
    } else if tsb > -25.0 {
        recommendations.push("Moderate fatigue - consider adding recovery days".to_string());
    } else {
        recommendations.push("⚠️ High fatigue detected - prioritize rest and recovery".to_string());
    }

    // CTL-based recommendations
    if ctl < 30.0 {
        recommendations.push("Build aerobic base with consistent easy training".to_string());
    } else if ctl > 80.0 && atl > 100.0 {
        recommendations.push("High training load - watch for overtraining signs".to_string());
    }

    // ATL vs CTL ratio
    let ratio = if ctl > 0.0 { atl / ctl } else { 0.0 };
    if ratio > 1.5 {
        recommendations.push("Recent spike in training - allow time to adapt".to_string());
    }

    recommendations
}

// ============================================================================
// Helper Functions for Performance Prediction (VDOT/Riegel)
// ============================================================================

/// Predict race performance using VDOT and Riegel formulas
fn predict_race_performance(
    activities: &[crate::models::Activity],
    target_sport: &str,
) -> serde_json::Value {
    // Filter activities by sport type
    let running_activities: Vec<&crate::models::Activity> = activities
        .iter()
        .filter(|a| format!("{:?}", a.sport_type).contains("Run"))
        .collect();

    if running_activities.is_empty() {
        return serde_json::json!({
            "target_sport": target_sport,
            "message": "No running activities found for prediction",
            "predictions": [],
        });
    }

    // Find best recent performance (fastest pace over distance > 3km)
    let Some((best_distance, best_time)) = find_best_performance(&running_activities) else {
        return serde_json::json!({
            "target_sport": target_sport,
            "message": "No suitable activities found for prediction (need distance > 3km)",
            "predictions": [],
        });
    };

    // Calculate VDOT from best performance
    let vdot = calculate_vdot(best_distance, best_time);

    // Generate race predictions using VDOT tables
    let predictions = generate_race_predictions(vdot);

    // Calculate confidence based on data quality
    let confidence = calculate_prediction_confidence(&running_activities);

    serde_json::json!({
        "target_sport": target_sport,
        "vdot": vdot.round(),
        "best_performance": {
            "distance_meters": best_distance,
            "time_seconds": best_time,
            "pace_min_km": (best_time / 60.0) / (best_distance / 1000.0),
        },
        "race_predictions": predictions,
        "confidence": confidence,
        "activities_analyzed": running_activities.len(),
        "notes": [
            "Predictions assume proper race preparation and taper",
            "Based on VDOT methodology by Jack Daniels",
            "Actual performance may vary with conditions and training",
        ],
    })
}

/// Find best performance (fastest pace over meaningful distance)
#[allow(clippy::cast_precision_loss)]
fn find_best_performance(activities: &[&crate::models::Activity]) -> Option<(f64, f64)> {
    const MIN_DISTANCE: f64 = 3000.0; // 3km minimum

    activities
        .iter()
        .filter_map(|a| {
            a.distance_meters.and_then(|distance| {
                if distance >= MIN_DISTANCE && a.duration_seconds > 0 {
                    #[allow(clippy::cast_precision_loss)]
                    let time_seconds = a.duration_seconds as f64;
                    let pace = time_seconds / distance; // seconds per meter
                    Some((distance, time_seconds, pace))
                } else {
                    None
                }
            })
        })
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(dist, time, _)| (dist, time))
}

/// Calculate VDOT from race performance
/// `VDOT` formula: `VO2max` estimation based on race time and distance
#[allow(
    clippy::suboptimal_flops,
    clippy::unreadable_literal,
    clippy::similar_names
)]
fn calculate_vdot(distance_meters: f64, time_seconds: f64) -> f64 {
    // Simplified VDOT calculation
    // Full formula is complex; this is an approximation

    let velocity_mps = distance_meters / time_seconds;
    let velocity_mpm = velocity_mps * 60.0; // meters per minute

    // Oxygen cost of running (ml/kg/min)
    // VO2 = -4.60 + 0.182258 * velocity + 0.000104 * velocity^2
    let vo2 = velocity_mpm.mul_add(0.182258, -4.60 + 0.000104 * velocity_mpm.powi(2));

    // Percentage of VO2max based on duration
    let duration_minutes = time_seconds / 60.0;
    let percent_max = if duration_minutes < 3.0 {
        1.0 // Very short = max effort
    } else if duration_minutes < 10.0 {
        0.98
    } else if duration_minutes < 30.0 {
        0.95
    } else if duration_minutes < 60.0 {
        0.90
    } else if duration_minutes < 150.0 {
        0.85
    } else {
        0.80
    };

    // VDOT = VO2 at race pace / percentage of max
    (vo2 / percent_max).clamp(30.0, 85.0)
}

/// Generate race time predictions for standard distances
fn generate_race_predictions(vdot: f64) -> Vec<serde_json::Value> {
    let mut predictions = Vec::new();

    // Standard race distances (meters)
    let races = vec![
        (5_000.0, "5K"),
        (10_000.0, "10K"),
        (21_097.5, "Half Marathon"),
        (42_195.0, "Marathon"),
    ];

    for (distance, name) in races {
        let predicted_time = predict_time_from_vdot(vdot, distance);

        predictions.push(serde_json::json!({
            "distance": name,
            "distance_meters": distance,
            "predicted_time_seconds": predicted_time.round(),
            "predicted_time_formatted": format_time(predicted_time),
            "predicted_pace_min_km": format_pace((predicted_time / 60.0) / (distance / 1000.0)),
        }));
    }

    predictions
}

/// Predict race time from VDOT for a given distance
#[allow(clippy::items_after_statements)]
fn predict_time_from_vdot(vdot: f64, distance_meters: f64) -> f64 {
    // Reverse VDOT calculation to get time
    // This is a simplification using Riegel's formula as approximation

    // Reference: 5K at given VDOT
    let reference_distance = 5000.0;
    let reference_velocity = calculate_velocity_from_vdot(vdot, reference_distance);
    let reference_time = reference_distance / reference_velocity;

    // Riegel's formula: T2 = T1 * (D2/D1)^1.06
    // Fatigue factor: longer distances require slower pace
    const FATIGUE_EXPONENT: f64 = 1.06;
    reference_time * (distance_meters / reference_distance).powf(FATIGUE_EXPONENT)
}

/// Calculate running velocity from VDOT
#[allow(
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::unreadable_literal
)]
fn calculate_velocity_from_vdot(vdot: f64, distance_meters: f64) -> f64 {
    // Estimate velocity that would produce this VDOT
    // Simplified inverse calculation

    let duration_estimate = if distance_meters <= 5_000.0 {
        0.95 // ~95% VO2max for 5K
    } else if distance_meters <= 10_000.0 {
        0.92
    } else if distance_meters <= 21_097.5 {
        0.88
    } else {
        0.82 // ~82% VO2max for marathon
    };

    let vo2_at_pace = vdot * duration_estimate;

    // Solve for velocity from VO2 equation
    // VO2 = -4.60 + 0.182258*V + 0.000104*V^2
    // Using quadratic formula
    let a: f64 = 0.000104;
    let b: f64 = 0.182258;
    let c: f64 = -4.60 - vo2_at_pace;

    let discriminant = b.mul_add(b, -(4.0 * a * c));
    if discriminant < 0.0 {
        return 3.0; // fallback: ~12 min/km
    }

    let velocity_mpm = (-b + discriminant.sqrt()) / (2.0 * a);
    velocity_mpm / 60.0 // convert to m/s
}

/// Format time in HH:MM:SS
fn format_time(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        let hours = (seconds / 3600.0).floor() as u32;
        let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
        let secs = (seconds % 60.0).round() as u32;

        if hours > 0 {
            format!("{hours}:{minutes:02}:{secs:02}")
        } else {
            format!("{minutes}:{secs:02}")
        }
    }
}

/// Format pace in MM:SS/km
fn format_pace(minutes_per_km: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        let mins = minutes_per_km.floor() as u32;
        let secs = ((minutes_per_km - f64::from(mins)) * 60.0).round() as u32;
        format!("{mins}:{secs:02}/km")
    }
}

/// Calculate prediction confidence based on data quality
#[allow(clippy::cast_precision_loss, clippy::similar_names)]
fn calculate_prediction_confidence(activities: &[&crate::models::Activity]) -> String {
    if activities.len() < 3 {
        "low"
    } else if activities.len() < 10 {
        "medium"
    } else {
        // Check consistency of recent performances
        let recent_paces: Vec<f64> = activities
            .iter()
            .take(5)
            .filter_map(|a| {
                a.distance_meters.and_then(|distance| {
                    if distance > 3_000.0 && a.duration_seconds > 0 {
                        #[allow(clippy::cast_precision_loss)]
                        let time = a.duration_seconds as f64;
                        Some(time / distance)
                    } else {
                        None
                    }
                })
            })
            .collect();

        if recent_paces.len() >= 3 {
            #[allow(clippy::cast_precision_loss)]
            let avg_pace = recent_paces.iter().sum::<f64>() / recent_paces.len() as f64;
            #[allow(clippy::cast_precision_loss)]
            let variance: f64 = recent_paces
                .iter()
                .map(|p| (p - avg_pace).powi(2))
                .sum::<f64>()
                / recent_paces.len() as f64;
            let std_dev = variance.sqrt();

            // Low variance = high confidence
            if std_dev / avg_pace < 0.1 {
                "high"
            } else {
                "medium"
            }
        } else {
            "medium"
        }
    }
    .to_string()
}

// ============================================================================
// Helper Functions for Training Load Analysis
// ============================================================================

/// Analyze training load with detailed TSS/CTL/ATL/TSB metrics
fn analyze_detailed_training_load(
    activities: &[crate::models::Activity],
    timeframe: &str,
) -> serde_json::Value {
    if activities.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "message": "No activities found for training load analysis",
        });
    }

    // Calculate TSS for each activity (reuse from fitness score)
    let mut activity_tss: Vec<(chrono::DateTime<chrono::Utc>, f64)> = activities
        .iter()
        .filter_map(|a| {
            let tss = estimate_tss(a);
            if tss > 0.0 {
                Some((a.start_date, tss))
            } else {
                None
            }
        })
        .collect();

    activity_tss.sort_by(|a, b| a.0.cmp(&b.0));

    if activity_tss.is_empty() {
        return serde_json::json!({
            "timeframe": timeframe,
            "message": "Insufficient data for training load calculation",
        });
    }

    // Calculate CTL, ATL, TSB (reuse existing functions)
    let ctl = calculate_ctl(&activity_tss);
    let atl = calculate_atl(&activity_tss);
    let tsb = ctl - atl;

    // Calculate weekly TSS totals
    let weekly_tss = calculate_weekly_tss_totals(&activity_tss);

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
            .push("Recent spike in training - allow adaptation time".to_string());
    }
    if ctl < 30.0 {
        periodization_suggestions
            .push("Building base - focus on consistency and volume".to_string());
    } else if ctl > 80.0 {
        periodization_suggestions
            .push("High fitness level - maintain or add recovery weeks".to_string());
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
        "activities_analyzed": activity_tss.len(),
        "interpretation": {
            "ctl": "Chronic Training Load - fitness level (42-day average TSS)",
            "atl": "Acute Training Load - fatigue level (7-day average TSS)",
            "tsb": "Training Stress Balance - form indicator (CTL - ATL)",
            "positive_tsb": "Fresh and recovered, ready for hard training",
            "negative_tsb": "Fatigued, prioritize recovery",
        },
    })
}

/// Calculate weekly TSS totals
#[allow(clippy::cast_possible_truncation)]
fn calculate_weekly_tss_totals(
    activity_tss: &[(chrono::DateTime<chrono::Utc>, f64)],
) -> Vec<serde_json::Value> {
    use std::collections::HashMap;

    if activity_tss.is_empty() {
        return Vec::new();
    }

    // Group by week
    let mut weekly_totals: HashMap<i32, f64> = HashMap::new();
    let first_date = activity_tss[0].0;

    for (date, tss) in activity_tss {
        let days_diff = (*date - first_date).num_days();
        let week_number = days_diff / 7;
        *weekly_totals.entry(week_number as i32).or_insert(0.0) += tss;
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
        "Overreached - high fatigue".to_string()
    } else if tsb < -10.0 {
        "Productive - building fitness under fatigue".to_string()
    } else if tsb < 5.0 {
        "Balanced - good training stress balance".to_string()
    } else if tsb < 15.0 {
        "Fresh - ready for quality work".to_string()
    } else {
        "Very fresh - possibly detraining".to_string()
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
        recommendations.push("⚠️ Critical fatigue - take 2-3 rest days immediately".to_string());
        recommendations.push("Reduce training volume by 50% this week".to_string());
    } else if tsb < -15.0 {
        recommendations.push("High fatigue - schedule recovery week".to_string());
        recommendations.push("Reduce intensity and add extra rest day".to_string());
    } else if tsb < -5.0 {
        recommendations
            .push("Moderate fatigue - maintain current load or slight reduction".to_string());
    } else if tsb > 15.0 {
        recommendations.push("Very fresh - good time for breakthrough workout or race".to_string());
    }

    // CTL/ATL ratio analysis
    let ratio = if ctl > 0.0 { atl / ctl } else { 0.0 };
    if ratio > 1.5 {
        recommendations
            .push("Recent training spike detected - allow 1-2 weeks adaptation".to_string());
    } else if ratio < 0.8 && ctl > 30.0 {
        recommendations.push("Well adapted to training - can increase load gradually".to_string());
    }

    // Progressive load recommendations
    if ctl < 30.0 {
        recommendations.push("Build weekly TSS by 3-5 points per week".to_string());
    } else if ctl > 80.0 {
        recommendations
            .push("High load - incorporate recovery weeks (reduce by 20-30%)".to_string());
    }

    if recommendations.is_empty() {
        recommendations
            .push("Training load is well balanced - maintain current approach".to_string());
    }

    recommendations
}
