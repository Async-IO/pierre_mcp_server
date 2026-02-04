// ABOUTME: Intelligence and analysis handlers with clean separation
// ABOUTME: AI-powered analysis tools that delegate to intelligence services
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::environment::default_provider;
use crate::config::intelligence::IntelligenceConfig;
use crate::constants::limits::{self, METERS_PER_KILOMETER};
use crate::constants::time_constants;
use crate::constants::units::METERS_PER_KM;
use crate::errors::{AppResult, ErrorCode};
use crate::intelligence::physiological_constants::api_limits::{
    DEFAULT_ACTIVITY_LIMIT, MAX_ACTIVITY_LIMIT,
};
use crate::intelligence::physiological_constants::business_thresholds::{
    ACHIEVEMENT_DISTANCE_THRESHOLD_KM, ACHIEVEMENT_ELEVATION_THRESHOLD_M,
};
use crate::intelligence::physiological_constants::efficiency_defaults::{
    DEFAULT_EFFICIENCY_SCORE, DEFAULT_EFFICIENCY_WITH_DISTANCE,
};
use crate::intelligence::physiological_constants::heart_rate::{
    AGE_BASED_MAX_HR_CONSTANT, HIGH_INTENSITY_HR_THRESHOLD,
};
use crate::intelligence::physiological_constants::hr_estimation::ASSUMED_MAX_HR;
use crate::intelligence::physiological_constants::unit_conversions::MS_TO_KMH_FACTOR;
use crate::intelligence::training_load::TrainingLoad;
use crate::intelligence::{
    HardEasyPattern, MetricType, OvertrainingSignals, PatternDetector, PerformancePredictor,
    RiskLevel, SafeMetricExtractor, SleepAnalyzer, StatisticalAnalyzer, TrainingLoadCalculator,
    TrainingStatus, TrendDataPoint, TrendDirection, TssDataPoint, VolumeProgressionPattern,
    VolumeTrend, WeeklySchedulePattern,
};
use crate::mcp::sampling_peer::SamplingPeer;
use crate::mcp::schema::{Content, CreateMessageRequest, ModelPreferences, PromptMessage};
use crate::models::Activity;
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use crate::providers::core::FitnessProvider;
use crate::providers::OAuth2Credentials;
use crate::utils::uuid::parse_user_id_for_protocol;
use chrono::{Duration, Utc};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{info, warn};

use super::{apply_format_to_response, extract_output_format};

/// Activity parameters extracted from request
struct ActivityParameters {
    distance: f64,
    duration: u64,
    elevation_gain: f64,
    heart_rate: Option<u32>,
    max_hr_provided: Option<f64>,
    user_age: Option<u32>,
}

/// Information about recovery adjustment applied to fitness score
struct RecoveryAdjustmentInfo {
    /// Recovery quality score (0-100)
    recovery_score: f64,
    /// Adjustment factor applied to fitness score (0.9-1.1)
    adjustment_factor: f64,
    /// Provider name used for sleep data
    provider_name: String,
}

/// Fetch sleep data and calculate recovery adjustment for fitness score
///
/// Fetches recent sleep data from the specified provider and calculates a recovery
/// score that adjusts the fitness score based on current recovery status.
///
/// Recovery adjustment factors:
/// - 90-100 (Excellent): +5% bonus (1.05)
/// - 70-89 (Good): No adjustment (1.0)
/// - 50-69 (Moderate): -5% penalty (0.95)
/// - <50 (Poor): -10% penalty (0.90)
async fn fetch_and_calculate_recovery_adjustment(
    executor: &UniversalToolExecutor,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&str>,
    sleep_provider_name: &str,
    analysis: &mut serde_json::Value,
) -> Result<RecoveryAdjustmentInfo, String> {
    use super::sleep_recovery::fetch_provider_sleep_data;

    // Fetch sleep data from provider
    let sleep_data =
        fetch_provider_sleep_data(executor, user_uuid, tenant_id, sleep_provider_name, 1)
            .await
            .map_err(|e| e.error.unwrap_or_else(|| "Unknown error".to_owned()))?;

    // Calculate sleep quality score using SleepAnalyzer
    let config = &IntelligenceConfig::global().sleep_recovery;
    let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
        .map_err(|e| format!("Sleep quality calculation failed: {e}"))?;

    let recovery_score = sleep_quality.overall_score;

    // Calculate adjustment factor based on recovery score
    let adjustment_factor = if recovery_score >= 90.0 {
        1.05 // Excellent recovery: +5%
    } else if recovery_score >= 70.0 {
        1.0 // Good recovery: no adjustment
    } else if recovery_score >= 50.0 {
        0.95 // Moderate recovery: -5%
    } else {
        0.90 // Poor recovery: -10%
    };

    // Apply adjustment to fitness score in the analysis
    if let Some(obj) = analysis.as_object_mut() {
        if let Some(serde_json::Value::Number(score)) = obj.get("fitness_score") {
            if let Some(current_score) = score.as_i64() {
                // Safe: fitness score is 0-100, adjustment factor is 0.9-1.1, result fits in i64
                #[allow(clippy::cast_precision_loss)]
                #[allow(clippy::cast_possible_truncation)]
                let adjusted_score = ((current_score as f64) * adjustment_factor).round() as i64;
                obj.insert(
                    "fitness_score".to_owned(),
                    serde_json::Value::Number(adjusted_score.into()),
                );
                obj.insert(
                    "fitness_score_unadjusted".to_owned(),
                    serde_json::Value::Number(current_score.into()),
                );
            }
        }
    }

    Ok(RecoveryAdjustmentInfo {
        recovery_score,
        adjustment_factor,
        provider_name: sleep_provider_name.to_owned(),
    })
}

/// Recovery context information for training load analysis
struct RecoveryContextInfo {
    /// Sleep quality score (0-100)
    sleep_quality_score: f64,
    /// Recovery status interpretation
    recovery_status: String,
    /// HRV RMSSD if available
    hrv_rmssd: Option<f64>,
    /// Sleep duration in hours
    sleep_hours: f64,
}

/// Fetch recovery context for training load analysis
///
/// Fetches recent sleep data and provides recovery context to interpret
/// training load data (CTL/ATL/TSB) more accurately.
async fn fetch_recovery_context_for_training_load(
    executor: &UniversalToolExecutor,
    user_uuid: uuid::Uuid,
    tenant_id: Option<&str>,
    sleep_provider_name: &str,
) -> Result<RecoveryContextInfo, String> {
    use super::sleep_recovery::fetch_provider_sleep_data;
    use SleepAnalyzer;

    // Fetch sleep data from provider
    let sleep_data =
        fetch_provider_sleep_data(executor, user_uuid, tenant_id, sleep_provider_name, 1)
            .await
            .map_err(|e| e.error.unwrap_or_else(|| "Unknown error".to_owned()))?;

    // Calculate sleep quality score
    let config = &IntelligenceConfig::global().sleep_recovery;
    let sleep_quality = SleepAnalyzer::calculate_sleep_quality(&sleep_data, config)
        .map_err(|e| format!("Sleep quality calculation failed: {e}"))?;

    // Determine recovery status based on sleep quality
    let recovery_status = if sleep_quality.overall_score >= 90.0 {
        "excellent".to_owned()
    } else if sleep_quality.overall_score >= 75.0 {
        "good".to_owned()
    } else if sleep_quality.overall_score >= 60.0 {
        "moderate".to_owned()
    } else if sleep_quality.overall_score >= 40.0 {
        "fair".to_owned()
    } else {
        "poor".to_owned()
    };

    Ok(RecoveryContextInfo {
        sleep_quality_score: sleep_quality.overall_score,
        recovery_status,
        hrv_rmssd: sleep_data.hrv_rmssd_ms,
        sleep_hours: sleep_data.duration_hours,
    })
}

/// Parse activity parameters from request
///
/// Extracts activity metrics (distance, duration, elevation, heart rate) and
/// user profile data (max HR, age) from the MCP request parameters.
///
/// # Arguments
/// * `request` - The incoming MCP request with parameters
///
/// # Returns
/// Parsed activity parameters or error if required fields are missing
///
/// # Errors
/// Returns `ProtocolError::InvalidRequest` if activity parameter is missing
fn parse_activity_parameters(
    request: &UniversalRequest,
) -> Result<ActivityParameters, ProtocolError> {
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

    let max_hr_provided = request
        .parameters
        .get("max_hr")
        .and_then(serde_json::Value::as_f64);

    let user_age = request
        .parameters
        .get("age")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok());

    Ok(ActivityParameters {
        distance,
        duration,
        elevation_gain,
        heart_rate,
        max_hr_provided,
        user_age,
    })
}

