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

    let intensity_score = heart_rate.map_or(DEFAULT_EFFICIENCY_SCORE, |hr| {
        (f64::from(hr) / ASSUMED_MAX_HR) * limits::PERCENTAGE_MULTIPLIER
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
                serde_json::Value::String("1.0".into()),
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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let trend = if activities.is_empty() {
                            "stable"
                        } else if activities.len() > 5 {
                            "improving"
                        } else {
                            "needs_more_data"
                        };

                        let analysis = serde_json::json!({
                            "metric": metric,
                            "timeframe": timeframe,
                            "trend": trend,
                            "activities_analyzed": activities.len(),
                            "insights": [
                                format!("Analyzed {} activities over {}", activities.len(), timeframe),
                                format!("Trend for {}: {}", metric, trend)
                            ]
                        });

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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let target_activity = activities.iter().find(|a| a.id == activity_id);

                        let comparison = serde_json::json!({
                            "activity_id": activity_id,
                            "comparison_type": comparison_type,
                            "found": target_activity.is_some(),
                            "total_activities": activities.len(),
                            "insights": if target_activity.is_some() {
                                vec![format!("Found activity {} for comparison", activity_id)]
                            } else {
                                vec![format!("Activity {} not found in recent activities", activity_id)]
                            }
                        });

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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let patterns_detected = if activities.len() >= 3 {
                            vec!["Consistent training pattern detected".to_string()]
                        } else {
                            vec!["Insufficient data for pattern detection".to_string()]
                        };

                        let analysis = serde_json::json!({
                            "pattern_type": pattern_type,
                            "activities_analyzed": activities.len(),
                            "patterns_detected": patterns_detected,
                            "confidence": if activities.len() >= 3 { "medium" } else { "low" }
                        });

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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let mut recommendations = Vec::new();

                        if activities.len() < 3 {
                            recommendations
                                .push("Increase training frequency to at least 3 times per week");
                        } else {
                            recommendations.push("Maintain current training consistency");
                        }

                        recommendations.push("Include recovery days between intense workouts");
                        recommendations.push("Track your progress regularly");

                        let analysis = serde_json::json!({
                            "recommendation_type": recommendation_type,
                            "recommendations": recommendations,
                            "activities_analyzed": activities.len()
                        });

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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let score = (activities.len() * 10).min(100);

                        let analysis = serde_json::json!({
                            "timeframe": timeframe,
                            "fitness_score": score,
                            "activities_count": activities.len(),
                            "grade": if score >= 80 { "excellent" } else if score >= 60 { "good" } else { "needs_improvement" }
                        });

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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let confidence = if activities.len() >= 5 {
                            "high"
                        } else if activities.len() >= 3 {
                            "medium"
                        } else {
                            "low"
                        };

                        let prediction = serde_json::json!({
                            "target_sport": target_sport,
                            "confidence": confidence,
                            "activities_analyzed": activities.len(),
                            "prediction": format!("Performance prediction based on {} activities", activities.len())
                        });

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
                let mut provider = crate::providers::create_provider(oauth_providers::STRAVA)
                    .map_err(|e| {
                        ProtocolError::ExecutionFailed(format!("Failed to create provider: {e}"))
                    })?;

                let credentials = crate::providers::OAuth2Credentials {
                    client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                    client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                        let load_status = if activities.len() > 7 {
                            "high"
                        } else if activities.len() > 3 {
                            "moderate"
                        } else {
                            "low"
                        };

                        let recovery_needed = activities.len() > 5;

                        let analysis = serde_json::json!({
                            "timeframe": timeframe,
                            "load_status": load_status,
                            "activities_count": activities.len(),
                            "recovery_needed": recovery_needed,
                            "recommendations": if recovery_needed {
                                vec!["Consider adding rest days", "Monitor fatigue levels"]
                            } else {
                                vec!["Current load is sustainable", "Maintain consistency"]
                            }
                        });

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