/// Determine maximum heart rate
///
/// Determines max HR using priority: 1) explicit value, 2) Fox formula from age
/// (220 - age), 3) default assumed constant. Returns both the calculated value
/// and a source descriptor for transparency.
///
/// # Arguments
/// * `max_hr_provided` - Explicitly provided max HR (highest priority)
/// * `user_age` - User age for Fox formula calculation
///
/// # Returns
/// Tuple of (`max_hr_value`, `source_description`)
fn determine_max_heart_rate(max_hr_provided: Option<f64>, user_age: Option<u32>) -> (f64, String) {
    use ASSUMED_MAX_HR;

    match (max_hr_provided, user_age) {
        (Some(hr), _) => (hr, "provided".to_owned()),
        (None, Some(age)) => {
            let max_hr = f64::from(AGE_BASED_MAX_HR_CONSTANT.saturating_sub(age));
            (max_hr, format!("calculated_from_age_{age}"))
        }
        (None, None) => (ASSUMED_MAX_HR, "default_assumed".to_owned()),
    }
}

/// Calculated fitness metrics
struct CalculatedMetrics {
    pace: f64,
    speed: f64,
    intensity_score: f64,
    efficiency_score: f64,
}

/// Calculate activity metrics from parameters
///
/// Computes pace (min/km), speed (km/h), intensity score (% of max HR),
/// and efficiency score (distance/elevation ratio). Uses defensive checks
/// to avoid division by zero.
///
/// # Arguments
/// * `params` - Activity parameters (distance, duration, elevation, HR)
/// * `max_hr` - Maximum heart rate for intensity calculation
///
/// # Returns
/// Calculated metrics structure
fn calculate_activity_metrics(params: &ActivityParameters, max_hr: f64) -> CalculatedMetrics {
    let duration_f64 =
        f64::from(u32::try_from(params.duration.min(u64::from(u32::MAX))).unwrap_or(u32::MAX));

    let pace = if params.distance > 0.0 && params.duration > 0 {
        duration_f64 / (params.distance / METERS_PER_KILOMETER)
    } else {
        0.0
    };

    let speed = if params.duration > 0 {
        (params.distance / duration_f64) * MS_TO_KMH_FACTOR
    } else {
        0.0
    };

    let intensity_score = params.heart_rate.map_or(DEFAULT_EFFICIENCY_SCORE, |hr| {
        (f64::from(hr) / max_hr) * limits::PERCENTAGE_MULTIPLIER
    });

    let efficiency_score = if params.distance > 0.0 && params.elevation_gain > 0.0 {
        (params.distance / params.elevation_gain).min(100.0)
    } else {
        DEFAULT_EFFICIENCY_WITH_DISTANCE
    };

    CalculatedMetrics {
        pace,
        speed,
        intensity_score,
        efficiency_score,
    }
}

/// Build metrics calculation response
///
/// Constructs the MCP response with calculated metrics, summary data, and metadata.
/// Includes timestamps and personalization flags for transparency.
///
/// # Arguments
/// * `params` - Original activity parameters
/// * `metrics` - Calculated metrics (pace, speed, intensity, efficiency)
/// * `max_hr` - Determined maximum heart rate
/// * `max_hr_source` - Source description for max HR
///
/// # Returns
/// Complete `UniversalResponse` with results and metadata
fn build_metrics_response(
    params: &ActivityParameters,
    metrics: &CalculatedMetrics,
    max_hr: f64,
    max_hr_source: &str,
) -> UniversalResponse {
    use limits;

    UniversalResponse {
        success: true,
        result: Some(serde_json::json!({
            "pace": metrics.pace,
            "speed": metrics.speed,
            "intensity_score": metrics.intensity_score,
            "efficiency_score": metrics.efficiency_score,
            "max_hr_used": max_hr,
            "max_hr_source": max_hr_source,
            "metrics_summary": {
                "distance_km": params.distance / METERS_PER_KILOMETER,
                "duration_minutes": params.duration / limits::SECONDS_PER_MINUTE,
                "elevation_meters": params.elevation_gain,
                "average_heart_rate": params.heart_rate
            }
        })),
        error: None,
        metadata: Some({
            let mut map = HashMap::new();
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
                serde_json::Value::Bool(
                    params.max_hr_provided.is_some() || params.user_age.is_some(),
                ),
            );
            map
        }),
    }
}

/// Fetch activity from provider and calculate metrics (helper for `activity_id` path)
async fn fetch_and_calculate_metrics(
    executor: &UniversalToolExecutor,
    request: &UniversalRequest,
    activity_id: &str,
    provider_name: &str,
    user_uuid: uuid::Uuid,
) -> Result<UniversalResponse, ProtocolError> {
    // Get valid token
    let token_data = match executor
        .auth_service
        .get_valid_token(user_uuid, provider_name, request.tenant_id.as_deref())
        .await
    {
        Ok(Some(token)) => token,
        Ok(None) => {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "No valid token for {provider_name}. Please connect using the connect_provider tool first."
                )),
                metadata: None,
            });
        }
        Err(e) => {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            });
        }
    };

    // Create configured provider
    let provider = executor
        .resources
        .provider_registry
        .create_provider(provider_name)
        .map_err(|e| ProtocolError::InternalError(format!("Failed to create provider: {e}")))?;

    // Safe: OAuth credential clones needed for struct field ownership
    let credentials = OAuth2Credentials {
        client_id: executor
            .resources
            .config
            .oauth
            .strava
            .client_id
            .clone() // Safe: OAuth config ownership
            .unwrap_or_default(),
        client_secret: executor
            .resources
            .config
            .oauth
            .strava
            .client_secret
            .clone() // Safe: OAuth config ownership
            .unwrap_or_default(),
        access_token: Some(token_data.access_token.clone()),
        refresh_token: Some(token_data.refresh_token.clone()),
        expires_at: Some(token_data.expires_at),
        // Inline scope to avoid feature-gated constant
        scopes: "activity:read_all".split(',').map(str::to_owned).collect(),
    };

    provider.set_credentials(credentials).await.map_err(|e| {
        ProtocolError::ConfigurationError(format!("Failed to set provider credentials: {e}"))
    })?;

    // Fetch activity from provider
    let activity = provider.get_activity(activity_id).await.map_err(|e| {
        ProtocolError::ExecutionFailed(format!("Failed to fetch activity {activity_id}: {e}"))
    })?;

    // Convert Activity model to parameters format
    let mut request_with_activity = request.clone();
    if let Some(params_obj) = request_with_activity.parameters.as_object_mut() {
        params_obj.insert(
            "activity".to_owned(),
            serde_json::json!({
                "distance": activity.distance_meters(),
                "duration": activity.duration_seconds(),
                "elevation_gain": activity.elevation_gain(),
                "average_heart_rate": activity.average_heart_rate(),
            }),
        );
    } else {
        return Err(ProtocolError::InvalidParameters(
            "parameters must be a JSON object".to_owned(),
        ));
    }

    // Parse parameters from converted activity
    let params = parse_activity_parameters(&request_with_activity)?;
    let (max_hr, max_hr_source) = determine_max_heart_rate(params.max_hr_provided, params.user_age);
    let metrics = calculate_activity_metrics(&params, max_hr);

    Ok(build_metrics_response(
        &params,
        &metrics,
        max_hr,
        &max_hr_source,
    ))
}

/// Handle `calculate_metrics` tool - calculate custom fitness metrics (async)
///
/// # Errors
/// Returns `ProtocolError` if activity parameter is missing or calculation fails
pub async fn handle_calculate_metrics(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Result<UniversalResponse, ProtocolError> {
    let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

    // Extract output format parameter: "json" (default) or "toon"
    let output_format = extract_output_format(&request);

    // Check if activity_id is provided (schema-compliant path)
    if let Some(activity_id) = request
        .parameters
        .get("activity_id")
        .and_then(serde_json::Value::as_str)
    {
        let provider_name = request
            .parameters
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ProtocolError::InvalidParameters(
                    "provider parameter required when using activity_id".to_owned(),
                )
            })?;

        let result =
            fetch_and_calculate_metrics(executor, &request, activity_id, provider_name, user_uuid)
                .await?;

        // Apply format transformation
        return Ok(apply_format_to_response(result, "metrics", output_format));
    }

    // Fallback path: activity object provided directly
    let params = parse_activity_parameters(&request)?;
    let (max_hr, max_hr_source) = determine_max_heart_rate(params.max_hr_provided, params.user_age);
    let metrics = calculate_activity_metrics(&params, max_hr);

    let result = build_metrics_response(&params, &metrics, max_hr, &max_hr_source);

    // Apply format transformation
    Ok(apply_format_to_response(result, "metrics", output_format))
}

/// Generate insights and recommendations from activity data
fn generate_activity_insights(activity: &Activity) -> (Vec<String>, Vec<&'static str>) {
    let mut insights = Vec::new();
    let mut recommendations = Vec::new();

    // Analyze distance
    if let Some(distance) = activity.distance_meters() {
        let km = distance / METERS_PER_KILOMETER;
        insights.push(format!("Activity covered {km:.2} km"));
        if km > ACHIEVEMENT_DISTANCE_THRESHOLD_KM {
            recommendations.push("Great long-distance effort! Ensure proper recovery time");
        }
    }

    // Analyze elevation
    if let Some(elevation) = activity.elevation_gain() {
        insights.push(format!("Total elevation gain: {elevation:.0} meters"));
        if elevation > ACHIEVEMENT_ELEVATION_THRESHOLD_M {
            recommendations.push("Significant elevation - consider targeted hill training");
        }
    }

    // Analyze heart rate
    if let Some(avg_hr) = activity.average_heart_rate() {
        insights.push(format!("Average heart rate: {avg_hr} bpm"));
        if avg_hr > HIGH_INTENSITY_HR_THRESHOLD {
            recommendations.push("High-intensity effort detected - monitor recovery");
        }
    }

    // Analyze calories
    if let Some(calories) = activity.calories() {
        insights.push(format!("Calories burned: {calories}"));
    }

    (insights, recommendations)
}

/// Build intelligence response metadata
///
/// Creates metadata map with activity ID, user ID, tenant ID, and analysis type
/// for tracking and audit purposes.
///
/// # Arguments
/// * `activity_id` - Activity identifier
/// * `user_uuid` - User UUID
/// * `tenant_id` - Optional tenant identifier
///
/// # Returns
/// `HashMap` with metadata key-value pairs
fn build_intelligence_metadata(
    activity_id: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();
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
    metadata
}

/// Create intelligence analysis JSON response with optional MCP sampling
async fn create_intelligence_response(
    activity: &Activity,
    activity_id: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
    sampling_peer: Option<&Arc<SamplingPeer>>,
) -> UniversalResponse {
    // Try MCP sampling first if available (uses client's LLM)
    if let Some(peer) = sampling_peer {
        match generate_activity_intelligence_via_sampling(peer, activity).await {
            Ok(llm_analysis) => {
                info!("Generated activity intelligence using MCP sampling");
                return UniversalResponse {
                    success: true,
                    result: Some(llm_analysis),
                    error: None,
                    metadata: Some({
                        let mut map = HashMap::new();
                        map.insert(
                            "activity_id".to_owned(),
                            serde_json::Value::String(activity_id.to_owned()),
                        );
                        map.insert(
                            "user_id".to_owned(),
                            serde_json::Value::String(user_uuid.to_string()),
                        );
                        if let Some(tid) = tenant_id.clone() {
                            map.insert("tenant_id".to_owned(), serde_json::Value::String(tid));
                        }
                        map.insert(
                            "analysis_source".to_owned(),
                            serde_json::Value::String("mcp_sampling".to_owned()),
                        );
                        map
                    }),
                };
            }
            Err(e) => {
                warn!(
                    "MCP sampling failed, falling back to static analysis: {}",
                    e
                );
            }
        }
    }

    // Fall back to static analysis
    let (insights, recommendations) = generate_activity_insights(activity);

    let summary = format!(
        "{:?} activity completed. {} insights generated.",
        activity.sport_type(),
        insights.len()
    );

    let duration_minutes = f64::from(
        u32::try_from(activity.duration_seconds().min(u64::from(u32::MAX))).unwrap_or(u32::MAX),
    ) / 60.0;

    let analysis = serde_json::json!({
        "activity_id": activity_id,
        "activity_type": format!("{:?}", activity.sport_type()),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "intelligence": {
            "summary": summary,
            "insights": insights,
            "recommendations": recommendations,
            "performance_metrics": {
                "distance_km": activity.distance_meters().map(|d| d / METERS_PER_KILOMETER),
                "duration_minutes": Some(duration_minutes),
                "elevation_meters": activity.elevation_gain(),
                "average_heart_rate": activity.average_heart_rate(),
                "max_heart_rate": activity.max_heart_rate(),
                "calories": activity.calories()
            }
        }
    });

    let metadata = build_intelligence_metadata(activity_id, user_uuid, tenant_id);

    UniversalResponse {
        success: true,
        result: Some(analysis),
        error: None,
        metadata: Some(metadata),
    }
}

/// Fetch activity and create intelligence response
///
/// Retrieves activity data from provider and generates intelligence analysis.
/// Returns error response if activity fetch fails.
///
/// # Arguments
/// * `provider` - Configured activity provider
/// * `activity_id` - Activity identifier to fetch
/// * `user_uuid` - User UUID for response metadata
/// * `tenant_id` - Optional tenant identifier
///
/// # Returns
/// `UniversalResponse` with intelligence or error
async fn fetch_and_analyze_activity(
    provider: Box<dyn FitnessProvider>,
    activity_id: &str,
    user_uuid: uuid::Uuid,
    tenant_id: Option<String>,
    sampling_peer: Option<&Arc<SamplingPeer>>,
) -> UniversalResponse {
    match provider.get_activity(activity_id).await {
        Ok(activity) => {
            create_intelligence_response(
                &activity,
                activity_id,
                user_uuid,
                tenant_id,
                sampling_peer,
            )
            .await
        }
        Err(e) => {
            // Handle NotFound by auto-fetching recent activities
            if e.code == ErrorCode::ResourceNotFound {
                // Activity not found - fetch recent activities to show valid IDs
                match provider.get_activities(Some(5), None).await {
                    Ok(activities) if !activities.is_empty() => {
                        let activity_list: Vec<String> = activities
                            .iter()
                            .map(|a| {
                                format!(
                                    "- {} (ID: {}): {} - {:?}",
                                    a.start_date().format("%Y-%m-%d"),
                                    a.id(),
                                    a.name(),
                                    a.sport_type()
                                )
                            })
                            .collect();

                        let most_recent = &activities[0];

                        // Analyze the most recent activity automatically
                        let mut response = create_intelligence_response(
                            most_recent,
                            most_recent.id(),
                            user_uuid,
                            tenant_id,
                            None, // No sampling in fallback path
                        )
                        .await;

                        // Add auto-selection note to the result
                        if let Some(result) = response.result.as_mut() {
                            result["auto_selected"] = serde_json::json!({
                                "reason": format!("Activity '{activity_id}' not found"),
                                "selected_activity": most_recent.id(),
                                "selected_activity_name": most_recent.name(),
                                "selected_activity_date": most_recent.start_date().format("%Y-%m-%d").to_string(),
                                "available_activities": activity_list
                            });
                        }

                        return response;
                    }
                    Ok(_) => {
                        return UniversalResponse {
                            success: false,
                            result: None,
                            error: Some(format!("Activity '{activity_id}' not found and no activities available in your account.")),
                            metadata: None,
                        };
                    }
                    Err(fetch_err) => {
                        return UniversalResponse {
                            success: false,
                            result: None,
                            error: Some(format!("Activity '{activity_id}' not found. Failed to fetch available activities: {fetch_err}")),
                            metadata: None,
                        };
                    }
                }
            }

            // Other errors - generic message
            UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to fetch activity {activity_id}: {e}")),
                metadata: None,
            }
        }
    }
}

/// Fetch activities and analyze performance trends
///
/// Retrieves recent activities from provider and performs trend analysis
/// for the specified metric and timeframe.
///
/// # Arguments
/// * `provider` - Configured fitness provider
/// * `metric` - Metric to analyze (e.g., "pace", "distance")
/// * `timeframe` - Analysis timeframe (e.g., "week", "month")
/// * `user_uuid` - User UUID for response metadata
///
/// # Returns
/// `UniversalResponse` with trend analysis or error
async fn fetch_and_analyze_trends(
    provider: Box<dyn FitnessProvider>,
    metric: &str,
    timeframe: &str,
    user_uuid: uuid::Uuid,
) -> UniversalResponse {
    use MAX_ACTIVITY_LIMIT;

    match provider
        .get_activities(Some(MAX_ACTIVITY_LIMIT), None)
        .await
    {
        Ok(activities) => {
            let analysis = analyze_performance_trend(&activities, metric, timeframe);

            UniversalResponse {
                success: true,
                result: Some(analysis),
                error: None,
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert(
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map
                }),
            }
        }
        Err(e) => UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Failed to fetch activities: {e}")),
            metadata: None,
        },
    }
}

/// Fetch activities and detect patterns
///
/// Retrieves recent activities from provider and performs pattern detection
/// based on the specified pattern type.
///
/// # Arguments
/// * `provider` - Configured fitness provider
/// * `pattern_type` - Type of pattern to detect (e.g., "`weekly_schedule`", "overtraining")
/// * `user_uuid` - User UUID for response metadata
///
/// # Returns
/// `UniversalResponse` with pattern analysis or error
async fn fetch_and_detect_patterns(
    provider: Box<dyn FitnessProvider>,
    pattern_type: &str,
    user_uuid: uuid::Uuid,
) -> UniversalResponse {
    use DEFAULT_ACTIVITY_LIMIT;

    match provider
        .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
        .await
    {
        Ok(activities) => {
            let analysis = detect_activity_patterns(&activities, pattern_type);

            UniversalResponse {
                success: true,
                result: Some(analysis),
                error: None,
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert(
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map
                }),
            }
        }
        Err(e) => UniversalResponse {
            success: false,
            result: None,
            error: Some(format!("Failed to fetch activities: {e}")),
            metadata: None,
        },
    }
}

/// Build response for missing OAuth token
///
/// Returns error response indicating user needs to connect Strava account.
/// Includes `authentication_required` flag in metadata for client guidance.
///
/// # Returns
/// Handle `get_activity_intelligence` tool - get AI analysis for activity (async)
///
/// # Errors
/// Returns `ProtocolError` if `activity_id` parameter is missing or validation fails
#[must_use]
pub fn handle_get_activity_intelligence(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_activity_intelligence cancelled by user".to_owned(),
                ));
            }
        }

        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: activity_id".to_owned())
            })?;

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "get_activity_intelligence cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some("Authenticated - analyzing activity...".to_owned()),
                    );
                }

                // Check cancellation before analysis
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "get_activity_intelligence cancelled before analysis".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_analyze_activity(
                    provider,
                    activity_id,
                    user_uuid,
                    request.tenant_id,
                    executor.resources.sampling_peer.as_ref(),
                )
                .await;

                // Report completion on success
                if result.success {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Activity intelligence retrieved".to_owned()),
                        );
                    }
                }

                // Apply format transformation
                Ok(apply_format_to_response(
                    result,
                    "intelligence",
                    output_format,
                ))
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `analyze_performance_trends` tool - analyze performance over time
#[must_use]
pub fn handle_analyze_performance_trends(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_performance_trends cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
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

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_performance_trends cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some(
                            "Authenticated - fetching activities for trend analysis...".to_owned(),
                        ),
                    );
                }

                // Check cancellation before analysis
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "analyze_performance_trends cancelled before analysis".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_analyze_trends(provider, metric, timeframe, user_uuid).await;

                // Report completion on success
                if result.success {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Performance trend analysis completed".to_owned()),
                        );
                    }
                }

                // Apply format transformation
                Ok(apply_format_to_response(result, "trends", output_format))
            }
            Err(response) => Ok(response),
        }
    })
}

/// Execute activity comparison with authenticated provider
async fn execute_activity_comparison(
    provider: Box<dyn FitnessProvider>,
    activity_id: &str,
    comparison_type: &str,
    compare_activity_id: Option<&str>,
    user_uuid: uuid::Uuid,
    request: &UniversalRequest,
) -> UniversalResponse {
    use DEFAULT_ACTIVITY_LIMIT;

    match provider.get_activity(activity_id).await {
        Ok(target_activity) => {
            // Report progress after getting target activity
            if let Some(reporter) = &request.progress_reporter {
                reporter.report(
                    66.0,
                    Some(100.0),
                    Some("Target activity retrieved - comparing...".to_owned()),
                );
            }

            let all_activities = provider
                .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                .await
                .unwrap_or_default();

            let comparison = compare_activity_logic(
                &target_activity,
                &all_activities,
                comparison_type,
                compare_activity_id,
            );

            // Report completion
            if let Some(reporter) = &request.progress_reporter {
                reporter.report(
                    100.0,
                    Some(100.0),
                    Some("Comparison completed successfully".to_owned()),
                );
            }

            UniversalResponse {
                success: true,
                result: Some(comparison),
                error: None,
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert(
                        "user_id".to_owned(),
                        serde_json::Value::String(user_uuid.to_string()),
                    );
                    map
                }),
            }
        }
        Err(e) => {
            let error_message = if e.code == ErrorCode::ResourceNotFound {
                format!(
                    "Activity '{activity_id}' not found. Please use get_activities to retrieve your activity IDs first, then use compare_activities with a valid ID from the list."
                )
            } else {
                format!("Failed to fetch activity {activity_id}: {e}")
            };

            UniversalResponse {
                success: false,
                result: None,
                error: Some(error_message),
                metadata: None,
            }
        }
    }
}

/// Handle `compare_activities` tool - compare two activities
#[must_use]
pub fn handle_compare_activities(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "compare_activities cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
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

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        33.0,
                        Some(100.0),
                        Some("Authenticated - fetching activities for comparison...".to_owned()),
                    );
                }

                let result = execute_activity_comparison(
                    provider,
                    activity_id,
                    comparison_type,
                    compare_activity_id,
                    user_uuid,
                    &request,
                )
                .await;

                // Apply format transformation
                Ok(apply_format_to_response(
                    result,
                    "comparison",
                    output_format,
                ))
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `detect_patterns` tool - detect patterns in activity data
#[must_use]
pub fn handle_detect_patterns(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "detect_patterns cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let pattern_type = request
            .parameters
            .get("pattern_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidRequest("Missing required parameter: pattern_type".to_owned())
            })?;

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                25.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "detect_patterns cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        50.0,
                        Some(100.0),
                        Some("Authenticated - analyzing activities for patterns...".to_owned()),
                    );
                }

                // Check cancellation before pattern detection
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "detect_patterns cancelled before analysis".to_owned(),
                        ));
                    }
                }

                let result = fetch_and_detect_patterns(provider, pattern_type, user_uuid).await;

                // Report completion on success
                if result.success {
                    if let Some(reporter) = &request.progress_reporter {
                        reporter.report(
                            100.0,
                            Some(100.0),
                            Some("Pattern detection completed".to_owned()),
                        );
                    }
                }

                // Apply format transformation
                Ok(apply_format_to_response(result, "patterns", output_format))
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `generate_recommendations` tool - generate training recommendations
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_generate_recommendations(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;
        use DEFAULT_ACTIVITY_LIMIT;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "generate_recommendations cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let recommendation_type = request
            .parameters
            .get("recommendation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                20.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "generate_recommendations cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        40.0,
                        Some(100.0),
                        Some("Authenticated - fetching activities...".to_owned()),
                    );
                }

                // Check cancellation before provider creation
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "generate_recommendations cancelled before fetch".to_owned(),
                        ));
                    }
                }

                match provider
                    .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                    .await
                {
                    Ok(activities) => {
                        // Report progress before generating recommendations
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                70.0,
                                Some(100.0),
                                Some("Generating training recommendations...".to_owned()),
                            );
                        }

                        // Try to use MCP sampling if available, otherwise use static analysis
                        let analysis = if let Some(sampling_peer) =
                            &executor.resources.sampling_peer
                        {
                            // Use MCP sampling (client's LLM) to generate personalized recommendations
                            match generate_recommendations_via_sampling(
                                sampling_peer,
                                &activities,
                                recommendation_type,
                            )
                            .await
                            {
                                Ok(llm_recommendations) => llm_recommendations,
                                Err(e) => {
                                    warn!("MCP sampling failed, falling back to static recommendations: {}", e);
                                    generate_training_recommendations(
                                        &activities,
                                        recommendation_type,
                                    )
                                }
                            }
                        } else {
                            // Fall back to static recommendations
                            generate_training_recommendations(&activities, recommendation_type)
                        };

                        // Report completion
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                100.0,
                                Some(100.0),
                                Some("Recommendations generated successfully".to_owned()),
                            );
                        }

                        let result = UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        };

                        // Apply format transformation
                        Ok(apply_format_to_response(
                            result,
                            "recommendations",
                            output_format,
                        ))
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `calculate_fitness_score` tool - calculate overall fitness score
///
/// Supports cross-provider integration:
/// - Use `provider` to specify where to fetch activity data (default: configured default provider)
/// - Use `sleep_provider` to optionally fetch recovery data from a different provider
///
/// When `sleep_provider` is specified, recovery quality factors into the fitness score:
/// - Excellent recovery (90-100): +5% fitness score bonus
/// - Good recovery (70-89): No adjustment
/// - Poor recovery (<70): -5% to -10% penalty
///
/// # Parameters
/// - `provider` (optional): Activity provider (default: configured default)
/// - `sleep_provider` (optional): Sleep/recovery provider for cross-provider analysis
/// - `timeframe` (optional): `month`, `last_90_days`, or `all_time`
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_calculate_fitness_score(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;
        use DEFAULT_ACTIVITY_LIMIT;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "calculate_fitness_score cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("month");

        // Extract optional sleep_provider for cross-provider recovery analysis
        let sleep_provider = request
            .parameters
            .get("sleep_provider")
            .and_then(|v| v.as_str());

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                20.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "calculate_fitness_score cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        40.0,
                        Some(100.0),
                        Some("Authenticated - fetching activities...".to_owned()),
                    );
                }

                // Check cancellation before provider creation
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "calculate_fitness_score cancelled before fetch".to_owned(),
                        ));
                    }
                }

                match provider
                    .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                    .await
                {
                    Ok(activities) => {
                        // Report progress before calculation
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                70.0,
                                Some(100.0),
                                Some("Calculating fitness metrics...".to_owned()),
                            );
                        }

                        let mut analysis = calculate_fitness_metrics(&activities, timeframe);

                        // If sleep_provider is specified, fetch recovery data and adjust score
                        let recovery_info = if let Some(sleep_provider_name) = sleep_provider {
                            match fetch_and_calculate_recovery_adjustment(
                                executor,
                                user_uuid,
                                request.tenant_id.as_deref(),
                                sleep_provider_name,
                                &mut analysis,
                            )
                            .await
                            {
                                Ok(info) => Some(info),
                                Err(err_msg) => {
                                    warn!(
                                        sleep_provider = sleep_provider_name,
                                        error = %err_msg,
                                        "Failed to fetch recovery data, proceeding without adjustment"
                                    );
                                    None
                                }
                            }
                        } else {
                            None
                        };

                        // Report completion
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                100.0,
                                Some(100.0),
                                Some("Fitness score calculated".to_owned()),
                            );
                        }

                        // Add recovery and provider info to response
                        if let Some(obj) = analysis.as_object_mut() {
                            if let Some(ref info) = recovery_info {
                                obj.insert(
                                    "recovery_adjustment".to_owned(),
                                    serde_json::json!({
                                        "recovery_score": info.recovery_score,
                                        "adjustment_factor": info.adjustment_factor,
                                        "sleep_provider": info.provider_name,
                                    }),
                                );
                            }
                            obj.insert(
                                "providers_used".to_owned(),
                                serde_json::json!({
                                    "activity_provider": provider_name,
                                    "sleep_provider": sleep_provider,
                                }),
                            );
                        }

                        let result = UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map.insert(
                                    "activity_provider".to_owned(),
                                    serde_json::Value::String(provider_name),
                                );
                                if let Some(sp) = sleep_provider {
                                    map.insert(
                                        "sleep_provider".to_owned(),
                                        serde_json::Value::String(sp.to_owned()),
                                    );
                                }
                                if recovery_info.is_some() {
                                    map.insert(
                                        "recovery_factored".to_owned(),
                                        serde_json::Value::Bool(true),
                                    );
                                }
                                map
                            }),
                        };

                        // Apply format transformation
                        Ok(apply_format_to_response(
                            result,
                            "fitness_score",
                            output_format,
                        ))
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `predict_performance` tool - predict future performance
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_predict_performance(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;
        use DEFAULT_ACTIVITY_LIMIT;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "predict_performance cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let target_sport = request
            .parameters
            .get("target_sport")
            .and_then(|v| v.as_str())
            .unwrap_or("Run");

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                20.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "predict_performance cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        40.0,
                        Some(100.0),
                        Some("Authenticated - fetching activities...".to_owned()),
                    );
                }

                // Check cancellation before provider creation
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "predict_performance cancelled before fetch".to_owned(),
                        ));
                    }
                }

                match provider
                    .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                    .await
                {
                    Ok(activities) => {
                        // Report progress before prediction
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                70.0,
                                Some(100.0),
                                Some("Predicting race performance...".to_owned()),
                            );
                        }

                        let prediction = predict_race_performance(&activities, target_sport);

                        // Report completion
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                100.0,
                                Some(100.0),
                                Some("Performance prediction completed".to_owned()),
                            );
                        }

                        let result = UniversalResponse {
                            success: true,
                            result: Some(prediction),
                            error: None,
                            metadata: Some({
                                let mut map = HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map
                            }),
                        };

                        // Apply format transformation
                        Ok(apply_format_to_response(
                            result,
                            "prediction",
                            output_format,
                        ))
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(response) => Ok(response),
        }
    })
}

/// Handle `analyze_training_load` tool - analyze training load and fatigue
///
/// Supports cross-provider integration:
/// - Use `provider` to specify where to fetch activity data (default: configured default provider)
/// - Use `sleep_provider` to optionally fetch recovery data from a different provider
///
/// When `sleep_provider` is specified, adds recovery context to training load analysis:
/// - Sleep quality score and HRV data
/// - Recovery status interpretation
/// - Recommendations adjusted for recovery state
///
/// # Parameters
/// - `provider` (optional): Activity provider (default: configured default)
/// - `sleep_provider` (optional): Sleep/recovery provider for cross-provider analysis
/// - `timeframe` (optional): "week", "month", etc.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn handle_analyze_training_load(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>> {
    Box::pin(async move {
        use parse_user_id_for_protocol;
        use DEFAULT_ACTIVITY_LIMIT;

        // Check cancellation at start
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_training_load cancelled by user".to_owned(),
                ));
            }
        }

        let provider_name = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .map_or_else(default_provider, String::from);
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;
        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("week");

        // Extract optional sleep_provider for cross-provider recovery analysis
        let sleep_provider = request
            .parameters
            .get("sleep_provider")
            .and_then(|v| v.as_str());

        // Extract output format parameter: "json" (default) or "toon"
        let output_format = extract_output_format(&request);

        // Report progress - starting authentication
        if let Some(reporter) = &request.progress_reporter {
            reporter.report(
                20.0,
                Some(100.0),
                Some("Checking authentication...".to_owned()),
            );
        }

        // Check cancellation before auth
        if let Some(token) = &request.cancellation_token {
            if token.is_cancelled().await {
                return Err(ProtocolError::OperationCancelled(
                    "analyze_training_load cancelled before authentication".to_owned(),
                ));
            }
        }

        match executor
            .auth_service
            .create_authenticated_provider(&provider_name, user_uuid, request.tenant_id.as_deref())
            .await
        {
            Ok(provider) => {
                // Report progress after auth
                if let Some(reporter) = &request.progress_reporter {
                    reporter.report(
                        40.0,
                        Some(100.0),
                        Some("Authenticated - fetching activities...".to_owned()),
                    );
                }

                // Check cancellation before provider creation
                if let Some(token) = &request.cancellation_token {
                    if token.is_cancelled().await {
                        return Err(ProtocolError::OperationCancelled(
                            "analyze_training_load cancelled before fetch".to_owned(),
                        ));
                    }
                }

                match provider
                    .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                    .await
                {
                    Ok(activities) => {
                        // Report progress before analysis
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                70.0,
                                Some(100.0),
                                Some(format!(
                                    "Analyzing training load for {} activities...",
                                    activities.len()
                                )),
                            );
                        }

                        let mut analysis = analyze_detailed_training_load(&activities, timeframe);

                        // If sleep_provider is specified, fetch recovery context
                        let recovery_context = if let Some(sleep_provider_name) = sleep_provider {
                            match fetch_recovery_context_for_training_load(
                                executor,
                                user_uuid,
                                request.tenant_id.as_deref(),
                                sleep_provider_name,
                            )
                            .await
                            {
                                Ok(context) => {
                                    // Add recovery context to analysis
                                    if let Some(obj) = analysis.as_object_mut() {
                                        obj.insert(
                                            "recovery_context".to_owned(),
                                            serde_json::json!({
                                                "sleep_quality_score": context.sleep_quality_score,
                                                "recovery_status": context.recovery_status,
                                                "hrv_available": context.hrv_rmssd.is_some(),
                                                "hrv_rmssd": context.hrv_rmssd,
                                                "sleep_hours": context.sleep_hours,
                                                "sleep_provider": sleep_provider_name,
                                            }),
                                        );
                                    }
                                    Some(context)
                                }
                                Err(err_msg) => {
                                    warn!(
                                        sleep_provider = sleep_provider_name,
                                        error = %err_msg,
                                        "Failed to fetch recovery context, proceeding without"
                                    );
                                    None
                                }
                            }
                        } else {
                            None
                        };

                        // Add provider info
                        if let Some(obj) = analysis.as_object_mut() {
                            obj.insert(
                                "providers_used".to_owned(),
                                serde_json::json!({
                                    "activity_provider": provider_name,
                                    "sleep_provider": sleep_provider,
                                }),
                            );
                        }

                        // Report completion
                        if let Some(reporter) = &request.progress_reporter {
                            reporter.report(
                                100.0,
                                Some(100.0),
                                Some("Training load analysis completed".to_owned()),
                            );
                        }

                        let result = UniversalResponse {
                            success: true,
                            result: Some(analysis),
                            error: None,
                            metadata: Some({
                                let mut map = HashMap::new();
                                map.insert(
                                    "user_id".to_owned(),
                                    serde_json::Value::String(user_uuid.to_string()),
                                );
                                map.insert(
                                    "activity_provider".to_owned(),
                                    serde_json::Value::String(provider_name),
                                );
                                if let Some(sp) = sleep_provider {
                                    map.insert(
                                        "sleep_provider".to_owned(),
                                        serde_json::Value::String(sp.to_owned()),
                                    );
                                }
                                if recovery_context.is_some() {
                                    map.insert(
                                        "recovery_context_included".to_owned(),
                                        serde_json::Value::Bool(true),
                                    );
                                }
                                map
                            }),
                        };

                        // Apply format transformation
                        Ok(apply_format_to_response(
                            result,
                            "training_load",
                            output_format,
                        ))
                    }
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch activities: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(response) => Ok(response),
        }
    })
}

// ============================================================================
// Helper Functions for Performance Trend Analysis
// ============================================================================

/// Analyze performance trend for a specific metric over time
fn analyze_performance_trend(
    activities: &[Activity],
    metric: &str,
    timeframe: &str,
) -> serde_json::Value {
    use SafeMetricExtractor;

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
    let filtered_activities: Vec<Activity> = activities
        .iter()
        .filter(|a| a.start_date() >= cutoff_date)
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
            "insights": [format!("Metric '{metric}' not available in enough activities")]
        });
    };

    if data_points_with_timestamp.len() < 2 {
        return serde_json::json!({
            "metric": metric,
            "timeframe": timeframe,
            "trend": "insufficient_data",
            "activities_analyzed": filtered_activities.len(),
            "insights": [format!("Metric '{metric}' not available in enough activities")]
        });
    }

    // Convert to TrendDataPoint format and perform regression
    compute_trend_statistics(metric, timeframe, metric_type, &data_points_with_timestamp)
}

/// Compute trend statistics from data points
fn compute_trend_statistics(
    metric: &str,
    timeframe: &str,
    metric_type: MetricType,
    data_points_with_timestamp: &[(chrono::DateTime<chrono::Utc>, f64)],
) -> serde_json::Value {
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
    // Cast is safe: data point count far below f64 precision limit (2^53)
    #[allow(clippy::cast_precision_loss)] // Safe: realistic data point counts
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
fn parse_metric_type(metric: &str) -> Result<MetricType, String> {
    use MetricType;
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
    target: &Activity,
    all_activities: &[Activity],
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
    target: &Activity,
    all_activities: &[Activity],
) -> serde_json::Value {
    // Find similar activities (same sport, similar distance/duration)
    let similar: Vec<&Activity> = all_activities
        .iter()
        .filter(|a| {
            a.id() != target.id()
                && a.sport_type() == target.sport_type()
                && is_similar_distance(a.distance_meters(), target.distance_meters())
        })
        .take(5)
        .collect();

    if similar.is_empty() {
        return serde_json::json!({
            "activity_id": target.id(),
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
    let target_hr = target.average_heart_rate().map(f64::from);

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

    if let (Some(target_elev), Some(avg_elev)) = (target.elevation_gain(), avg_elevation) {
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
        "activity_id": target.id(),
        "comparison_type": "similar_activities",
        "comparison_count": similar.len(),
        "sport_type": format!("{:?}", target.sport_type()),
        "comparisons": comparisons,
        "insights": insights,
    })
}

/// Compare activity with personal records
fn compare_with_personal_records(
    target: &Activity,
    all_activities: &[Activity],
) -> serde_json::Value {
    // Find same sport activities
    let same_sport: Vec<&Activity> = all_activities
        .iter()
        .filter(|a| a.sport_type() == target.sport_type())
        .collect();

    if same_sport.is_empty() {
        return serde_json::json!({
            "activity_id": target.id(),
            "comparison_type": "pr_comparison",
            "insights": ["No other activities of this sport type found"],
        });
    }

    let mut pr_comparisons = Vec::new();
    let mut insights = Vec::new();

    // Compare with longest distance
    if let Some(distance) = target.distance_meters() {
        let max_distance = same_sport
            .iter()
            .filter_map(|a| a.distance_meters())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

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
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

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
    if let Some(power) = target.average_power() {
        let max_power = same_sport.iter().filter_map(|a| a.average_power()).max();

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
        "activity_id": target.id(),
        "comparison_type": "pr_comparison",
        "sport_type": format!("{:?}", target.sport_type()),
        "pr_comparisons": pr_comparisons,
        "insights": insights,
    })
}

/// Compare activity with a specific activity by ID
fn compare_with_specific_activity(
    target: &Activity,
    all_activities: &[Activity],
    compare_id: &str,
) -> serde_json::Value {
    // Find the specific activity to compare with
    let compare_activity = all_activities.iter().find(|a| a.id() == compare_id);

    let Some(compare) = compare_activity else {
        return serde_json::json!({
            "activity_id": target.id(),
            "comparison_type": "specific_activity",
            "error": format!("Activity with ID '{compare_id}' not found"),
            "insights": [format!("Could not find activity '{compare_id}' for comparison")],
        });
    };

    // Calculate metrics for both activities
    let target_pace = calculate_pace(target);
    let compare_pace = calculate_pace(compare);
    let target_hr = target.average_heart_rate().map(f64::from);
    let compare_hr = compare.average_heart_rate().map(f64::from);

    let mut comparisons = Vec::new();
    let mut insights = Vec::new();

    // Distance comparison
    if let (Some(target_dist), Some(compare_dist)) =
        (target.distance_meters(), compare.distance_meters())
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
    let duration_diff_pct = ((target.duration_seconds() as f64
        - compare.duration_seconds() as f64)
        / compare.duration_seconds() as f64)
        * 100.0;
    comparisons.push(serde_json::json!({
        "metric": "duration",
        "current": target.duration_seconds(),
        "comparison": compare.duration_seconds(),
        "difference_percent": duration_diff_pct,
    }));

    // Elevation comparison
    if let (Some(target_elev), Some(compare_elev)) =
        (target.elevation_gain(), compare.elevation_gain())
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
    if let (Some(target_power), Some(compare_power)) =
        (target.average_power(), compare.average_power())
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
        "activity_id": target.id(),
        "comparison_type": "specific_activity",
        "comparison_activity_id": compare_id,
        "comparison_activity_name": compare.name(),
        "sport_type": format!("{:?}", target.sport_type()),
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
fn calculate_pace(activity: &Activity) -> Option<f64> {
    if let Some(distance) = activity.distance_meters() {
        if distance > 0.0 && activity.duration_seconds() > 0 {
            #[allow(clippy::cast_precision_loss)]
            let seconds_per_km = (activity.duration_seconds() as f64 / distance) * METERS_PER_KM;
            return Some(seconds_per_km / 60.0); // convert to min/km
        }
    }
    None
}

/// Calculate average pace from activities
fn calculate_average_pace(activities: &[&Activity]) -> Option<f64> {
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
fn calculate_average_hr(activities: &[&Activity]) -> Option<f64> {
    let hrs: Vec<f64> = activities
        .iter()
        .filter_map(|a| a.average_heart_rate().map(f64::from))
        .collect();
    if hrs.is_empty() {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let avg = hrs.iter().sum::<f64>() / hrs.len() as f64;
    Some(avg)
}

/// Calculate average elevation from activities
fn calculate_average_elevation(activities: &[&Activity]) -> Option<f64> {
    let elevs: Vec<f64> = activities
        .iter()
        .filter_map(|a| a.elevation_gain())
        .collect();
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
fn detect_activity_patterns(activities: &[Activity], pattern_type: &str) -> serde_json::Value {
    use PatternDetector;

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
fn format_weekly_schedule(pattern: &WeeklySchedulePattern) -> serde_json::Value {
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
fn format_hard_easy_pattern(pattern: &HardEasyPattern) -> serde_json::Value {
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
fn format_volume_progression(pattern: &VolumeProgressionPattern) -> serde_json::Value {
    use VolumeTrend;

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
fn format_overtraining_signals(signals: &OvertrainingSignals) -> serde_json::Value {
    use RiskLevel;

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

/// Generate activity intelligence via MCP sampling
///
/// Analyzes a single activity using the client's LLM via MCP sampling for AI-powered insights.
///
/// # Arguments
/// * `sampling_peer` - MCP sampling peer for LLM requests
/// * `activity` - Activity to analyze
///
/// # Returns
/// JSON response with LLM-generated activity analysis
///
/// # Errors
/// Returns error if sampling request fails or response is invalid
async fn generate_activity_intelligence_via_sampling(
    sampling_peer: &Arc<SamplingPeer>,
    activity: &Activity,
) -> AppResult<serde_json::Value> {
    use {Content, CreateMessageRequest, ModelPreferences, PromptMessage};

    // Prepare activity data for LLM analysis
    #[allow(clippy::cast_precision_loss)]
    let duration_min = activity.duration_seconds() as f64 / 60.0;
    let distance_km = activity.distance_meters().map(|d| d / 1000.0);
    let avg_pace = activity
        .average_speed()
        .map(|s| if s > 0.0 { 1000.0 / (s * 60.0) } else { 0.0 });

    let activity_summary = format!(
        "Activity Type: {:?}\n\
         Duration: {duration_min:.1} minutes\n\
         Distance: {}\n\
         Average Pace: {}\n\
         Average Heart Rate: {}\n\
         Calories: {}",
        activity.sport_type(),
        distance_km.map_or_else(|| "N/A".to_owned(), |d| format!("{d:.2} km")),
        avg_pace.map_or_else(|| "N/A".to_owned(), |p| format!("{p:.2} min/km")),
        activity
            .average_heart_rate()
            .map_or_else(|| "N/A".to_owned(), |hr| format!("{hr} bpm")),
        activity
            .calories()
            .map_or_else(|| "N/A".to_owned(), |c| c.to_string())
    );

    // Create prompt for LLM
    let prompt = format!(
        "You are an expert fitness coach analyzing an athlete's activity.\n\n\
         {activity_summary}\n\n\
         Provide AI-powered insights about this activity focusing on:\n\
         1. Performance analysis (pacing, effort level)\n\
         2. Training effectiveness\n\
         3. Specific recommendations for improvement\n\
         4. Recovery suggestions\n\n\
         Format your response as JSON with this structure:\n\
         {{\n\
           \"summary\": \"brief overall assessment\",\n\
           \"insights\": [\"insight 1\", \"insight 2\", ...],\n\
           \"recommendations\": [\"recommendation 1\", \"recommendation 2\", ...],\n\
           \"performance_score\": \"rating out of 10\",\n\
           \"analysis_type\": \"ai_powered\"\n\
         }}"
    );

    // Send sampling request to client's LLM
    let request = CreateMessageRequest {
        messages: vec![PromptMessage::user(Content::Text {
            text: prompt,
        })],
        model_preferences: Some(ModelPreferences {
            // Hint for high-quality model - client decides actual model
            hints: None,
            intelligence_priority: Some(0.9),
            cost_priority: None,
            speed_priority: None,
        }),
        max_tokens: 800,
        temperature: Some(0.7),
        system_prompt: Some("You are an expert fitness coach providing detailed activity analysis. Always respond with valid JSON.".to_owned()),
        include_context: None,
        stop_sequences: None,
        metadata: None,
    };

    let result = sampling_peer.create_message(request).await?;

    // Parse LLM response
    serde_json::from_str::<serde_json::Value>(&result.content.text).or_else(|_| {
        // Wrap non-JSON response
        Ok(serde_json::json!({
            "summary": result.content.text,
            "insights": [result.content.text],
            "recommendations": [],
            "analysis_type": "ai_powered",
            "source": "mcp_sampling"
        }))
    })
}

/// Generate training recommendations via MCP sampling
///
/// Sends activity data to the client's LLM via MCP sampling for AI-powered coaching advice.
/// Returns natural language recommendations based on training patterns.
///
/// # Arguments
/// * `sampling_peer` - MCP sampling peer for LLM requests
/// * `activities` - Recent activity data
/// * `recommendation_type` - Type of recommendations requested
///
/// # Returns
/// JSON response with LLM-generated training recommendations
///
/// # Errors
/// Returns error if sampling request fails or response is invalid
async fn generate_recommendations_via_sampling(
    sampling_peer: &Arc<SamplingPeer>,
    activities: &[Activity],
    recommendation_type: &str,
) -> AppResult<serde_json::Value> {
    use {Content, CreateMessageRequest, ModelPreferences, PromptMessage};

    // Prepare activity summary for LLM analysis
    let activity_summary = if activities.is_empty() {
        "No recent training data available.".to_owned()
    } else {
        let recent_count = activities.len().min(10);
        let recent_activities = &activities[..recent_count];

        let total_distance: f64 = recent_activities
            .iter()
            .filter_map(Activity::distance_meters)
            .sum();
        let total_duration: u64 = recent_activities
            .iter()
            .map(Activity::duration_seconds)
            .sum();
        let activity_types: Vec<String> = recent_activities
            .iter()
            .map(|a| format!("{:?}", a.sport_type()))
            .collect();

        {
            #[allow(clippy::cast_precision_loss)]
            let duration_hours = total_duration as f64 / 3600.0;
            #[allow(clippy::cast_precision_loss)]
            let activities_per_week = recent_count as f64 / 4.0;

            format!(
                "Recent training data ({recent_count} activities):\n\
                 - Total distance: {:.2} km\n\
                 - Total duration: {duration_hours:.1} hours\n\
                 - Activity types: {}\n\
                 - Activities per week: {activities_per_week:.1}",
                total_distance / 1000.0,
                activity_types.join(", ")
            )
        }
    };

    // Create prompt for LLM
    let prompt = format!(
        "You are an expert fitness coach analyzing training data.\n\n\
         {activity_summary}\n\n\
         Please provide {recommendation_type} training recommendations based on this data. \
         Focus on actionable advice for improving performance, preventing injury, \
         and optimizing training load. Format your response as JSON with the following structure:\n\
         {{\n\
           \"recommendation_type\": \"{recommendation_type}\",\n\
           \"recommendations\": [\"recommendation 1\", \"recommendation 2\", ...],\n\
           \"priority\": \"high/medium/low\",\n\
           \"reasoning\": \"brief explanation\"\n\
         }}"
    );

    // Send sampling request to client's LLM
    let request = CreateMessageRequest {
        messages: vec![PromptMessage::user(Content::Text {
            text: prompt,
        })],
        model_preferences: Some(ModelPreferences {
            // High intelligence priority - client decides actual model
            hints: None,
            intelligence_priority: Some(0.8),
            cost_priority: None,
            speed_priority: None,
        }),
        max_tokens: 1024,
        temperature: Some(0.7),
        system_prompt: Some("You are an expert fitness coach providing personalized training advice. Always respond with valid JSON.".to_owned()),
        include_context: None,
        stop_sequences: None,
        metadata: None,
    };

    let result = sampling_peer.create_message(request).await?;

    // Parse LLM response as JSON
    let response_text = &result.content.text;
    serde_json::from_str::<serde_json::Value>(response_text).or_else(|_| {
        // If LLM didn't return pure JSON, wrap the text in a response structure
        Ok(serde_json::json!({
            "recommendation_type": recommendation_type,
            "recommendations": [response_text],
            "priority": "medium",
            "reasoning": "Generated via MCP sampling",
            "source": "mcp_sampling"
        }))
    })
}

/// Generate personalized training recommendations
fn generate_training_recommendations(
    activities: &[Activity],
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
        .filter(|a| a.start_date() >= four_weeks_ago)
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
        "nutrition" => generate_nutrition_recommendations(&recent_activities),
        _ => generate_comprehensive_recommendations(&recent_activities),
    }
}

/// Generate weekly training plan recommendations using training load analysis
fn generate_training_plan_recommendations(activities: &[Activity]) -> serde_json::Value {
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
    load: &TrainingLoad,
    recommendations: &mut Vec<String>,
    priority: &mut &str,
    recovery_status: &mut &str,
    reasoning: &mut String,
) {
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
fn generate_recovery_recommendations(activities: &[Activity]) -> serde_json::Value {
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
fn generate_intensity_recommendations(activities: &[Activity]) -> serde_json::Value {
    use PatternDetector;

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
fn generate_goal_specific_recommendations(activities: &[Activity]) -> serde_json::Value {
    use HashMap;
    use PerformancePredictor;

    // Detect primary sport
    let mut sport_counts: HashMap<String, usize> = HashMap::new();
    for activity in activities {
        let sport = format!("{:?}", activity.sport_type());
        *sport_counts.entry(sport).or_insert(0) += 1;
    }

    let primary_sport = sport_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map_or("Unknown", |(sport, _)| sport.as_str());

    // Find best recent performance for predictions
    let best_performance = activities
        .iter()
        .filter(|a| a.distance_meters().is_some() && a.duration_seconds() > 0)
        .filter_map(|a| {
            let distance = a.distance_meters()?;
            #[allow(clippy::cast_precision_loss)]
            let time_secs = a.duration_seconds() as f64;
            if distance > 3_000.0 && distance < 50_000.0 && time_secs > 0.0 {
                Some((distance, time_secs))
            } else {
                None
            }
        })
        .max_by(|a, b| {
            let pace_a = a.1 / (a.0 / METERS_PER_KILOMETER);
            let pace_b = b.1 / (b.0 / METERS_PER_KILOMETER);
            pace_b.partial_cmp(&pace_a).unwrap_or(Ordering::Equal)
        });

    let mut recommendations = Vec::new();
    let priority = "medium";
    let mut race_predictions = None;

    // Generate race time predictions if we have performance data
    if let Some((distance, time)) = best_performance {
        if let Ok(predictions) = PerformancePredictor::generate_race_predictions(distance, time) {
            race_predictions = Some(serde_json::json!({
                "based_on": format!("{:.1}km in {}", distance / METERS_PER_KILOMETER, PerformancePredictor::format_time(time)),
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

/// Generate nutrition recommendations based on recent activity
/// Calculate activity nutrition metrics (duration, calories, intensity)
fn calculate_nutrition_metrics(activity: &Activity) -> (f64, f64, &'static str) {
    use time_constants;

    let duration_hours = f64::from(
        u32::try_from(activity.duration_seconds().min(u64::from(u32::MAX))).unwrap_or(u32::MAX),
    ) / time_constants::SECONDS_PER_HOUR_F64;

    let calories_burned = f64::from(activity.calories().unwrap_or_else(|| {
        let duration_mins = u32::try_from(activity.duration_seconds() / 60).unwrap_or(u32::MAX);
        duration_mins * 10
    }));

    let intensity = activity.average_heart_rate().map_or(
        if duration_hours > 1.5 {
            "moderate"
        } else {
            "low"
        },
        |avg_hr| {
            let avg_hr_f64 = f64::from(avg_hr);
            if avg_hr_f64 > 160.0 {
                "high"
            } else if avg_hr_f64 > 130.0 {
                "moderate"
            } else {
                "low"
            }
        },
    );

    (duration_hours, calories_burned, intensity)
}

/// Calculate macronutrient needs based on workout intensity and duration
fn calculate_macronutrient_needs(intensity: &str, duration_hours: f64) -> (f64, f64, f64) {
    let protein_g = if intensity == "high" || duration_hours > 1.5 {
        30.0 + (duration_hours * 5.0).min(20.0)
    } else {
        20.0 + (duration_hours * 5.0).min(15.0)
    };

    let carbs_g = duration_hours * 70.0;
    let hydration_ml = duration_hours * 750.0;

    (protein_g, carbs_g, hydration_ml)
}

/// Build meal suggestions based on workout intensity
fn build_meal_suggestions(intensity: &str) -> Vec<serde_json::Value> {
    let mut suggestions = vec![
        serde_json::json!({
            "option": "Quick Recovery Shake",
            "description": "Protein shake with banana and honey",
            "protein_g": 25,
            "carbs_g": 50,
            "timing": "Immediate (0-15 min)"
        }),
        serde_json::json!({
            "option": "Greek Yogurt Bowl",
            "description": "200g Greek yogurt with granola, berries, and honey",
            "protein_g": 20,
            "carbs_g": 60,
            "timing": "Within 30 minutes"
        }),
        serde_json::json!({
            "option": "Recovery Meal",
            "description": "Grilled chicken with sweet potato and vegetables",
            "protein_g": 35,
            "carbs_g": 50,
            "timing": "Within 2 hours"
        }),
    ];

    if intensity == "high" {
        suggestions.push(serde_json::json!({
            "option": "Endurance Option",
            "description": "Pasta with lean meat sauce and mixed salad",
            "protein_g": 30,
            "carbs_g": 80,
            "timing": "Within 2 hours"
        }));
    }

    suggestions
}

fn generate_nutrition_recommendations(activities: &[Activity]) -> serde_json::Value {
    let most_recent = activities.iter().max_by_key(|a| a.start_date());

    if most_recent.is_none() {
        return serde_json::json!({
            "recommendation_type": "nutrition",
            "priority": "medium",
            "reasoning": "No recent activity data available",
            "recommendations": [
                "Maintain balanced nutrition with adequate protein (1.6-2.2g/kg body weight)",
                "Stay hydrated throughout the day (2-3 liters water)",
                "Eat regular meals with complex carbohydrates, lean protein, and healthy fats"
            ],
        });
    }

    let Some(activity) = most_recent else {
        return serde_json::json!({
            "recommendations": ["No recent activities found for nutrition analysis"],
        });
    };

    let (duration_hours, calories_burned, intensity) = calculate_nutrition_metrics(activity);
    let (protein_g, carbs_g, hydration_ml) =
        calculate_macronutrient_needs(intensity, duration_hours);

    let mut recommendations = vec![
        format!(
            "Within 30 minutes: Consume {:.0}g protein and {:.0}g carbohydrates for optimal recovery",
            protein_g,
            carbs_g * 0.5
        ),
        format!(
            "Rehydrate with {:.0}-{:.0}ml of water or electrolyte drink",
            hydration_ml,
            hydration_ml * 1.3
        ),
    ];

    if intensity == "high" || duration_hours > 1.0 {
        recommendations.push(
            "Follow up with a complete meal within 2 hours to fully replenish glycogen stores"
                .to_owned(),
        );
    }

    let meal_suggestions = build_meal_suggestions(intensity);

    let mut key_insights = vec![
        format!(
            "Activity burned approximately {:.0} calories",
            calories_burned
        ),
        format!("Workout intensity: {intensity} - adjust nutrition accordingly"),
    ];

    if duration_hours > 1.5 {
        key_insights
            .push("Extended duration activity - prioritize carbohydrate replenishment".to_owned());
    }

    serde_json::json!({
        "recommendation_type": "nutrition",
        "priority": if intensity == "high" { "high" } else { "medium" },
        "reasoning": format!(
            "Based on {:.1} hour {intensity} intensity {:?} with {:.0} calories burned",
            duration_hours,
            activity.sport_type(),
            calories_burned
        ),
        "recovery_window": "Critical recovery period: 0-2 hours post-workout",
        "key_insights": key_insights,
        "recommendations": recommendations,
        "meal_suggestions": meal_suggestions,
        "macronutrient_targets": {
            "protein_g": protein_g.round(),
            "carbohydrates_g": carbs_g.round(),
            "hydration_ml": hydration_ml.round(),
        },
        "activity_summary": {
            "name": &activity.name(),
            "type": &activity.sport_type(),
            "duration_minutes": activity.duration_seconds() / 60,
            "distance_km": activity.distance_meters().map(|d| (d / 1000.0).round()),
            "calories": calories_burned.round(),
        }
    })
}

/// Generate comprehensive recommendations combining all analyses
fn generate_comprehensive_recommendations(activities: &[Activity]) -> serde_json::Value {
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
fn calculate_fitness_metrics(activities: &[Activity], timeframe: &str) -> serde_json::Value {
    use chrono::{Duration, Utc};
    use TrainingLoadCalculator;

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
        .filter(|a| a.start_date() >= cutoff_date)
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
fn calculate_consistency_score(activities: &[Activity]) -> f64 {
    use HashMap;

    if activities.is_empty() {
        return 0.0;
    }

    // Get the date range
    let first_date = activities.iter().map(Activity::start_date).min();
    let last_date = activities.iter().map(Activity::start_date).max();

    let (Some(first), Some(last)) = (first_date, last_date) else {
        return 0.0;
    };

    // Calculate total weeks spanned
    let weeks_spanned = ((last - first).num_days() / 7).max(1);

    // Group activities by week number (days since first / 7)
    let mut activities_per_week: HashMap<i64, u32> = HashMap::new();

    for activity in activities {
        let days_since_first = (activity.start_date() - first).num_days();
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
fn calculate_performance_trend(activities: &[Activity]) -> f64 {
    let activities_with_pace: Vec<_> = activities
        .iter()
        .filter(|a| a.distance_meters().is_some() && a.duration_seconds() > 0)
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
            let distance = a.distance_meters().unwrap_or_else(|| {
                warn!(
                    activity_id = a.id(),
                    "Activity missing distance_meters in pace calculation, using 1.0m fallback"
                );
                1.0
            });
            a.duration_seconds() as f64 / distance
        })
        .sum::<f64>()
        / first_half.len() as f64;

    #[allow(clippy::cast_precision_loss)]
    let second_avg_pace: f64 = second_half
        .iter()
        .map(|a| {
            let distance = a.distance_meters().unwrap_or_else(|| {
                warn!(
                    activity_id = a.id(),
                    "Activity missing distance_meters in pace calculation, using 1.0m fallback"
                );
                1.0
            });
            a.duration_seconds() as f64 / distance
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
fn calculate_trend(activities: &[Activity]) -> &'static str {
    if activities.len() < 4 {
        return "stable";
    }

    let mid_point = activities.len() / 2;
    let older_half = &activities[..mid_point];
    let recent_half = &activities[mid_point..];

    #[allow(clippy::cast_precision_loss)]
    let older_avg_duration = older_half
        .iter()
        .map(Activity::duration_seconds)
        .sum::<u64>() as f64
        / older_half.len() as f64;

    #[allow(clippy::cast_precision_loss)]
    let recent_avg_duration = recent_half
        .iter()
        .map(Activity::duration_seconds)
        .sum::<u64>() as f64
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
fn predict_race_performance(activities: &[Activity], target_sport: &str) -> serde_json::Value {
    use PerformancePredictor;

    // Filter activities by sport type
    let running_activities: Vec<&Activity> = activities
        .iter()
        .filter(|a| format!("{:?}", a.sport_type()).contains("Run"))
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

    let best_distance = best_activity.distance_meters().unwrap_or_else(|| {
        warn!(
            activity_id = best_activity.id(),
            "Best activity missing distance_meters despite find_best_performance validation, using 0.0m"
        );
        0.0
    });
    #[allow(clippy::cast_precision_loss)]
    let best_time = best_activity.duration_seconds() as f64;

    // Generate race predictions using PerformancePredictor (includes VDOT calculation)
    match PerformancePredictor::generate_race_predictions(best_distance, best_time) {
        Ok(race_predictions) => {
            // Calculate confidence based on data quality
            let confidence =
                calculate_prediction_confidence(&running_activities, &best_activity.start_date());

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
                    "date": best_activity.start_date().to_rfc3339(),
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
                "error": format!("Failed to generate predictions: {e}"),
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
    activities: &[&Activity],
    best_activity_date: &chrono::DateTime<chrono::Utc>,
) -> String {
    use chrono::Utc;
    use TrainingLoadCalculator;

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
fn analyze_detailed_training_load(activities: &[Activity], timeframe: &str) -> serde_json::Value {
    use TrainingLoadCalculator;

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
fn calculate_weekly_tss_from_history(tss_history: &[TssDataPoint]) -> Vec<serde_json::Value> {
    use HashMap;

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
